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
    pub year: Option<i32>,
    pub poster_local: Option<String>,
    pub rating: Option<f64>,
    pub genre: Vec<String>,
    pub is_hidden: bool,
    pub progress: Option<i64>,
    pub duration: Option<i64>,
    pub group_names: Vec<String>,     // 所属分组名称列表
    pub is_favorite: bool,            // 是否收藏（在favorite类型分组中）
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

        let movies = rows.into_iter().map(|r| {
            let group_info = get_movie_group_info(conn, &r.file_id);
            MovieItem {
                file_id: r.file_id,
                title: r.title,
                year: r.year,
                poster_local: r.poster_local,
                rating: r.rating,
                genre: parse_genre(&r.genre),
                is_hidden: r.is_hidden,
                progress: Some(r.progress),
                duration: Some(r.duration),
                group_names: group_info.0,
                is_favorite: group_info.1,
            }
        }).collect();

        Ok(MoviesResponse { movies, total })
    })
}

fn get_movie_group_info(conn: &rusqlite::Connection, file_id: &str) -> (Vec<String>, bool) {
    let mut stmt = conn.prepare(
        "SELECT g.name, g.type FROM groups g JOIN movie_groups mg ON g.id = mg.group_id WHERE mg.movie_id = ?1"
    ).ok();
    let mut names = Vec::new();
    let mut is_fav = false;
    if let Some(stmt) = stmt.as_mut() {
        if let Ok(rows) = stmt.query_map([file_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))) {
            for r in rows.flatten() {
                names.push(r.0);
                if r.1 == "favorite" { is_fav = true; }
            }
        }
    }
    (names, is_fav)
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
pub fn hide_movies(file_ids: Vec<String>) -> Result<(), CommandError> {
    db::with_db(|conn| {
        for fid in &file_ids {
            conn.execute("UPDATE movies SET is_hidden = 1 WHERE file_id = ?1", [fid])?;
        }
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
