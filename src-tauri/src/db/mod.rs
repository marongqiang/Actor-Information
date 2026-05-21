use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub mod queries;

pub static DB: once_cell::sync::Lazy<Mutex<Connection>> =
    once_cell::sync::Lazy::new(|| {
        let db_path = get_db_path();
        let conn = Connection::open(&db_path).expect("无法打开数据库");

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = -20000;
             PRAGMA foreign_keys = ON;"
        ).expect("无法设置数据库 PRAGMA");

        run_migrations(&conn);
        seed_config(&conn);

        Mutex::new(conn)
    });

fn get_db_path() -> PathBuf {
    let data_dir = get_data_dir();
    std::fs::create_dir_all(&data_dir).ok();
    data_dir.join("vault.db")
}

pub fn get_data_dir() -> PathBuf {
    // Use exe directory so data stays with the app, not in AppData
    std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("data")
}

fn run_migrations(conn: &Connection) {
    let migrations = vec![
        ("v1", include_str!("migrations/v1.sql")),
        ("v2", include_str!("migrations/v2.sql")),
        ("v3", include_str!("migrations/v3.sql")),
        ("v4", include_str!("migrations/v4.sql")),
        ("v5", include_str!("migrations/v5.sql")),
        ("v6", include_str!("migrations/v6.sql")),
        ("v7", include_str!("migrations/v7.sql")),
        ("v8", include_str!("migrations/v8.sql")),
        ("v9", include_str!("migrations/v9.sql")),
        ("v10", include_str!("migrations/v10.sql")),
    ];

    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version TEXT PRIMARY KEY,
            applied_at INTEGER NOT NULL
        )",
        [],
    ).ok();

    for (version, sql) in &migrations {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM schema_version WHERE version = ?1",
                [version],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !exists {
            // Execute each statement individually, ignoring "duplicate column" errors
            for stmt in sql.split(';') {
                let stmt = stmt.trim();
                if stmt.is_empty() { continue; }
                if let Err(e) = conn.execute(stmt, []) {
                    let msg = e.to_string().to_lowercase();
                    if msg.contains("duplicate column") || msg.contains("already exists") {
                        log::warn!("迁移 {}: 跳过重复语句", version);
                    } else {
                        panic!("迁移 {} 失败: {} | SQL前80字节: {}", version, e, truncate_str(stmt, 80));
                    }
                }
            }
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            conn.execute(
                "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
                rusqlite::params![version, now],
            ).ok();
        }
    }
}

fn seed_config(conn: &Connection) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let defaults = vec![
        ("db_version", "5"),
        ("scan_depth", "5"),
        ("cache_max_size", "2147483648"),
        ("theme", "dark"),
        ("poster_size", "medium"),
        ("font_size", "14"),
        ("auto_start", "false"),
        ("privacy_title", "智能网盘影视库"),
        ("privacy_tray", "智能网盘影视库"),
        ("proxy_enabled", "false"),
        ("external_player", ""),
        ("scrape_sources", r#"["tmdb","imdb","douban","javbus","javdb","fanza","airav","xcity","mgstage","fc2","jav321","javlibrary","arzon"]"#),
        ("video_extensions", r#"["mp4","mkv","avi","mov","rmvb","flv","wmv","ts","iso","m2ts"]"#),
        ("use_system_credential", "0"),
        ("auto_resume_tasks", "1"),
        ("playback_refresh_interval", "240"),
        ("local_actor_base_dir", r"D:\Media Library\Actor Information\picture"),
        ("auto_create_actors_from_scrape", "1"),
        ("actor_pending_review", "1"),
        ("allow_app_rename_actor_folders", "0"),
        ("actor_merge_auto_merge_folders", "1"),
        ("actor_merge_file_naming_pattern", "{name}_{index}{ext}"),
        ("actor_merge_dry_run", "1"),
    ];

    for (key, value) in defaults {
        conn.execute(
            "INSERT OR IGNORE INTO config (key, value, use_system_credential, updated_at) VALUES (?1, ?2, 0, ?3)",
            rusqlite::params![key, value, now],
        ).ok();
    }
}

pub fn with_db<F, T>(f: F) -> Result<T, crate::utils::error::CommandError>
where
    F: FnOnce(&Connection) -> Result<T, crate::utils::error::CommandError>,
{
    let guard = DB.lock().map_err(|e| crate::utils::error::CommandError::internal(&format!("数据库锁失败: {}", e)))?;
    f(&guard)
}

fn truncate_str(s: &str, max: usize) -> &str {
    if s.len() <= max { return s; }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) { end -= 1; }
    &s[..end]
}

pub fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
