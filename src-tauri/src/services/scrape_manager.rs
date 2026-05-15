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

/// Scrape a single file using enabled sources.
/// Returns a list of results sorted by score descending.
pub async fn scrape_file(file_id: &str, sources: &[String]) -> Result<Vec<ScrapeResult>, CommandError> {
    // Get the file info from DB
    let file_name = db::with_db(|conn| {
        conn.query_row(
            "SELECT file_name FROM movies WHERE file_id = ?1",
            [file_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|e| CommandError::not_found(&format!("文件不存在: {}", e)))
    })?;

    let parsed = filename_parser::parse_filename(&file_name);
    let search_query = if let Some(ref id) = parsed.id_number {
        id.clone()
    } else {
        parsed.cleaned.clone()
    };

    let mut results: Vec<ScrapeResult> = Vec::new();

    for source in sources {
        match source.as_str() {
            "tmdb" => {
                if let Ok(r) = scrape_tmdb(&search_query).await {
                    results.push(r);
                }
            }
            "douban" => {
                if let Ok(r) = scrape_douban(&search_query).await {
                    results.push(r);
                }
            }
            "javbus" => {
                if let Ok(r) = scrape_javbus(&search_query).await {
                    results.push(r);
                }
            }
            _ => {
                // Other sources are placeholders
                log::debug!("刮削源 {} 尚未实现", source);
            }
        }
    }

    // Sort by score descending
    results.sort_by(|a, b| b.score.cmp(&a.score));

    Ok(results)
}

/// Apply a scrape result to a movie record
pub async fn apply_scrape_result(file_id: &str, result: &ScrapeResult) -> CommandResult<()> {
    let now = db::now_ts();
    db::with_db(|conn| {
        conn.execute(
            "UPDATE movies SET title = ?1, year = ?2, poster_url = ?3, overview = ?4,
             rating = ?5, runtime = ?6, director = ?7, genre = ?8, updated_at = ?9
             WHERE file_id = ?10",
            rusqlite::params![
                result.title, result.year, result.poster_url, result.overview,
                result.rating, result.runtime, result.director,
                result.genre.as_ref().map(|g| serde_json::to_string(g).unwrap_or_default()),
                now, file_id
            ],
        )?;

        // Insert actors
        if let Some(actors) = &result.actors {
            for name in actors {
                conn.execute("INSERT OR IGNORE INTO actors (name) VALUES (?1)", [name])?;
                let actor_id: i64 = conn.query_row(
                    "SELECT id FROM actors WHERE name = ?1", [name], |row| row.get(0)
                )?;
                conn.execute(
                    "INSERT OR IGNORE INTO movie_actors (movie_id, actor_id) VALUES (?1, ?2)",
                    rusqlite::params![file_id, actor_id],
                )?;
            }
        }

        Ok(())
    })
}

// ─── Source-specific scrapers ───

async fn scrape_tmdb(query: &str) -> Result<ScrapeResult, CommandError> {
    // Placeholder - would use TMDB API with an API key
    Ok(ScrapeResult {
        source: "tmdb".to_string(),
        title: query.to_string(),
        year: None,
        poster_url: None,
        backdrop_url: None,
        overview: None,
        rating: None,
        runtime: None,
        director: None,
        genre: None,
        actors: None,
        score: 30,
    })
}

async fn scrape_douban(query: &str) -> Result<ScrapeResult, CommandError> {
    // Placeholder - would scrape Douban
    Ok(ScrapeResult {
        source: "douban".to_string(),
        title: query.to_string(),
        year: None,
        poster_url: None,
        backdrop_url: None,
        overview: None,
        rating: None,
        runtime: None,
        director: None,
        genre: None,
        actors: None,
        score: 20,
    })
}

async fn scrape_javbus(query: &str) -> Result<ScrapeResult, CommandError> {
    // Placeholder - would scrape JavBus
    Ok(ScrapeResult {
        source: "javbus".to_string(),
        title: query.to_string(),
        year: None,
        poster_url: None,
        backdrop_url: None,
        overview: None,
        rating: None,
        runtime: None,
        director: None,
        genre: None,
        actors: None,
        score: 25,
    })
}
