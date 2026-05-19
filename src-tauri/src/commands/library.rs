use crate::db::{self, queries};
use crate::utils::error::{CommandError, CommandResult};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct FilterParams {
    pub keyword: Option<String>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub group_id: Option<i64>,
    pub is_hidden: Option<bool>,
    pub is_finished: Option<bool>,
    pub favorites_only: Option<bool>,
}

#[derive(Serialize)]
pub struct MovieItem {
    pub file_id: String,
    pub title: String,
    pub original_title: Option<String>,
    pub chinese_name: Option<String>,
    pub year: Option<i32>,
    pub poster_local: Option<String>,
    pub rating: Option<f64>,
    pub genre: Vec<String>,
    pub is_hidden: bool,
    pub runtime: Option<i32>,
    pub progress: Option<i64>,
    pub duration: Option<i64>,
    pub scrape_status: i32,
    pub scrape_started_at: Option<i64>,
    pub scrape_finished_at: Option<i64>,
    pub scrape_error: Option<String>,
}

#[derive(Serialize)]
pub struct MoviesResponse {
    pub movies: Vec<MovieItem>,
    pub total: i64,
}

#[tauri::command]
pub fn get_movies(
    filters: FilterParams,
    sort: String,
    page: i64,
    page_size: Option<i64>,
) -> Result<MoviesResponse, CommandError> {
    let ps = page_size.unwrap_or(20);
    db::with_db(|conn| {
        let (rows, total) = queries::get_movies_paginated(
            conn,
            filters.keyword.as_deref(),
            filters.year,
            filters.genre.as_deref(),
            filters.group_id,
            filters.is_hidden,
            filters.favorites_only,
            &sort,
            page,
            ps,
        )?;

        let movies = rows.into_iter().map(|r| MovieItem {
            file_id: r.file_id, title: r.title,
            original_title: r.original_title, chinese_name: r.chinese_name,
            year: r.year, poster_local: r.poster_local, rating: r.rating,
            genre: parse_genre(&r.genre), runtime: r.runtime, is_hidden: r.is_hidden,
            progress: Some(r.progress), duration: Some(r.duration),
            scrape_status: r.scrape_status,
            scrape_started_at: r.scrape_started_at,
            scrape_finished_at: r.scrape_finished_at,
            scrape_error: r.scrape_error,
        }).collect();

        Ok(MoviesResponse { movies, total })
    })
}

fn parse_genre(genre_json: &Option<String>) -> Vec<String> {
    genre_json.as_ref()
        .and_then(|g| serde_json::from_str(g).ok())
        .unwrap_or_default()
}

#[derive(Serialize)]
pub struct MovieDetailResponse {
    pub file_id: String,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    pub poster_local: Option<String>,
    pub backdrop_local: Option<String>,
    pub overview: Option<String>,
    pub rating: Option<f64>,
    pub runtime: Option<i32>,
    pub director: Option<String>,
    pub genre: Vec<String>,
    pub file_name: String,
    pub file_size: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_hidden: bool,
    pub actors: Vec<String>,
    pub groups: Vec<GroupResponse>,
}

#[derive(Serialize)]
pub struct GroupResponse {
    pub id: i64,
    pub name: String,
    pub group_type: String,
}

#[tauri::command]
pub fn get_movie_detail(file_id: String) -> Result<Option<MovieDetailResponse>, CommandError> {
    db::with_db(|conn| {
        let detail = match queries::get_movie_detail(conn, &file_id)? {
            Some(d) => d,
            None => return Ok(None),
        };

        let actors = queries::get_movie_actors(conn, &file_id)?;
        let groups = queries::get_movie_groups(conn, &file_id)?.into_iter()
            .map(|g| GroupResponse { id: g.id, name: g.name, group_type: g.group_type })
            .collect();

        Ok(Some(MovieDetailResponse {
            file_id: detail.file_id,
            title: detail.title,
            original_title: detail.original_title,
            year: detail.year,
            poster_local: detail.poster_local,
            backdrop_local: detail.backdrop_url,
            overview: detail.overview,
            rating: detail.rating,
            runtime: detail.runtime,
            director: detail.director,
            genre: parse_genre(&detail.genre),
            file_name: detail.file_name,
            file_size: detail.file_size,
            created_at: detail.created_at,
            updated_at: detail.updated_at,
            is_hidden: detail.is_hidden,
            actors,
            groups,
        }))
    })
}

#[tauri::command]
pub fn batch_action(
    file_ids: Vec<String>,
    action: String,
) -> Result<(), CommandError> {
    db::with_db(|conn| {
        match action.as_str() {
            "mark_watched" => {
                let now = db::now_ts();
                for fid in &file_ids {
                    conn.execute(
                        "INSERT OR REPLACE INTO play_progress (file_id, progress, duration, is_finished, updated_at)
                         SELECT ?1, duration, duration, 1, ?2 FROM play_progress WHERE file_id = ?1",
                        rusqlite::params![fid, now],
                    )?;
                }
            }
            "mark_unwatched" => {
                for fid in &file_ids {
                    conn.execute("DELETE FROM play_progress WHERE file_id = ?1", [fid])?;
                }
            }
            "rescrape" => {
                // Trigger rescrape - mark movies for re-scraping
                log::info!("批量重新刮削: {} 个文件", file_ids.len());
            }
            _ => return Err(CommandError::invalid_input("未知操作")),
        }
        Ok(())
    })
}

#[tauri::command]
pub fn batch_set_scrape_status(file_ids: Vec<String>, status: i32) -> Result<(), CommandError> {
    db::with_db(|conn| {
        for fid in &file_ids {
            conn.execute("UPDATE movies SET scrape_status = ?1 WHERE file_id = ?2", rusqlite::params![status, fid])?;
        }
        log::info!("批量更新刮削状态: {} 个文件 -> {}", file_ids.len(), status);
        Ok(())
    })
}

#[tauri::command]
pub fn hide_movies(file_ids: Vec<String>) -> Result<(), CommandError> {
    db::with_db(|conn| {
        for fid in &file_ids {
            conn.execute("UPDATE movies SET is_hidden = 1 WHERE file_id = ?1", [fid])?;
        }
        Ok(())
    })
}

/// Find movies associated with an actress by name
#[tauri::command]
pub fn get_actress_movies(actress_name: String) -> Result<Vec<MovieItem>, CommandError> {
    db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT m.file_id, m.title, m.original_title, m.chinese_name, m.year, m.poster_local, m.rating, m.genre, m.runtime, m.is_hidden,
                    COALESCE(p.progress,0), COALESCE(p.duration,0), COALESCE(m.scrape_status,0)
             FROM movies m LEFT JOIN play_progress p ON m.file_id=p.file_id
             WHERE m.file_id IN (SELECT movie_id FROM movie_actors ma JOIN actors a ON ma.actor_id=a.id WHERE a.name=?1)
             ORDER BY m.year DESC LIMIT 50",
        )?;
        let movies: Vec<MovieItem> = stmt.query_map([&actress_name], |row| {
            Ok(MovieItem {
                file_id: row.get(0)?, title: row.get(1)?, original_title: row.get(2)?,
                chinese_name: row.get(3)?, year: row.get(4)?,
                poster_local: row.get(5)?, rating: row.get(6)?,
                genre: parse_genre(&row.get::<_,Option<String>>(7)?),
                runtime: row.get(8)?,
                is_hidden: row.get::<_,i32>(9)?!=0,
                progress: Some(row.get(10)?), duration: Some(row.get(11)?),
                scrape_status: row.get(12)?,
                scrape_started_at: None, scrape_finished_at: None, scrape_error: None,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(movies)
    })
}

#[tauri::command]
pub fn set_movie_chinese_name(file_id: String, chinese_name: String) -> Result<(), CommandError> {
    db::with_db(|conn| {
        conn.execute("UPDATE movies SET chinese_name = ?1 WHERE file_id = ?2", rusqlite::params![chinese_name, file_id])?;
        Ok(())
    })
}

#[tauri::command]
pub fn unhide_movies(file_ids: Vec<String>) -> Result<(), CommandError> {
    db::with_db(|conn| {
        for fid in &file_ids {
            conn.execute("UPDATE movies SET is_hidden = 0 WHERE file_id = ?1", [fid])?;
        }
        Ok(())
    })
}

// ─── Genre Translation Library ───

#[derive(serde::Serialize)]
pub struct GenreTranslation {
    pub ja_name: String,
    pub cn_name: String,
    pub blacklisted: bool,
}

#[tauri::command]
pub fn get_genre_translations() -> Result<Vec<GenreTranslation>, CommandError> {
    db::with_db(|conn| {
        let rows = queries::get_all_genre_translations(conn)?;
        log::info!("标签库查询: {} 条记录", rows.len());
        Ok(rows.into_iter().map(|(ja_name, cn_name, blacklisted)| GenreTranslation { ja_name, cn_name, blacklisted }).collect())
    })
}

#[tauri::command]
pub fn set_genre_blacklist(ja_name: String, blacklisted: bool) -> Result<(), CommandError> {
    db::with_db(|conn| queries::set_genre_blacklist(conn, &ja_name, blacklisted))
}

#[tauri::command]
pub fn set_genre_translation(ja_name: String, cn_name: String) -> Result<(), CommandError> {
    db::with_db(|conn| queries::set_genre_translation(conn, &ja_name, &cn_name))
}

#[tauri::command]
pub fn delete_genre_translation(ja_name: String) -> Result<(), CommandError> {
    log::info!("删除标签翻译: {}", ja_name);
    db::with_db(|conn| queries::delete_genre_translation(conn, &ja_name))
}
