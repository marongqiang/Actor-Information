use rusqlite::{params, Connection};
use crate::utils::error::{CommandError, CommandResult};

// ─── Movies ───

pub fn insert_movie(conn: &Connection, m: &InsertMovie) -> CommandResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO movies (file_id, title, original_title, year, poster_url, poster_local, backdrop_url,
         overview, rating, runtime, director, genre, file_name, file_size, created_at, updated_at, is_hidden)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
        params![m.file_id, m.title, m.original_title, m.year, m.poster_url, m.poster_local, m.backdrop_url,
                m.overview, m.rating, m.runtime, m.director, m.genre, m.file_name, m.file_size,
                m.created_at, m.updated_at, m.is_hidden],
    )?;
    Ok(())
}

pub struct InsertMovie {
    pub file_id: String,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    pub poster_url: Option<String>,
    pub poster_local: Option<String>,
    pub backdrop_url: Option<String>,
    pub overview: Option<String>,
    pub rating: Option<f64>,
    pub runtime: Option<i32>,
    pub director: Option<String>,
    pub genre: Option<String>,
    pub file_name: String,
    pub file_size: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_hidden: bool,
}

pub fn get_movies_paginated(
    conn: &Connection,
    keyword: Option<&str>,
    year: Option<i32>,
    genre: Option<&str>,
    group_id: Option<i64>,
    is_hidden: Option<bool>,
    favorites_only: Option<bool>,
    sort: &str,
    page: i64,
    page_size: i64,
) -> CommandResult<(Vec<MovieRow>, i64)> {
    let mut where_clauses = vec!["1=1".to_string()];

    // favorites_only: only show movies in groups with type='favorite'
    if favorites_only == Some(true) {
        where_clauses.push("m.file_id IN (SELECT movie_id FROM movie_groups mg JOIN groups g ON mg.group_id = g.id WHERE g.type = 'favorite')".to_string());
    }
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(kw) = keyword {
        if !kw.is_empty() {
            param_values.push(Box::new(format!("%{}%", kw)));
            where_clauses.push(format!("title LIKE ?{}", param_values.len()));
        }
    }
    if let Some(y) = year {
        param_values.push(Box::new(y));
        where_clauses.push(format!("year = ?{}", param_values.len()));
    }
    if let Some(g) = genre {
        if !g.is_empty() {
            param_values.push(Box::new(format!("%\"{}\"%", g)));
            where_clauses.push(format!("genre LIKE ?{}", param_values.len()));
        }
    }
    if let Some(gid) = group_id {
        param_values.push(Box::new(gid));
        where_clauses.push(format!(
            "m.file_id IN (SELECT movie_id FROM movie_groups WHERE group_id = ?{})",
            param_values.len()
        ));
    }
    if let Some(hidden) = is_hidden {
        param_values.push(Box::new(hidden as i32));
        where_clauses.push(format!("m.is_hidden = ?{}", param_values.len()));
    } else {
        where_clauses.push("m.is_hidden = 0".to_string());
    }

    let where_sql = where_clauses.join(" AND ");

    let order = match sort {
        "year_desc" => "year DESC",
        "year_asc" => "year ASC",
        "title_asc" => "title ASC",
        "rating_desc" => "rating DESC",
        _ => "updated_at DESC",
    };

    // Count total
    let count_sql = format!("SELECT COUNT(*) FROM movies m WHERE {}", where_sql);
    let total: i64 = {
        let mut stmt = conn.prepare(&count_sql)?;
        let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
        stmt.query_row(params_ref.as_slice(), |row| row.get(0))?
    };

    // Fetch page
    let offset = (page - 1) * page_size;
    let query_sql = format!(
        "SELECT m.file_id, m.title, m.year, m.poster_local, m.rating, m.genre, m.is_hidden,
                COALESCE(p.progress, 0), COALESCE(p.duration, 0)
         FROM movies m
         LEFT JOIN play_progress p ON m.file_id = p.file_id
         WHERE {}
         ORDER BY m.{} LIMIT ?{} OFFSET ?{}",
        where_sql,
        order,
        param_values.len() + 1,
        param_values.len() + 2,
    );

    let mut final_params: Vec<Box<dyn rusqlite::types::ToSql>> = param_values;
    final_params.push(Box::new(page_size));
    final_params.push(Box::new(offset));

    let mut stmt = conn.prepare(&query_sql)?;
    let params_ref: Vec<&dyn rusqlite::types::ToSql> = final_params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(params_ref.as_slice(), |row| {
        Ok(MovieRow {
            file_id: row.get(0)?,
            title: row.get(1)?,
            year: row.get(2)?,
            poster_local: row.get(3)?,
            rating: row.get(4)?,
            genre: row.get(5)?,
            is_hidden: row.get::<_, i32>(6)? != 0,
            progress: row.get(7)?,
            duration: row.get(8)?,
        })
    })?.filter_map(|r| r.ok()).collect();

    Ok((rows, total))
}

pub struct MovieRow {
    pub file_id: String,
    pub title: String,
    pub year: Option<i32>,
    pub poster_local: Option<String>,
    pub rating: Option<f64>,
    pub genre: Option<String>,
    pub is_hidden: bool,
    pub progress: i64,
    pub duration: i64,
}

pub fn get_movie_detail(conn: &Connection, file_id: &str) -> CommandResult<Option<MovieDetailRow>> {
    let result = conn.query_row(
        "SELECT file_id, title, original_title, year, poster_url, poster_local, backdrop_url,
         overview, rating, runtime, director, genre, file_name, file_size, created_at, updated_at, is_hidden
         FROM movies WHERE file_id = ?1",
        [file_id],
        |row| {
            Ok(MovieDetailRow {
                file_id: row.get(0)?,
                title: row.get(1)?,
                original_title: row.get(2)?,
                year: row.get(3)?,
                poster_url: row.get(4)?,
                poster_local: row.get(5)?,
                backdrop_url: row.get(6)?,
                overview: row.get(7)?,
                rating: row.get(8)?,
                runtime: row.get(9)?,
                director: row.get(10)?,
                genre: row.get(11)?,
                file_name: row.get(12)?,
                file_size: row.get(13)?,
                created_at: row.get(14)?,
                updated_at: row.get(15)?,
                is_hidden: row.get::<_, i32>(16)? != 0,
            })
        },
    ).optional().map_err(|e| CommandError::db(&format!("查询影片详情失败: {}", e)))?;

    Ok(result)
}

pub struct MovieDetailRow {
    pub file_id: String,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    pub poster_url: Option<String>,
    pub poster_local: Option<String>,
    pub backdrop_url: Option<String>,
    pub overview: Option<String>,
    pub rating: Option<f64>,
    pub runtime: Option<i32>,
    pub director: Option<String>,
    pub genre: Option<String>,
    pub file_name: String,
    pub file_size: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_hidden: bool,
}

pub fn get_movie_actors(conn: &Connection, file_id: &str) -> CommandResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT a.name FROM actors a JOIN movie_actors ma ON a.id = ma.actor_id WHERE ma.movie_id = ?1"
    )?;
    let names = stmt.query_map([file_id], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(names)
}

pub fn get_movie_groups(conn: &Connection, file_id: &str) -> CommandResult<Vec<GroupRow>> {
    let mut stmt = conn.prepare(
        "SELECT g.id, g.name, g.type FROM groups g JOIN movie_groups mg ON g.id = mg.group_id WHERE mg.movie_id = ?1"
    )?;
    let groups = stmt.query_map([file_id], |row| {
        Ok(GroupRow { id: row.get(0)?, name: row.get(1)?, group_type: row.get(2)? })
    })?.filter_map(|r| r.ok()).collect();
    Ok(groups)
}

pub struct GroupRow {
    pub id: i64,
    pub name: String,
    pub group_type: String,
}

// ─── Config ───

pub fn get_config(conn: &Connection, key: &str) -> CommandResult<Option<String>> {
    conn.query_row("SELECT value FROM config WHERE key = ?1", [key], |row| row.get(0))
        .optional()
        .map_err(|e| CommandError::db(&format!("查询配置失败: {}", e)))
}

pub fn set_config(conn: &Connection, key: &str, value: &str) -> CommandResult<()> {
    let now = crate::db::now_ts();
    conn.execute(
        "INSERT OR REPLACE INTO config (key, value, updated_at) VALUES (?1, ?2, ?3)",
        params![key, value, now],
    )?;
    Ok(())
}

pub fn get_all_config(conn: &Connection) -> CommandResult<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT key, value FROM config WHERE value IS NOT NULL")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?.filter_map(|r| r.ok()).collect();
    Ok(rows)
}

// ─── Play Progress ───

pub fn get_progress(conn: &Connection, file_id: &str) -> CommandResult<ProgressRow> {
    conn.query_row(
        "SELECT progress, duration, is_finished FROM play_progress WHERE file_id = ?1",
        [file_id],
        |row| Ok(ProgressRow {
            progress: row.get(0)?,
            duration: row.get(1)?,
            is_finished: row.get::<_, i32>(2)?,
        }),
    ).or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(ProgressRow { progress: 0, duration: 0, is_finished: 0 }),
        e => Err(CommandError::db(&format!("查询进度失败: {}", e))),
    })
}

pub struct ProgressRow {
    pub progress: i64,
    pub duration: i64,
    pub is_finished: i32,
}

pub fn save_progress_row(conn: &Connection, file_id: &str, progress: i64, duration: i64) -> CommandResult<()> {
    let now = crate::db::now_ts();
    conn.execute(
        "INSERT OR REPLACE INTO play_progress (file_id, progress, duration, is_finished, updated_at)
         VALUES (?1, ?2, ?3, CASE WHEN ?2 >= ?3 AND ?3 > 0 THEN 1 ELSE 0 END, ?4)",
        params![file_id, progress, duration, now],
    )?;
    Ok(())
}

pub fn end_playback(conn: &Connection, file_id: &str) -> CommandResult<()> {
    let now = crate::db::now_ts();
    conn.execute(
        "INSERT OR REPLACE INTO play_progress (file_id, progress, duration, is_finished, updated_at)
         SELECT ?1, duration, duration, 1, ?2 FROM play_progress WHERE file_id = ?1",
        params![file_id, now],
    )?;
    Ok(())
}

// ─── Groups ───

pub fn get_all_groups(conn: &Connection, category: Option<&str>) -> CommandResult<Vec<GroupRowWithCount>> {
    let (where_clause, param): (String, Option<String>) = match category {
        Some(c) => ("WHERE g.type = ?1".to_string(), Some(c.to_string())),
        None => (String::new(), None),
    };
    let sql = format!(
        "SELECT g.id, g.name, g.type, g.sort_order, COUNT(mg.movie_id)
         FROM groups g LEFT JOIN movie_groups mg ON g.id = mg.group_id
         {} GROUP BY g.id ORDER BY g.sort_order",
        where_clause
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<GroupRowWithCount> = if let Some(ref p) = param {
        stmt.query_map(rusqlite::params![p], |row| {
            Ok(GroupRowWithCount {
                id: row.get(0)?, name: row.get(1)?, group_type: row.get(2)?,
                sort_order: row.get(3)?, movie_count: row.get(4)?,
            })
        })?.filter_map(|r| r.ok()).collect()
    } else {
        stmt.query_map([], |row| {
            Ok(GroupRowWithCount {
                id: row.get(0)?, name: row.get(1)?, group_type: row.get(2)?,
                sort_order: row.get(3)?, movie_count: row.get(4)?,
            })
        })?.filter_map(|r| r.ok()).collect()
    };
    Ok(rows)
}

pub struct GroupRowWithCount {
    pub id: i64,
    pub name: String,
    pub group_type: String,
    pub sort_order: i64,
    pub movie_count: i64,
}

pub fn create_group(conn: &Connection, name: &str, group_type: &str) -> CommandResult<i64> {
    let now = crate::db::now_ts();
    conn.execute(
        "INSERT INTO groups (name, type, sort_order, created_at) VALUES (?1, ?2, 0, ?3)",
        params![name, group_type, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_group(conn: &Connection, group_id: i64) -> CommandResult<()> {
    conn.execute("DELETE FROM groups WHERE id = ?1", [group_id])?;
    Ok(())
}

pub fn rename_group(conn: &Connection, group_id: i64, new_name: &str) -> CommandResult<()> {
    conn.execute("UPDATE groups SET name = ?1 WHERE id = ?2", params![new_name, group_id])?;
    Ok(())
}

pub fn add_movies_to_group(conn: &Connection, group_id: i64, file_ids: &[String]) -> CommandResult<()> {
    let now = crate::db::now_ts();
    for fid in file_ids {
        conn.execute(
            "INSERT OR IGNORE INTO movie_groups (group_id, movie_id, added_at) VALUES (?1, ?2, ?3)",
            params![group_id, fid, now],
        )?;
    }
    Ok(())
}

pub fn remove_movie_from_group(conn: &Connection, group_id: i64, file_id: &str) -> CommandResult<()> {
    conn.execute("DELETE FROM movie_groups WHERE group_id = ?1 AND movie_id = ?2", params![group_id, file_id])?;
    Ok(())
}

// ─── Actors / Actress ───

pub fn insert_actor(conn: &Connection, name: &str) -> CommandResult<i64> {
    conn.execute("INSERT OR IGNORE INTO actors (name) VALUES (?1)", [name])?;
    Ok(conn.last_insert_rowid())
}

pub fn link_actor_to_movie(conn: &Connection, movie_id: &str, actor_id: i64) -> CommandResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO movie_actors (movie_id, actor_id) VALUES (?1, ?2)",
        params![movie_id, actor_id],
    )?;
    Ok(())
}

pub fn get_actresses_paginated(
    conn: &Connection,
    page: i64,
    page_size: i64,
    search: Option<&str>,
) -> CommandResult<(Vec<ActressRow>, i64)> {
    let (where_clause, param): (String, Option<String>) = if let Some(s) = search {
        if s.is_empty() {
            ("1=1".into(), None)
        } else {
            (format!("WHERE (a.name LIKE ?1 OR EXISTS (SELECT 1 FROM actress_aliases al WHERE al.actress_id = a.id AND al.alias_name LIKE ?1))"), Some(format!("%{}%", s)))
        }
    } else {
        ("1=1".into(), None)
    };

    let count_sql = format!("SELECT COUNT(*) FROM av_actors a {}", where_clause);
    let total: i64 = if let Some(ref p) = param {
        conn.query_row(&count_sql, [p], |row| row.get(0))?
    } else {
        conn.query_row(&count_sql, [], |row| row.get(0))?
    };

    let offset = (page - 1) * page_size;
    let query_sql = format!(
        "SELECT a.id, a.name, a.avatar_local, a.debut_year, a.height, a.bust, a.waist, a.hip, a.cup, a.letter,
                (SELECT COUNT(*) FROM movie_actors ma JOIN actors act ON ma.actor_id = act.id WHERE act.name = a.name)
         FROM av_actors a {} ORDER BY a.name LIMIT ?{} OFFSET ?{}",
        where_clause,
        if param.is_some() { 2 } else { 1 },
        if param.is_some() { 3 } else { 2 },
    );

    let mut stmt = conn.prepare(&query_sql)?;
    let rows = if let Some(ref p) = param {
        stmt.query_map(params![p, page_size, offset], |row| {
            Ok(ActressRow {
                id: row.get(0)?, name: row.get(1)?, avatar_local: row.get(2)?,
                debut_year: row.get(3)?, height: row.get(4)?, bust: row.get(5)?,
                waist: row.get(6)?, hip: row.get(7)?, cup: row.get(8)?,
                letter: row.get(9)?, movie_count: row.get(10)?,
            })
        })?.filter_map(|r| r.ok()).collect()
    } else {
        stmt.query_map(params![page_size, offset], |row| {
            Ok(ActressRow {
                id: row.get(0)?, name: row.get(1)?, avatar_local: row.get(2)?,
                debut_year: row.get(3)?, height: row.get(4)?, bust: row.get(5)?,
                waist: row.get(6)?, hip: row.get(7)?, cup: row.get(8)?,
                letter: row.get(9)?, movie_count: row.get(10)?,
            })
        })?.filter_map(|r| r.ok()).collect()
    };

    Ok((rows, total))
}

pub struct ActressRow {
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

pub fn get_actress_aliases(conn: &Connection, actress_id: i64) -> CommandResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT alias_name FROM actress_aliases WHERE actress_id = ?1")?;
    let names = stmt.query_map([actress_id], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok()).collect();
    Ok(names)
}

pub fn add_actress_alias(conn: &Connection, actress_id: i64, alias: &str) -> CommandResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO actress_aliases (actress_id, alias_name) VALUES (?1, ?2)",
        params![actress_id, alias],
    )?;
    Ok(())
}

// ─── Tasks ───

pub fn insert_task(conn: &Connection, task: &TaskRow) -> CommandResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO tasks (id, type, target_ids, status, progress, result, created_at, updated_at, error, checkpoint)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![task.id, task.task_type, task.target_ids, task.status, task.progress,
                task.result, task.created_at, task.updated_at, task.error, task.checkpoint],
    )?;
    Ok(())
}

pub struct TaskRow {
    pub id: String,
    pub task_type: String,
    pub target_ids: String,
    pub status: String,
    pub progress: i32,
    pub result: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub error: Option<String>,
    pub checkpoint: Option<String>,
}

pub fn get_pending_tasks(conn: &Connection) -> CommandResult<Vec<TaskRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, type, target_ids, status, progress, result, created_at, updated_at, error, checkpoint
         FROM tasks WHERE status IN ('pending', 'running', 'paused') ORDER BY created_at"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(TaskRow {
            id: row.get(0)?, task_type: row.get(1)?, target_ids: row.get(2)?,
            status: row.get(3)?, progress: row.get(4)?, result: row.get(5)?,
            created_at: row.get(6)?, updated_at: row.get(7)?,
            error: row.get(8)?, checkpoint: row.get(9)?,
        })
    })?.filter_map(|r| r.ok()).collect();
    Ok(rows)
}

// Helper for optional query results
trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
