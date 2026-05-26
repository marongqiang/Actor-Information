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
    let mut builder = reqwest::blocking::Client::builder()
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/148.0.0.0 Safari/537.36 Edg/148.0.0.0");

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

    // Warm up cookies + age verification
    let _ = client.get("https://www.javbus.com/")
        .header("Cookie", "existmag=all; over18=18")
        .send();

    // Direct detail URL
    let detail_url = format!("https://www.javbus.com/ja/{}", &code);
    log::info!("JavBus 详情: {}", detail_url);
    let resp = client.get(&detail_url)
        .header("Referer", "https://www.javbus.com/")
        .header("Cookie", "existmag=all; over18=18")
        .send()
        .map_err(|e| CommandError::network(&e.to_string()))?;
    let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    log::info!("JavBus 响应: {} bytes, Cloudflare={} AgeVerify={}", body.len(), body.contains("Cloudflare"), body.contains("Age Verification"));

    if body.len() < 500 || body.contains("Cloudflare") || body.contains("404") {
        return Err(CommandError::scrape_failed("JavBus 无结果"));
    }
    if body.contains("Age Verification") {
        return Err(CommandError::scrape_failed("JavBus 年龄验证失败"));
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
