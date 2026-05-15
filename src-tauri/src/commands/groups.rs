use crate::db::{self, queries};
use crate::utils::error::CommandResult;
use serde::Serialize;

#[derive(Serialize)]
pub struct GroupItem {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub group_type: String,
    pub sort_order: i64,
    pub movie_count: i64,
}

#[tauri::command]
pub fn clean_all_groups() -> Result<String, crate::utils::error::CommandError> {
    db::with_db(|conn| {
        let mg = conn.execute("DELETE FROM movie_groups", [])?;
        let ag = conn.execute("DELETE FROM actress_group_members", [])?;
        let g = conn.execute("DELETE FROM groups", [])?;
        let ag2 = conn.execute("DELETE FROM actress_groups", [])?;
        Ok(format!("已清理: {}个影片分组, {}个演员分组, {}个影片关联, {}个演员关联", g, ag2, mg, ag))
    })
}

#[tauri::command]
pub fn get_groups(category: Option<String>) -> Result<Vec<GroupItem>, crate::utils::error::CommandError> {
    let cat = category.unwrap_or_else(|| "all".to_string());
    db::with_db(|conn| {
        let rows = queries::get_all_groups(conn, if cat == "all" { None } else { Some(&cat) })?;
        let groups = rows.into_iter().map(|r| GroupItem {
            id: r.id, name: r.name, group_type: r.group_type,
            sort_order: r.sort_order, movie_count: r.movie_count,
        }).collect();
        Ok(groups)
    })
}

#[tauri::command]
pub fn create_group(
    name: String,
    group_type: Option<String>,
) -> Result<GroupItem, crate::utils::error::CommandError> {
    let gtype = group_type.unwrap_or_else(|| "manual".to_string());

    // Check for duplicate name within the same type (允许跨类型同名)
    db::with_db(|conn| {
        let exists: bool = conn
            .query_row("SELECT COUNT(*) > 0 FROM groups WHERE name = ?1 AND type = ?2", rusqlite::params![&name, &gtype], |r| r.get(0))
            .unwrap_or(false);
        if exists {
            return Err(crate::utils::error::CommandError::invalid_input(&format!("该类型下分组「{}」已存在", name)));
        }
        Ok(())
    })?;

    let id = db::with_db(|conn| queries::create_group(conn, &name, &gtype))?;
    Ok(GroupItem {
        id,
        name,
        group_type: gtype,
        sort_order: 0,
        movie_count: 0,
    })
}

#[tauri::command]
pub fn rename_group(group_id: i64, new_name: String) -> Result<(), crate::utils::error::CommandError> {
    db::with_db(|conn| queries::rename_group(conn, group_id, &new_name))
}

#[tauri::command]
pub fn delete_group(group_id: i64) -> Result<(), crate::utils::error::CommandError> {
    db::with_db(|conn| queries::delete_group(conn, group_id))
}

#[tauri::command]
pub fn reorder_groups(ordered_ids: Vec<i64>) -> Result<(), crate::utils::error::CommandError> {
    db::with_db(|conn| {
        for (i, id) in ordered_ids.iter().enumerate() {
            conn.execute(
                "UPDATE groups SET sort_order = ?1 WHERE id = ?2",
                rusqlite::params![i as i64, id],
            )?;
        }
        Ok(())
    })
}

#[tauri::command]
pub fn add_movies_to_group(group_id: i64, file_ids: Vec<String>) -> Result<(), crate::utils::error::CommandError> {
    db::with_db(|conn| queries::add_movies_to_group(conn, group_id, &file_ids))
}

#[tauri::command]
pub fn remove_movie_from_group(group_id: i64, file_id: String) -> Result<(), crate::utils::error::CommandError> {
    db::with_db(|conn| queries::remove_movie_from_group(conn, group_id, &file_id))
}

// ─── Actress Groups ───

#[derive(Serialize)]
pub struct ActressGroupItem {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
    pub member_count: i64,
}

#[tauri::command]
pub fn get_actress_groups() -> Result<Vec<ActressGroupItem>, crate::utils::error::CommandError> {
    db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT ag.id, ag.name, ag.sort_order, COUNT(agm.actress_id)
             FROM actress_groups ag LEFT JOIN actress_group_members agm ON ag.id = agm.group_id
             GROUP BY ag.id ORDER BY ag.sort_order"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ActressGroupItem {
                id: row.get(0)?,
                name: row.get(1)?,
                sort_order: row.get(2)?,
                member_count: row.get(3)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(rows)
    })
}

#[tauri::command]
pub fn create_actress_group(name: String) -> Result<ActressGroupItem, crate::utils::error::CommandError> {
    db::with_db(|conn| {
        conn.execute(
            "INSERT INTO actress_groups (name, sort_order) VALUES (?1, 0)",
            [&name],
        )?;
        let id = conn.last_insert_rowid();
        Ok(ActressGroupItem { id, name, sort_order: 0, member_count: 0 })
    })
}

#[tauri::command]
pub fn rename_actress_group(group_id: i64, new_name: String) -> Result<(), crate::utils::error::CommandError> {
    db::with_db(|conn| {
        conn.execute("UPDATE actress_groups SET name = ?1 WHERE id = ?2", rusqlite::params![new_name, group_id])?;
        Ok(())
    })
}

#[tauri::command]
pub fn delete_actress_group(group_id: i64) -> Result<(), crate::utils::error::CommandError> {
    db::with_db(|conn| {
        conn.execute("DELETE FROM actress_groups WHERE id = ?1", [group_id])?;
        Ok(())
    })
}

#[tauri::command]
pub fn add_actresses_to_group(group_id: i64, actress_ids: Vec<i64>) -> Result<(), crate::utils::error::CommandError> {
    let now = db::now_ts();
    db::with_db(|conn| {
        for aid in &actress_ids {
            conn.execute(
                "INSERT OR IGNORE INTO actress_group_members (group_id, actress_id, added_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![group_id, aid, now],
            )?;
        }
        Ok(())
    })
}

#[tauri::command]
pub fn remove_actress_from_group(group_id: i64, actress_id: i64) -> Result<(), crate::utils::error::CommandError> {
    db::with_db(|conn| {
        conn.execute(
            "DELETE FROM actress_group_members WHERE group_id = ?1 AND actress_id = ?2",
            rusqlite::params![group_id, actress_id],
        )?;
        Ok(())
    })
}
