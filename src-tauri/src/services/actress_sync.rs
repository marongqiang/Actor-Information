use crate::db;
use crate::utils::error::CommandResult;

/// Sync actress data: first scan local folders, then sync from movie actors.
/// Local folder actors take priority, new scraped actors are marked pending.
pub async fn sync_actress_data() -> CommandResult<()> {
    log::info!("开始同步演员数据...");

    // 1. Scan local folders first (priority)
    match super::actress_folder_manager::scan_local_actress_folder(None) {
        Ok(result) => {
            log::info!("本地文件夹扫描完成: 新增 {} / 总计 {}", result.added, result.total);
        }
        Err(e) => {
            log::warn!("本地文件夹扫描失败（可能目录不存在）: {}", e);
        }
    }

    // 2. Sync from movie actors table to av_actors
    db::with_db(|conn| {
        // Import actors from movies into av_actors if they don't exist
        // New actors from scrape are marked as pending
        conn.execute(
            "INSERT OR IGNORE INTO av_actors (name, letter, is_pending, source)
             SELECT DISTINCT a.name, SUBSTR(UPPER(a.name), 1, 1), 1, 'scrape'
             FROM actors a
             WHERE a.name NOT IN (SELECT name FROM av_actors)",
            [],
        )?;

        let count = conn.changes();
        log::info!("从影片演员表同步了 {} 位新演员", count);
        Ok(())
    })?;

    // 3. Update actor letter indexes
    db::with_db(|conn| {
        conn.execute(
            "UPDATE av_actors SET letter = SUBSTR(UPPER(name), 1, 1) WHERE letter IS NULL",
            [],
        )?;
        Ok(())
    })?;

    log::info!("演员数据同步完成");
    Ok(())
}

/// Find an actress by name, checking aliases too
pub fn find_actress(name: &str) -> CommandResult<Option<crate::db::queries::ActressRow>> {
    db::with_db(|conn| {
        // Try direct match first
        let result = conn.query_row(
            "SELECT id, name, avatar_local, debut_year, height, bust, waist, hip, cup, letter,
                    (SELECT COUNT(*) FROM movie_actors ma JOIN actors a ON ma.actor_id = a.id WHERE a.name = av_actors.name)
             FROM av_actors WHERE name = ?1",
            [name],
            |row| {
                Ok(crate::db::queries::ActressRow {
                    id: row.get(0)?, name: row.get(1)?, avatar_local: row.get(2)?,
                    debut_year: row.get(3)?, height: row.get(4)?, bust: row.get(5)?,
                    waist: row.get(6)?, hip: row.get(7)?, cup: row.get(8)?,
                    letter: row.get(9)?, movie_count: row.get(10)?,
                })
            },
        );

        match result {
            Ok(row) => Ok(Some(row)),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // Try alias
                let alias_result = conn.query_row(
                    "SELECT a.id, a.name, a.avatar_local, a.debut_year, a.height, a.bust, a.waist, a.hip, a.cup, a.letter,
                            (SELECT COUNT(*) FROM movie_actors ma JOIN actors act ON ma.actor_id = act.id WHERE act.name = a.name)
                     FROM av_actors a JOIN actress_aliases al ON a.id = al.actress_id
                     WHERE al.alias_name = ?1",
                    [name],
                    |row| {
                        Ok(crate::db::queries::ActressRow {
                            id: row.get(0)?, name: row.get(1)?, avatar_local: row.get(2)?,
                            debut_year: row.get(3)?, height: row.get(4)?, bust: row.get(5)?,
                            waist: row.get(6)?, hip: row.get(7)?, cup: row.get(8)?,
                            letter: row.get(9)?, movie_count: row.get(10)?,
                        })
                    },
                );

                match alias_result {
                    Ok(row) => Ok(Some(row)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(crate::utils::error::CommandError::db(&format!("查询演员失败: {}", e))),
                }
            }
            Err(e) => Err(crate::utils::error::CommandError::db(&format!("查询演员失败: {}", e))),
        }
    })
}

/// Merge two actress records
pub fn merge_actresses(source_id: i64, target_id: i64) -> CommandResult<()> {
    db::with_db(|conn| {
        // Move aliases
        conn.execute(
            "UPDATE actress_aliases SET actress_id = ?1 WHERE actress_id = ?2",
            rusqlite::params![target_id, source_id],
        )?;

        // Move group memberships
        conn.execute(
            "INSERT OR IGNORE INTO actress_group_members (group_id, actress_id, added_at)
             SELECT group_id, ?1, ?2 FROM actress_group_members WHERE actress_id = ?3",
            rusqlite::params![target_id, crate::db::now_ts(), source_id],
        )?;
        conn.execute("DELETE FROM actress_group_members WHERE actress_id = ?1", [source_id])?;

        // Move movie actor associations
        let source_name: String = conn.query_row(
            "SELECT name FROM av_actors WHERE id = ?1", [source_id], |row| row.get(0)
        )?;
        let target_name: String = conn.query_row(
            "SELECT name FROM av_actors WHERE id = ?1", [target_id], |row| row.get(0)
        )?;

        // Update movie_actors to point to target actor
        let source_actor_id: Option<i64> = conn.query_row(
            "SELECT id FROM actors WHERE name = ?1", [&source_name], |row| row.get(0)
        ).ok();
        let target_actor_id: Option<i64> = conn.query_row(
            "SELECT id FROM actors WHERE name = ?1", [&target_name], |row| row.get(0)
        ).ok();

        if let (Some(sid), Some(tid)) = (source_actor_id, target_actor_id) {
            conn.execute(
                "UPDATE movie_actors SET actor_id = ?1 WHERE actor_id = ?2",
                rusqlite::params![tid, sid],
            )?;
        }

        // Delete source actress
        conn.execute("DELETE FROM av_actors WHERE id = ?1", [source_id])?;

        log::info!("合并演员完成: {} -> {}", source_name, target_name);
        Ok(())
    })
}

pub fn delete_actresses(ids: &[i64]) -> CommandResult<usize> {
    db::with_db(|conn| {
        let id_list = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
        let sql = format!("DELETE FROM av_actors WHERE id IN ({})", id_list);
        let deleted = conn.execute(&sql, [])?;
        Ok(deleted)
    })
}
