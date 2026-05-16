use crate::db;
use crate::utils::error::{CommandError, CommandResult};
use crate::utils::filename_parser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScrapeResult {
    pub source: String,
    pub title: String,
    pub year: Option<i32>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub overview: Option<String>,
    pub rating: Option<f64>,
    pub runtime: Option<i32>,
    pub director: Option<String>,
    pub genre: Option<Vec<String>>,
    pub actors: Option<Vec<String>>,
    pub score: i32,
}

static SCRAPE_CLIENT: once_cell::sync::Lazy<reqwest::blocking::Client> = once_cell::sync::Lazy::new(|| {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .expect("Failed to build scrape client")
});

/// Synchronous scrape - call from spawn_blocking context
pub fn scrape_file(file_id: &str, sources: &[String]) -> Result<Vec<ScrapeResult>, CommandError> {
    let file_name = db::with_db(|conn| {
        conn.query_row("SELECT file_name FROM movies WHERE file_id=?1", [file_id], |r| r.get::<_, String>(0))
            .map_err(|_| CommandError::not_found("文件不存在"))
    })?;

    let parsed = filename_parser::parse_filename(&file_name);
    let query = parsed.id_number.as_deref().unwrap_or(&parsed.cleaned);

    let mut results = Vec::new();
    for source in sources {
        let r = match source.as_str() {
            "tmdb" => scrape_tmdb(query),
            "javbus" => scrape_javbus(query),
            "javdb" => scrape_javdb(query),
            _ => continue,
        };
        if let Ok(r) = r { results.push(r); }
    }
    results.sort_by(|a, b| b.score.cmp(&a.score));
    Ok(results)
}

pub fn apply_scrape_result(file_id: &str, result: &ScrapeResult) -> CommandResult<()> {
    let now = db::now_ts();
    db::with_db(|conn| {
        conn.execute("UPDATE movies SET title=?1,year=?2,poster_url=?3,overview=?4,rating=?5,runtime=?6,director=?7,genre=?8,scrape_status=2,updated_at=?9 WHERE file_id=?10",
            rusqlite::params![result.title,result.year,result.poster_url,result.overview,result.rating,result.runtime,result.director,
                result.genre.as_ref().map(|g| serde_json::to_string(g).unwrap_or_default()), now, file_id])?;
        if let Some(actors) = &result.actors {
            for name in actors {
                conn.execute("INSERT OR IGNORE INTO actors (name) VALUES (?1)", [name])?;
                let actor_id: i64 = conn.query_row("SELECT id FROM actors WHERE name=?1", [name], |r| r.get(0))?;
                conn.execute("INSERT OR IGNORE INTO movie_actors (movie_id,actor_id) VALUES (?1,?2)", rusqlite::params![file_id, actor_id])?;
            }
        }
        Ok(())
    })
}

// ─── TMDB (blocking HTTP) ───

fn scrape_tmdb(query: &str) -> Result<ScrapeResult, CommandError> {
    let api_key = db::with_db(|conn| crate::db::queries::get_config(conn, "tmdb_api_key")).ok().flatten()
        .or_else(|| std::env::var("TMDB_API_KEY").ok())
        .ok_or_else(|| CommandError::scrape_failed("TMDB API Key未配置"))?;

    let url = format!("https://api.themoviedb.org/3/search/movie?api_key={}&query={}&language=zh-CN", api_key, encode(query));
    let resp = SCRAPE_CLIENT.get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let json: serde_json::Value = resp.json().map_err(|e| CommandError::network(&e.to_string()))?;
    let results = json["results"].as_array().ok_or_else(|| CommandError::scrape_failed("TMDB无结果"))?;
    if results.is_empty() { return Err(CommandError::scrape_failed("TMDB无匹配")); }

    let item = &results[0];
    let tmdb_id = item["id"].as_i64().unwrap_or(0);
    let title = item["title"].as_str().unwrap_or(query).to_string();
    let year = item["release_date"].as_str().and_then(|d| d[..4].parse().ok());
    let overview = item["overview"].as_str().map(|s| s.to_string());
    let rating = item["vote_average"].as_f64();
    let poster = item["poster_path"].as_str().map(|p| format!("https://image.tmdb.org/t/p/w500{}", p));
    let backdrop = item["backdrop_path"].as_str().map(|p| format!("https://image.tmdb.org/t/p/w1280{}", p));
    let genre_ids: Vec<i64> = item["genre_ids"].as_array().map(|a| a.iter().filter_map(|v| v.as_i64()).collect()).unwrap_or_default();

    let mut director = None;
    let mut actors = Vec::new();
    if tmdb_id > 0 {
        let cu = format!("https://api.themoviedb.org/3/movie/{}/credits?api_key={}&language=zh-CN", tmdb_id, api_key);
        if let Ok(resp) = SCRAPE_CLIENT.get(&cu).send() {
            if let Ok(cj) = resp.json::<serde_json::Value>() {
                if let Some(crew) = cj["crew"].as_array() {
                    for c in crew { if c["job"].as_str() == Some("Director") { director = c["name"].as_str().map(|s| s.to_string()); break; } }
                }
                if let Some(cast) = cj["cast"].as_array() {
                    for c in cast.iter().take(5) { if let Some(n) = c["name"].as_str() { actors.push(n.to_string()); } }
                }
            }
        }
    }

    let genre_names = if !genre_ids.is_empty() {
        let gu = format!("https://api.themoviedb.org/3/genre/movie/list?api_key={}&language=zh-CN", api_key);
        SCRAPE_CLIENT.get(&gu).send().ok().and_then(|r| r.json::<serde_json::Value>().ok()).and_then(|gj|
            gj["genres"].as_array().map(|a| a.iter().filter_map(|g|
                if genre_ids.contains(&g["id"].as_i64().unwrap_or(0)) { g["name"].as_str().map(|s| s.to_string()) } else { None }
            ).collect())
        )
    } else { None };

    Ok(ScrapeResult { source: "tmdb".into(), title, year, poster_url: poster, backdrop_url: backdrop, overview, rating, runtime: None, director, genre: genre_names, actors: Some(actors), score: 80 })
}

// ─── JavBus (blocking HTML parse) ───

fn scrape_javbus(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.to_uppercase().replace(['-', '_', ' '], "");
    let url = format!("https://www.javbus.com/{}", code);
    let resp = SCRAPE_CLIENT.get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    if !resp.status().is_success() { return Err(CommandError::scrape_failed("JavBus未找到")); }
    let html = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&html);

    let s_title = scraper::Selector::parse("h3").unwrap();
    let s_cover = scraper::Selector::parse(".bigImage img").unwrap();
    let s_info = scraper::Selector::parse(".info p").unwrap();
    let s_actor = scraper::Selector::parse("#star-div a").unwrap();
    let s_genre = scraper::Selector::parse(".genre a").unwrap();

    let title = doc.select(&s_title).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&s_cover).next().and_then(|e| e.value().attr("src").map(|s| s.to_string()));

    let mut director = None; let mut year = None; let mut runtime = None;
    for p in doc.select(&s_info) {
        let t = p.text().collect::<String>();
        if t.contains("導演") || t.contains("导演") { director = t.split(':').nth(1).or_else(|| t.split('：').nth(1)).map(|s| s.trim().to_string()); }
        if t.contains("發行日期") || t.contains("发行日期") { year = t.split_whitespace().last().and_then(|d| d[..4].parse().ok()); }
        if t.contains("長度") || t.contains("长度") { runtime = t.split_whitespace().last().and_then(|s| s.trim().parse().ok()); }
    }
    let actors: Vec<String> = doc.select(&s_actor).filter_map(|a| { let n = a.text().collect::<String>().trim().to_string(); if n.is_empty() { None } else { Some(n) } }).collect();
    let genres: Vec<String> = doc.select(&s_genre).filter_map(|g| { let n = g.text().collect::<String>().trim().to_string(); if n.is_empty() { None } else { Some(n) } }).collect();

    Ok(ScrapeResult { source: "javbus".into(), title, year, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime, director, genre: if genres.is_empty() { None } else { Some(genres) }, actors: if actors.is_empty() { None } else { Some(actors) }, score: 75 })
}

// ─── JavDB (blocking HTML parse) ───

fn scrape_javdb(query: &str) -> Result<ScrapeResult, CommandError> {
    let code = query.to_uppercase().replace(['-', '_', ' '], "");
    let url = format!("https://javdb.com/search?q={}&f=all", code);
    let resp = SCRAPE_CLIENT.get(&url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    if !resp.status().is_success() { return Err(CommandError::scrape_failed("JavDB搜索失败")); }
    let html = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&html);

    let s_item = scraper::Selector::parse(".movie-list .item a.box").unwrap();
    let detail_url = doc.select(&s_item).next().and_then(|e| e.value().attr("href").map(|s| format!("https://javdb.com{}", s)));
    let detail_url = match detail_url { Some(u) => u, None => return Err(CommandError::scrape_failed("JavDB无结果")) };

    let resp = SCRAPE_CLIENT.get(&detail_url).send().map_err(|e| CommandError::network(&e.to_string()))?;
    let html = resp.text().map_err(|e| CommandError::network(&e.to_string()))?;
    let doc = scraper::Html::parse_document(&html);

    let s_title = scraper::Selector::parse(".video-title strong").unwrap();
    let s_cover = scraper::Selector::parse(".video-cover img").unwrap();
    let s_panel = scraper::Selector::parse(".panel-block").unwrap();
    let s_actor = scraper::Selector::parse(".actress-name a").unwrap();

    let title = doc.select(&s_title).next().map(|e| e.text().collect::<String>().trim().to_string()).unwrap_or_else(|| query.to_string());
    let poster = doc.select(&s_cover).next().and_then(|e| e.value().attr("src").map(|s| s.to_string()));
    let mut year = None; let mut director = None; let mut runtime = None;
    for b in doc.select(&s_panel) {
        let t = b.text().collect::<String>();
        if t.contains("日期:") { year = t.split("日期:").nth(1).and_then(|d| d.trim()[..4].parse().ok()); }
        if t.contains("導演:") || t.contains("导演:") { director = t.split("導演:").nth(1).or_else(|| t.split("导演:").nth(1)).map(|s| s.trim().to_string()); }
        if t.contains("時長:") || t.contains("时长:") { runtime = t.split("時長:").nth(1).or_else(|| t.split("时长:").nth(1)).and_then(|s| s.trim().parse().ok()); }
    }
    let actors: Vec<String> = doc.select(&s_actor).filter_map(|a| { let n = a.text().collect::<String>().trim().to_string(); if n.is_empty() { None } else { Some(n) } }).collect();

    Ok(ScrapeResult { source: "javdb".into(), title, year, poster_url: poster, backdrop_url: None, overview: None, rating: None, runtime, director, genre: None, actors: if actors.is_empty() { None } else { Some(actors) }, score: 70 })
}

fn encode(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || "-_.~".contains(c) { c.to_string() } else { format!("%{:02X}", c as u8) }).collect()
}
