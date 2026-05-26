// JavBus scraper — ported from MetaTube Go provider
use crate::db;
use crate::utils::error::{CommandError, CommandResult};
use scraper::{Html, Selector};

#[derive(Debug)]
pub struct JavBusResult {
    pub title: String,
    pub year: Option<i32>,
    pub poster_url: Option<String>,
    pub overview: Option<String>,
    pub rating: Option<f64>,
    pub runtime: Option<i32>,
    pub director: Option<String>,
    pub genres: Vec<String>,
    pub actors: Vec<String>,
}

fn build_client() -> reqwest::blocking::Client {
    let ua = format!("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{}.0.0.0 Safari/537.36", 120 + rand::random::<u8>() as u16 % 30);
    let mut builder = reqwest::blocking::Client::builder()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(ua);

    if let Ok(Some(enabled)) = db::with_db(|c| crate::db::queries::get_config(c, "proxy_enabled")) {
        if enabled == "true" {
            if let Ok(Some(host)) = db::with_db(|c| crate::db::queries::get_config(c, "proxy_host")) {
                let port = db::with_db(|c| crate::db::queries::get_config(c, "proxy_port"))
                    .ok().flatten().and_then(|p| p.parse().ok()).unwrap_or(1080);
                if let Ok(proxy) = reqwest::Proxy::all(&format!("http://{}:{}", host, port)) {
                    builder = builder.proxy(proxy);
                }
            }
        }
    }
    builder.build().expect("Failed to build JavBus client")
}

pub fn search_javbus(query: &str) -> Result<JavBusResult, CommandError> {
    let client = build_client();
    let code = query.trim().to_uppercase().replace(['-', '_', ' '], "");

    // Step 1: Visit homepage with existmag cookie to seed the cookie_store
    // This is the key cookie MetaTube uses. The response may set additional cookies.
    let _ = client.get("https://www.javbus.com/")
        .header("Referer", "https://www.javbus.com/")
        .header("Cookie", "existmag=all")
        .send();

    // Step 2: Access detail page — cookie_store now has existmag + any Set-Cookie from step 1
    // Do NOT set Cookie header here; let cookie_store handle it (like MetaTube's colly)
    let detail_url = format!("https://www.javbus.com/ja/{}", &code);
    log::info!("JavBus 详情: {}", detail_url);
    let resp = client.get(&detail_url)
        .header("Referer", "https://www.javbus.com/")
        .header("Accept-Language", "ja-JP,ja;q=0.9")
        .send()
        .map_err(|e| CommandError::network(&e.to_string()))?;
    // Manual redirect handling (like MetaTube's WithDisableRedirects)
    let status = resp.status();
    let redirect_url = resp.headers().get("location").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    let mut body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    if status.is_redirection() {
        if let Some(loc) = redirect_url {
            let loc = if loc.starts_with("http") { loc }
                else if loc.starts_with('/') { format!("https://www.javbus.com{}", loc) }
                else { format!("https://www.javbus.com/{}", loc) };
            log::info!("JavBus 重定向: {}", loc);
            let r2 = client.get(&loc)
                .header("Referer", &detail_url)
                .header("Accept-Language", "ja-JP,ja;q=0.9")
                .send()
                .map_err(|e| CommandError::network(&e.to_string()))?;
            body = r2.text().map_err(|e| CommandError::network(&e.to_string()))?;
        }
    }
    log::info!("JavBus 响应: {} bytes, Cloudflare={} AgeVerify={}", body.len(), body.contains("Cloudflare"), body.contains("Age Verification"));

    // Handle age verification page
    let body = if body.contains("Age Verification") {
        log::info!("JavBus 年龄验证页面, 查找跳过方式...");
        let doc = Html::parse_document(&body);
        let mut verify_url = None;
        // Method 1: Find <a> link with "over18", "enter", "age"
        if let Ok(sel) = Selector::parse("a") {
            for a in doc.select(&sel) {
                if let Some(href) = a.value().attr("href") {
                    let text = a.text().collect::<String>().to_lowercase();
                    if text.contains("enter") || text.contains("18") || text.contains("over")
                        || href.contains("over18") || href.contains("age") || text.contains("はい")
                        || text.contains("yes") || text.contains("agree") || text.contains("同意") {
                        verify_url = Some(if href.starts_with("http") { href.to_string() }
                            else if href.starts_with('/') { format!("https://www.javbus.com{}", href) }
                            else { format!("https://www.javbus.com/{}", href) });
                        log::info!("JavBus 验证链接: {} -> {}", text.trim(), verify_url.as_ref().unwrap());
                        break;
                    }
                }
            }
        }
        // Method 2: Detect Cloudflare Turnstile or JS challenge — try a direct bypass
        if verify_url.is_none() && (body.contains("challenge") || body.contains("cf-")) {
            log::info!("JavBus Cloudflare挑战, 尝试 /cdn-cgi/ bypass");
            verify_url = Some(detail_url.clone());
        }
        // Follow verification URL, or try direct access with delay
        if let Some(vu) = verify_url {
            let resp = client.get(&vu)
                .header("Referer", &detail_url)
                .header("Cookie", "existmag=all; over18=18")
                .send()
                .map_err(|e| CommandError::network(&e.to_string()))?;
            resp.text().map_err(|e| CommandError::network(&e.to_string()))?
        } else {
            log::warn!("JavBus 未找到验证方式");
            body // return original body, will be caught by bogus filter
        }
    } else {
        body
    };

    if body.len() < 500 || body.contains("Cloudflare") || body.contains("404") {
        return Err(CommandError::scrape_failed("JavBus 无结果"));
    }

    let doc = Html::parse_document(&body);

    // Title + Cover — try multiple selectors
    let mut title = query.to_string();
    let mut cover_url = None;
    let title_selectors = ["a.bigImage img", ".bigImage img", "img.bigImage", "h3", "title"];
    for sel_str in &title_selectors {
        if let Ok(sel) = Selector::parse(sel_str) {
            if let Some(el) = doc.select(&sel).next() {
                let mut t = el.value().attr("title").unwrap_or("").to_string();
                if t.is_empty() { t = el.text().collect::<String>().trim().to_string(); }
                if !t.is_empty() && t.len() > 2 { title = t; break; }
            }
        }
    }
    // Cover
    let cover_selectors = ["a.bigImage img", ".bigImage img"];
    for sel_str in &cover_selectors {
        if let Ok(sel) = Selector::parse(sel_str) {
            if let Some(img) = doc.select(&sel).next() {
                cover_url = img.value().attr("src").map(|s| {
                    if s.starts_with("http") { s.to_string() } else { format!("https://www.javbus.com{}", s) }
                });
                if cover_url.is_some() { break; }
            }
        }
    }

    // Info fields
    let mut runtime = None;
    let mut year = None;
    let info_selectors = ["div.col-md-3.info p", ".info p", ".col-md-3 p", "p", ".header p"];
    for sel_str in &info_selectors {
        if let Ok(sel) = Selector::parse(sel_str) {
            for p in doc.select(&sel) {
                let text = p.text().collect::<String>().trim().to_string();
                if text.contains("収録時間") || text.contains("时长") || text.contains("時間") {
                    runtime = runtime.or_else(|| text.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().ok());
                }
                if text.contains("発売日") || text.contains("日期") {
                    let nums: String = text.chars().filter(|c| c.is_ascii_digit() || *c == '-').collect();
                    if nums.len() >= 4 { year = nums[..4].parse().ok(); }
                }
            }
        }
    }

    // Genres
    let mut genres = Vec::new();
    let genre_selectors = ["span.genre label a", "span.genre a", ".genre a", "a[href*='genre']"];
    for sel_str in &genre_selectors {
        if let Ok(sel) = Selector::parse(sel_str) {
            for g in doc.select(&sel) {
                let t = g.text().collect::<String>().trim().to_string();
                if !t.is_empty() { genres.push(t); }
            }
        }
    }

    // Actors
    let mut actors = Vec::new();
    let actor_selectors = ["#star-div a", ".star-div a", "a[href*='star']", "a[href*='actress']"];
    for sel_str in &actor_selectors {
        if let Ok(sel) = Selector::parse(sel_str) {
            for a in doc.select(&sel) {
                let t = a.text().collect::<String>().trim().to_string();
                if !t.is_empty() { actors.push(t); }
            }
        }
    }

    // Poster: convert thumb to full cover
    let poster = cover_url.map(|s| {
        s.replace("/thumbs/", "/cover/").replace("/thumb/", "/cover/")
    });

    log::info!("JavBus 结果: title={} actors={} genres={} runtime={:?}", title, actors.len(), genres.len(), runtime);

    Ok(JavBusResult {
        title, year, poster_url: poster, overview: None, rating: None, runtime, director: None,
        genres, actors,
    })
}
