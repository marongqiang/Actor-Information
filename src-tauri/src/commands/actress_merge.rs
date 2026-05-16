use crate::db::{self, queries};
use crate::services::{actress_folder_manager, actress_sync};
use crate::utils::error::CommandResult;
use serde::{Deserialize, Serialize};

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
    pub local_folder_name: Option<String>,
    pub is_pending: bool,
    pub source: Option<String>,
}

#[derive(Serialize)]
pub struct ActressByLetter {
    pub letter: String,
    pub actresses: Vec<ActressItem>,
}

#[derive(Serialize)]
pub struct PaginatedActress {
    pub list: Vec<ActressItem>,
    pub total: i64,
}

// ─── Sync ───

#[tauri::command]
pub async fn sync_actress_data() -> Result<(), crate::utils::error::CommandError> {
    actress_sync::sync_actress_data().await
}

// ─── Query ───

#[tauri::command]
pub fn get_actresses_by_letter() -> Result<Vec<ActressByLetter>, crate::utils::error::CommandError> {
    db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT a.id, a.name, a.avatar_local, a.debut_year, a.height, a.bust, a.waist, a.hip, a.cup, a.letter,
                    (SELECT COUNT(*) FROM movie_actors ma JOIN actors act ON ma.actor_id = act.id WHERE act.name = a.name),
                    a.local_folder_name, a.is_pending, a.source
             FROM av_actors a ORDER BY a.letter, a.name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ActressItem {
                id: row.get(0)?, name: row.get(1)?, avatar_local: row.get(2)?,
                debut_year: row.get(3)?, height: row.get(4)?, bust: row.get(5)?,
                waist: row.get(6)?, hip: row.get(7)?, cup: row.get(8)?,
                letter: row.get(9)?, movie_count: row.get(10)?,
                local_folder_name: row.get(11)?, is_pending: row.get::<_, i32>(12)? != 0,
                source: row.get(13)?,
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
            current_actresses.push(row);
        }

        if !current_actresses.is_empty() {
            letters.push(ActressByLetter { letter: current_letter, actresses: current_actresses });
        }

        Ok(letters)
    })
}

#[tauri::command]
pub fn get_actresses_paginated(
    page: i64,
    page_size: i64,
    search: Option<String>,
    sort_field: Option<String>,
    sort_order: Option<String>,
    include_pending: Option<bool>,
    group_id: Option<i64>,
) -> Result<PaginatedActress, crate::utils::error::CommandError> {
    db::with_db(|conn| {
        let mut where_parts: Vec<String> = Vec::new();
        // include_pending: None=全部, Some(true)=仅待审核, Some(false)=仅已确认
        match include_pending {
            Some(true) => where_parts.push("a.is_pending = 1".to_string()),
            Some(false) => where_parts.push("a.is_pending = 0".to_string()),
            None => {} // show all
        }
        if let Some(gid) = group_id {
            where_parts.push(format!("a.id IN (SELECT actress_id FROM actress_group_members WHERE group_id = {})", gid));
        }
        let base_where = if where_parts.is_empty() { String::new() } else { format!("WHERE {}", where_parts.join(" AND ")) };

        let (search_clause, search_param) = if let Some(ref s) = search {
            if !s.is_empty() {
                let clause = if base_where.is_empty() {
                    format!("WHERE (a.name LIKE ?1 OR EXISTS (SELECT 1 FROM actress_aliases al WHERE al.actress_id = a.id AND al.alias_name LIKE ?1))")
                } else {
                    format!("{} AND (a.name LIKE ?1 OR EXISTS (SELECT 1 FROM actress_aliases al WHERE al.actress_id = a.id AND al.alias_name LIKE ?1))", base_where)
                };
                (clause, Some(format!("%{}%", s)))
            } else {
                (base_where.to_string(), None)
            }
        } else {
            (base_where.to_string(), None)
        };

        let count_sql = format!(
            "SELECT COUNT(*) FROM av_actors a {}",
            if search_clause.is_empty() { "".into() } else { search_clause.clone() }
        );
        let total: i64 = if let Some(ref p) = search_param {
            conn.query_row(&count_sql, [p], |row| row.get(0))?
        } else {
            conn.query_row(&count_sql, [], |row| row.get(0))?
        };

        let offset = (page - 1) * page_size;

        // Dynamic sort
        let valid_fields = ["name", "debut_year", "height", "bust", "waist", "hip", "cup", "movie_count", "id", "is_pending", "source", "letter"];
        let sort_col = sort_field.as_deref().filter(|f| valid_fields.contains(f)).unwrap_or("name");
        let sort_dir = sort_order.as_deref().unwrap_or("asc");
        let order_clause = if sort_col == "movie_count" {
            format!("ORDER BY movie_count {}", sort_dir)
        } else {
            format!("ORDER BY a.\"{}\" {}", sort_col.replace('\'', "''"), sort_dir)
        };

        let query_sql = format!(
            "SELECT a.id, a.name, a.avatar_local, a.debut_year, a.height, a.bust, a.waist, a.hip, a.cup, a.letter,
                    (SELECT COUNT(*) FROM movie_actors ma JOIN actors act ON ma.actor_id = act.id WHERE act.name = a.name) as movie_count,
                    a.local_folder_name, a.is_pending, a.source
             FROM av_actors a {} {} LIMIT ?{} OFFSET ?{}",
            search_clause, order_clause,
            if search_param.is_some() { 2 } else { 1 },
            if search_param.is_some() { 3 } else { 2 },
        );

        let mut stmt = conn.prepare(&query_sql)?;
        let rows: Vec<ActressItem> = if let Some(ref p) = search_param {
            stmt.query_map(rusqlite::params![p, page_size, offset], |row| {
                Ok(ActressItem {
                    id: row.get(0)?, name: row.get(1)?, avatar_local: row.get(2)?,
                    debut_year: row.get(3)?, height: row.get(4)?, bust: row.get(5)?,
                    waist: row.get(6)?, hip: row.get(7)?, cup: row.get(8)?,
                    letter: row.get(9)?, movie_count: row.get(10)?,
                    local_folder_name: row.get(11)?, is_pending: row.get::<_, i32>(12)? != 0,
                    source: row.get(13)?,
                })
            })?.filter_map(|r| r.ok()).collect()
        } else {
            stmt.query_map(rusqlite::params![page_size, offset], |row| {
                Ok(ActressItem {
                    id: row.get(0)?, name: row.get(1)?, avatar_local: row.get(2)?,
                    debut_year: row.get(3)?, height: row.get(4)?, bust: row.get(5)?,
                    waist: row.get(6)?, hip: row.get(7)?, cup: row.get(8)?,
                    letter: row.get(9)?, movie_count: row.get(10)?,
                    local_folder_name: row.get(11)?, is_pending: row.get::<_, i32>(12)? != 0,
                    source: row.get(13)?,
                })
            })?.filter_map(|r| r.ok()).collect()
        };

        Ok(PaginatedActress { list: rows, total })
    })
}

#[tauri::command]
pub fn find_actress(name: String) -> Result<Option<ActressItem>, crate::utils::error::CommandError> {
    actress_sync::find_actress(&name).map(|opt| {
        opt.map(|row| ActressItem {
            id: row.id, name: row.name, avatar_local: row.avatar_local,
            debut_year: row.debut_year, height: row.height, bust: row.bust,
            waist: row.waist, hip: row.hip, cup: row.cup, letter: row.letter,
            movie_count: row.movie_count,
            local_folder_name: None, is_pending: false, source: None,
        })
    })
}

// ─── CRUD ───

#[tauri::command]
pub fn update_actress(id: i64, data: serde_json::Value) -> Result<(), crate::utils::error::CommandError> {
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
        if let Some(src) = data.get("source").and_then(|v| v.as_str()) {
            conn.execute("UPDATE av_actors SET source = ?1 WHERE id = ?2", rusqlite::params![src, id])?;
        }
        if let Some(pending) = data.get("is_pending").and_then(|v| v.as_bool()) {
            conn.execute("UPDATE av_actors SET is_pending = ?1 WHERE id = ?2", rusqlite::params![pending as i32, id])?;
        }
        Ok(())
    })
}

#[tauri::command]
pub fn delete_actresses(ids: Vec<i64>) -> Result<i64, crate::utils::error::CommandError> {
    Ok(actress_sync::delete_actresses(&ids)? as i64)
}

#[tauri::command]
pub fn delete_all_actresses() -> Result<i64, crate::utils::error::CommandError> {
    db::with_db(|conn| {
        let count = conn.execute("DELETE FROM av_actors", [])?;
        conn.execute("DELETE FROM actress_aliases", [])?;
        conn.execute("DELETE FROM actress_group_members", [])?;
        Ok(count as i64)
    })
}

#[tauri::command]
pub fn get_actress_aliases(actress_id: i64) -> Result<Vec<String>, crate::utils::error::CommandError> {
    db::with_db(|conn| queries::get_actress_aliases(conn, actress_id))
}

#[tauri::command]
pub fn add_actress_alias(actress_id: i64, alias: String) -> Result<(), crate::utils::error::CommandError> {
    db::with_db(|conn| queries::add_actress_alias(conn, actress_id, &alias))
}

// ─── Local Folder Commands (new in 1.2.0) ───

#[tauri::command]
pub fn scan_local_actress_folder(folder_path: Option<String>) -> Result<actress_folder_manager::ScanResult, crate::utils::error::CommandError> {
    actress_folder_manager::scan_local_actress_folder(folder_path)
}

#[tauri::command]
pub fn refresh_actress_avatar(actress_id: i64) -> Result<(), crate::utils::error::CommandError> {
    actress_folder_manager::refresh_actress_avatar(actress_id)
}

#[tauri::command]
pub fn confirm_actor(actor_id: i64, accepted: bool) -> Result<(), crate::utils::error::CommandError> {
    actress_folder_manager::confirm_actor(actor_id, accepted)
}

#[tauri::command]
pub fn update_actor_local_folder(actor_id: i64, folder_path: Option<String>) -> Result<(), crate::utils::error::CommandError> {
    actress_folder_manager::update_actor_local_folder(actor_id, folder_path)
}

#[tauri::command]
pub fn rename_actor_and_folder(actor_id: i64, new_name: String, rename_folder: bool) -> Result<actress_folder_manager::RenameResult, crate::utils::error::CommandError> {
    actress_folder_manager::rename_actor_and_folder(actor_id, &new_name, rename_folder)
}

#[tauri::command]
pub fn merge_actresses(source_id: i64, target_id: i64, options: actress_folder_manager::MergeOptions) -> Result<actress_folder_manager::MergeResult, crate::utils::error::CommandError> {
    actress_folder_manager::merge_actresses(source_id, target_id, options)
}

#[tauri::command]
pub fn detect_duplicate_actresses(threshold: Option<f64>) -> Result<Vec<actress_folder_manager::DuplicatePair>, crate::utils::error::CommandError> {
    actress_folder_manager::detect_duplicate_actresses(threshold)
}

#[tauri::command]
pub fn get_actress_local_folder(actress_id: i64) -> Result<Option<String>, crate::utils::error::CommandError> {
    actress_folder_manager::get_actress_local_folder(actress_id)
}

#[tauri::command]
pub fn sync_actress_with_local_folder(actress_id: i64) -> Result<(), crate::utils::error::CommandError> {
    actress_folder_manager::sync_actress_with_local_folder(actress_id)
}
