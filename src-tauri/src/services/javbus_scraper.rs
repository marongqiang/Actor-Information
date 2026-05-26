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

    // Step 1: Search
    let search_url = format!("https://www.javbus.com/ja/search/{}", &code);
    log::info!("JavBus 搜索: {}", search_url);
    let resp = client.get(&search_url)
        .header("Referer", "https://www.javbus.com/")
        .header("Cookie", "existmag=all")
        .send()
        .map_err(|e| CommandError::network(&e.to_string()))?;
    let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = Html::parse_document(&body);

    // Find first movie-box link
    let sel_box = Selector::parse("a.movie-box").unwrap();
    let sel_img = Selector::parse("img").unwrap();
    let sel_date = Selector::parse("date").unwrap();

    let mut detail_url = None;
    let mut title = query.to_string();
    let mut thumb_url = None;
    let mut year = None;

    for item in doc.select(&sel_box) {
        if let Some(img) = item.select(&sel_img).next() {
            title = img.value().attr("title").unwrap_or(query).to_string();
            thumb_url = img.value().attr("src").map(|s| {
                if s.starts_with("http") { s.to_string() }
                else { format!("https://www.javbus.com{}", s) }
            });
        }
        detail_url = item.value().attr("href").map(|h| {
            if h.starts_with("http") { h.to_string() }
            else { format!("https://www.javbus.com{}", h) }
        });
        let dates: Vec<_> = item.select(&sel_date).collect();
        if dates.len() >= 2 {
            if let Ok(y) = dates[1].text().collect::<String>().trim().parse::<i32>() {
                year = Some(y);
            }
        }
        break; // Take first result
    }

    let d_url = match detail_url {
        Some(u) => u,
        None => return Err(CommandError::scrape_failed("JavBus 无结果")),
    };

    // Step 2: Detail page
    log::info!("JavBus 详情: {}", d_url);
    let resp2 = client.get(&d_url)
        .header("Referer", "https://www.javbus.com/")
        .header("Cookie", "existmag=all")
        .send()
        .map_err(|e| CommandError::network(&e.to_string()))?;
    let body2 = resp2.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc2 = Html::parse_document(&body2);

    // Title + Cover
    let sel_big_img = Selector::parse("a.bigImage img").unwrap();
    let mut cover_url = None;
    if let Some(img) = doc2.select(&sel_big_img).next() {
        let t = img.value().attr("title").unwrap_or(&title);
        if !t.is_empty() { title = t.to_string(); }
        cover_url = img.value().attr("src").map(|s| {
            if s.starts_with("http") { s.to_string() }
            else { format!("https://www.javbus.com{}", s) }
        });
    }

    // Info fields (runtime, date, number, director, etc.)
    let sel_info = Selector::parse("div.col-md-3.info p").unwrap();
    let mut runtime = None;
    let mut director = None;
    for p in doc2.select(&sel_info) {
        let text = p.text().collect::<String>();
        let text = text.trim();
        if text.contains("収録時間") || text.contains("时长") {
            // Extract minutes from text like "120 分"
            runtime = text.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().ok();
        }
        if text.contains("監督") || text.contains("导演") {
            if let Some(a) = p.select(&Selector::parse("a").unwrap()).next() {
                director = Some(a.text().collect::<String>().trim().to_string());
            }
        }
        if text.contains("発売日") && year.is_none() {
            let nums: String = text.chars().filter(|c| c.is_ascii_digit() || *c == '-').collect();
            if nums.len() >= 4 { year = nums[..4].parse().ok(); }
        }
    }

    // Genres
    let sel_genre = Selector::parse("span.genre label a, span.genre a").unwrap();
    let mut genres = Vec::new();
    for g in doc2.select(&sel_genre) {
        let t = g.text().collect::<String>().trim().to_string();
        if !t.is_empty() { genres.push(t); }
    }

    // Actors
    let sel_actor = Selector::parse("#star-div a, .star-div a").unwrap();
    let mut actors = Vec::new();
    for a in doc2.select(&sel_actor) {
        let t = a.text().collect::<String>().trim().to_string();
        if !t.is_empty() { actors.push(t); }
    }

    // Cover: try to convert thumb to full cover
    let poster = cover_url.or(thumb_url).map(|s| {
        s.replace("/thumbs/", "/cover/").replace("/thumb/", "/cover/")
    });

    log::info!("JavBus 结果: title={} actors={} genres={} runtime={:?}", title, actors.len(), genres.len(), runtime);

    Ok(JavBusResult {
        title, year, poster_url: poster, overview: None, rating: None, runtime, director,
        genres, actors,
    })
}
