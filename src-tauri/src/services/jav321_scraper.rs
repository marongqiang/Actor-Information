// JAV321 scraper — ported from MetaTube Go provider
use crate::db;
use crate::utils::error::{CommandError, CommandResult};
use scraper::{Html, Selector};

#[derive(Debug)]
pub struct Jav321Result {
    pub title: String,
    pub year: Option<i32>,
    pub poster_url: Option<String>,
    pub overview: Option<String>,
    pub rating: Option<f64>,
    pub runtime: Option<i32>,
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
    builder.build().expect("Failed to build JAV321 client")
}

pub fn search_jav321(query: &str) -> Result<Jav321Result, CommandError> {
    let client = build_client();
    let code = query.trim().to_lowercase().replace(['-', '_', ' '], "");

    // JAV321 detail page uses lowercase ID
    let url = format!("https://www.jav321.com/video/{}", code);
    log::info!("JAV321 详情: {}", url);
    let resp = client.get(&url)
        .header("Referer", "https://www.jav321.com/")
        .send()
        .map_err(|e| CommandError::network(&e.to_string()))?;
    let body = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;

    if body.contains("not found") || body.len() < 500 {
        return Err(CommandError::scrape_failed("JAV321 无结果"));
    }

    let doc = Html::parse_document(&body);

    let title = doc.select(&Selector::parse("h3").unwrap()).next()
        .or_else(|| doc.select(&Selector::parse(".movie-title, .video-title, h2, h1").unwrap()).next())
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_else(|| query.to_string());

    // Cover image
    let poster = doc.select(&Selector::parse(".movie-img img, .video-img img, .container img").unwrap()).next()
        .or_else(|| doc.select(&Selector::parse("img[src*='pics']").unwrap()).next())
        .and_then(|img| img.value().attr("src").map(|s| {
            if s.starts_with("http") { s.to_string() } else { format!("https://www.jav321.com{}", s) }
        }));

    // Overview/summary — usually in a panel-body or description div
    let overview = doc.select(&Selector::parse(".panel-body, .description, .summary, .movie-desc, .synopsis").unwrap()).next()
        .map(|e| e.text().collect::<String>().trim().to_string());

    // Runtime, date, etc. from table rows or info section
    let mut runtime = None;
    let mut year = None;
    let mut rating = None;

    for row in doc.select(&Selector::parse("tr, .info-item, .detail-item, p").unwrap()) {
        let text = row.text().collect::<String>();
        let text = text.trim();
        if text.is_empty() { continue; }
        // Runtime: "収録時間" or "时长" or "Duration"
        if text.contains("収録時間") || text.contains("时长") || text.contains("Duration") || text.contains("時間") {
            let nums: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = nums.parse::<i64>() {
                runtime = Some(if n > 1000 { (n / 60) as i32 } else { n as i32 });
            }
        }
        // Release date
        if (text.contains("発売日") || text.contains("日期") || text.contains("Release")) && year.is_none() {
            let nums: String = text.chars().filter(|c| c.is_ascii_digit() || *c == '-').collect();
            if nums.len() >= 4 { year = nums[..4].parse().ok(); }
        }
        // Rating
        if text.contains("評分") || text.contains("评分") || text.contains("Score") {
            let nums: String = text.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
            rating = nums.parse().ok();
        }
    }

    // Actors
    let actors: Vec<String> = doc.select(&Selector::parse(".star-name a, .actress a, .cast a, a[href*='actress']").unwrap())
        .filter_map(|a| {
            let n = a.text().collect::<String>().trim().to_string();
            if n.is_empty() { None } else { Some(n) }
        }).collect();

    // Genres
    let genres: Vec<String> = doc.select(&Selector::parse(".tag a, .category a, .genre a, a[href*='genre']").unwrap())
        .filter_map(|a| {
            let n = a.text().collect::<String>().trim().to_string();
            if n.is_empty() { None } else { Some(n) }
        }).collect();

    log::info!("JAV321 结果: title={} actors={} genres={} overview={}B runtime={:?}", title, actors.len(), genres.len(), overview.as_ref().map_or(0, |o| o.len()), runtime);

    Ok(Jav321Result { title, year, poster_url: poster, overview, rating, runtime, genres, actors })
}
