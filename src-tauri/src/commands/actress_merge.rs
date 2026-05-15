use crate::db::{self, queries};
use crate::services::actress_sync;
use crate::utils::error::CommandResult;
use serde::Serialize;

#[derive(Serialize)]
pub struct ActressItem {
    pub id: i64,
    pub name: String,
    pub avatar_local: Option<String>,
    pub debut_year: Option<i32>,
    pub height: Option<i32>,
    pub bust: Option<i32>,
    pub waist: Option<i32>,
    pub hip: Option<i32>,
    pub cup: Option<String>,
    pub letter: Option<String>,
    pub movie_count: i64,
}

impl From<queries::ActressRow> for ActressItem {
    fn from(row: queries::ActressRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            avatar_local: row.avatar_local,
            debut_year: row.debut_year,
            height: row.height,
            bust: row.bust,
            waist: row.waist,
            hip: row.hip,
            cup: row.cup,
            letter: row.letter,
            movie_count: row.movie_count,
        }
    }
}

#[tauri::command]
pub async fn sync_actress_data() -> Result<(), crate::utils::error::CommandError> {
    actress_sync::sync_actress_data().await
}

#[derive(Serialize)]
pub struct ActressByLetter {
    pub letter: String,
    pub actresses: Vec<ActressItem>,
}

#[tauri::command]
pub fn get_actresses_by_letter() -> Result<Vec<ActressByLetter>, crate::utils::error::CommandError> {
    db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, avatar_local, debut_year, height, bust, waist, hip, cup, letter,
                    (SELECT COUNT(*) FROM movie_actors ma JOIN actors a ON ma.actor_id = a.id WHERE a.name = av_actors.name)
             FROM av_actors ORDER BY letter, name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(queries::ActressRow {
                id: row.get(0)?, name: row.get(1)?, avatar_local: row.get(2)?,
                debut_year: row.get(3)?, height: row.get(4)?, bust: row.get(5)?,
                waist: row.get(6)?, hip: row.get(7)?, cup: row.get(8)?,
                letter: row.get(9)?, movie_count: row.get(10)?,
            })
        })?.filter_map(|r| r.ok());

        let mut letters: Vec<ActressByLetter> = Vec::new();
        let mut current_letter = String::new();
        let mut current_actresses: Vec<ActressItem> = Vec::new();

        for row in rows {
            let letter = row.letter.clone().unwrap_or_else(|| "#".to_string());
            let first_char = letter.chars().next().unwrap_or('#').to_string();

            if first_char != current_letter {
                if !current_actresses.is_empty() {
                    letters.push(ActressByLetter {
                        letter: current_letter.clone(),
                        actresses: std::mem::take(&mut current_actresses),
                    });
                }
                current_letter = first_char;
            }
            current_actresses.push(ActressItem::from(row));
        }

        if !current_actresses.is_empty() {
            letters.push(ActressByLetter {
                letter: current_letter,
                actresses: current_actresses,
            });
        }

        Ok(letters)
    })
}

#[derive(Serialize)]
pub struct PaginatedActress {
    pub list: Vec<ActressItem>,
    pub total: i64,
}

#[tauri::command]
pub fn get_actresses_paginated(
    page: i64,
    page_size: i64,
    search: Option<String>,
    sort_field: Option<String>,
    sort_order: Option<String>,
) -> Result<PaginatedActress, crate::utils::error::CommandError> {
    db::with_db(|conn| {
        let (rows, total) = queries::get_actresses_paginated(conn, page, page_size, search.as_deref())?;
        let list: Vec<ActressItem> = rows.into_iter().map(ActressItem::from).collect();
        Ok(PaginatedActress { list, total })
    })
}

#[tauri::command]
pub fn find_actress(name: String) -> Result<Option<ActressItem>, crate::utils::error::CommandError> {
    actress_sync::find_actress(&name).map(|opt| opt.map(ActressItem::from))
}

#[tauri::command]
pub fn update_actress(
    id: i64,
    data: serde_json::Value,
) -> Result<(), crate::utils::error::CommandError> {
    db::with_db(|conn| {
        if let Some(name) = data.get("name").and_then(|v| v.as_str()) {
            conn.execute("UPDATE av_actors SET name = ?1 WHERE id = ?2", rusqlite::params![name, id])?;
        }
        if let Some(year) = data.get("debut_year").and_then(|v| v.as_i64()) {
            conn.execute("UPDATE av_actors SET debut_year = ?1 WHERE id = ?2", rusqlite::params![year as i32, id])?;
        }
        if let Some(h) = data.get("height").and_then(|v| v.as_i64()) {
            conn.execute("UPDATE av_actors SET height = ?1 WHERE id = ?2", rusqlite::params![h as i32, id])?;
        }
        if let Some(b) = data.get("bust").and_then(|v| v.as_i64()) {
            conn.execute("UPDATE av_actors SET bust = ?1 WHERE id = ?2", rusqlite::params![b as i32, id])?;
        }
        if let Some(w) = data.get("waist").and_then(|v| v.as_i64()) {
            conn.execute("UPDATE av_actors SET waist = ?1 WHERE id = ?2", rusqlite::params![w as i32, id])?;
        }
        if let Some(h) = data.get("hip").and_then(|v| v.as_i64()) {
            conn.execute("UPDATE av_actors SET hip = ?1 WHERE id = ?2", rusqlite::params![h as i32, id])?;
        }
        if let Some(cup) = data.get("cup").and_then(|v| v.as_str()) {
            conn.execute("UPDATE av_actors SET cup = ?1 WHERE id = ?2", rusqlite::params![cup, id])?;
        }
        Ok(())
    })
}

#[tauri::command]
pub fn delete_actresses(ids: Vec<i64>) -> Result<i64, crate::utils::error::CommandError> {
    Ok(actress_sync::delete_actresses(&ids)? as i64)
}

#[tauri::command]
pub fn get_actress_aliases(actress_id: i64) -> Result<Vec<String>, crate::utils::error::CommandError> {
    db::with_db(|conn| queries::get_actress_aliases(conn, actress_id))
}

#[tauri::command]
pub fn add_actress_alias(actress_id: i64, alias: String) -> Result<(), crate::utils::error::CommandError> {
    db::with_db(|conn| queries::add_actress_alias(conn, actress_id, &alias))
}

#[tauri::command]
pub fn merge_actresses(source_id: i64, target_id: i64) -> Result<(), crate::utils::error::CommandError> {
    actress_sync::merge_actresses(source_id, target_id)
}
