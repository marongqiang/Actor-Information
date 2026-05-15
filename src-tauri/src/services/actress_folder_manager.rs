use crate::db;
use crate::utils::error::{CommandError, CommandResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Get the configured local actor base directory
pub fn get_actor_base_dir() -> CommandResult<PathBuf> {
    let path = db::with_db(|conn| {
        crate::db::queries::get_config(conn, "local_actor_base_dir")
    })?
    .unwrap_or_else(|| r"D:\Media Library\Actor Information\picture".to_string());

    Ok(PathBuf::from(path))
}

// ─── Scan local folder for actors ───

#[derive(Serialize)]
pub struct ScanResult {
    pub added: i64,
    pub total: i64,
}

pub fn scan_local_actress_folder(folder_path: Option<String>) -> CommandResult<ScanResult> {
    let base_dir = match folder_path {
        Some(p) => PathBuf::from(p),
        None => get_actor_base_dir()?,
    };

    if !base_dir.exists() {
        return Err(CommandError::invalid_input(&format!(
            "演员文件夹不存在: {}",
            base_dir.display()
        )));
    }

    let mut total = 0i64;
    let mut added = 0i64;

    for entry in fs::read_dir(&base_dir)
        .map_err(|e| CommandError::internal(&format!("读取目录失败: {}", e)))?
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let folder_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        total += 1;

        // Check if an actress with this folder_name already exists
        let exists = db::with_db(|conn| {
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM av_actors WHERE local_folder_name = ?1 OR name = ?1",
                    [&folder_name],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            Ok(exists)
        })?;

        if !exists {
            // Get the first image file from the folder as avatar
            let avatar = find_first_image(&path);

            let now = db::now_ts();
            let letter = first_char_upper(&folder_name);

            db::with_db(|conn| {
                conn.execute(
                    "INSERT INTO av_actors (name, avatar_local, local_folder_name, is_pending, source, letter, created_at)
                     VALUES (?1, ?2, ?3, 0, 'local_folder', ?4, ?5)",
                    rusqlite::params![folder_name, avatar.as_deref(), folder_name, letter, now],
                )?;
                Ok(())
            })?;
            added += 1;
        }
    }

    Ok(ScanResult { added, total })
}

fn first_char_upper(name: &str) -> String {
    name.chars()
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_else(|| "#".to_string())
}

fn find_first_image(dir: &Path) -> Option<String> {
    let exts = ["jpg", "jpeg", "png", "webp", "gif", "bmp"];
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                if exts.contains(&ext_lower.as_str()) {
                    return Some(path.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

// ─── Get/Update local folder for an actress ───

pub fn get_actress_local_folder(actress_id: i64) -> CommandResult<Option<String>> {
    db::with_db(|conn| {
        let result = conn.query_row(
            "SELECT local_folder_name FROM av_actors WHERE id = ?1",
            [actress_id],
            |row| row.get::<_, Option<String>>(0),
        );
        match result {
            Ok(r) => Ok(r),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CommandError::db(&format!("查询失败: {}", e))),
        }
    })
}

pub fn update_actor_local_folder(actor_id: i64, folder_path: Option<String>) -> CommandResult<()> {
    db::with_db(|conn| {
        conn.execute(
            "UPDATE av_actors SET local_folder_name = ?1 WHERE id = ?2",
            rusqlite::params![folder_path, actor_id],
        )?;
        Ok(())
    })
}

// ─── Confirm/reject pending actor ───

pub fn confirm_actor(actor_id: i64, accepted: bool) -> CommandResult<()> {
    if accepted {
        db::with_db(|conn| {
            conn.execute(
                "UPDATE av_actors SET is_pending = 0 WHERE id = ?1",
                [actor_id],
            )?;
            Ok(())
        })
    } else {
        // Reject: delete the actor record
        db::with_db(|conn| {
            conn.execute("DELETE FROM av_actors WHERE id = ?1 AND is_pending = 1", [actor_id])?;
            Ok(())
        })
    }
}

// ─── Rename actor and optionally rename folder ───

#[derive(Serialize)]
pub struct RenameResult {
    pub success: bool,
    pub error: Option<String>,
}

pub fn rename_actor_and_folder(
    actor_id: i64,
    new_name: &str,
    rename_folder: bool,
) -> CommandResult<RenameResult> {
    // Check if the actor has a local folder
    let folder_name: Option<String> = db::with_db(|conn| {
        match conn.query_row(
            "SELECT local_folder_name FROM av_actors WHERE id = ?1",
            [actor_id],
            |row| row.get::<_, Option<String>>(0),
        ) {
            Ok(v) => Ok(v),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CommandError::db(&format!("查询失败: {}", e))),
        }
    })?;

    let base_dir = get_actor_base_dir()?;

    if rename_folder {
        if let Some(ref old_folder) = folder_name {
            let old_path = base_dir.join(old_folder);
            let new_path = base_dir.join(new_name);

            if old_path.exists() && !new_path.exists() {
                // Check if user has authorized folder rename
                let allow_rename = db::with_db(|conn| {
                    crate::db::queries::get_config(conn, "allow_app_rename_actor_folders")
                })?
                .unwrap_or_else(|| "0".to_string());

                if allow_rename == "1" {
                    fs::rename(&old_path, &new_path).map_err(|e| {
                        CommandError::internal(&format!("重命名文件夹失败: {}", e))
                    })?;
                }
            }

            // Update local_folder_name in DB
            db::with_db(|conn| {
                conn.execute(
                    "UPDATE av_actors SET name = ?1, local_folder_name = ?1 WHERE id = ?2",
                    rusqlite::params![new_name, actor_id],
                )?;
                Ok(())
            })?;
        } else {
            // Just rename the actress
            db::with_db(|conn| {
                conn.execute(
                    "UPDATE av_actors SET name = ?1 WHERE id = ?2",
                    rusqlite::params![new_name, actor_id],
                )?;
                Ok(())
            })?;
        }
    } else {
        db::with_db(|conn| {
            conn.execute(
                "UPDATE av_actors SET name = ?1 WHERE id = ?2",
                rusqlite::params![new_name, actor_id],
            )?;
            Ok(())
        })?;
    }

    Ok(RenameResult {
        success: true,
        error: None,
    })
}

// ─── Merge actresses with folder merging ───

#[derive(Serialize, Deserialize)]
pub struct MergeOptions {
    pub merge_folders: bool,
    #[serde(default = "default_conflict_policy")]
    pub conflict_policy: String,
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

fn default_conflict_policy() -> String {
    "rename".to_string()
}
fn default_dry_run() -> bool {
    true
}

#[derive(Serialize)]
pub struct MergeResult {
    pub success: bool,
    pub moved_files: Vec<String>,
    pub conflicts: Vec<String>,
    pub renamed_files: Vec<FileRename>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct FileRename {
    pub from: String,
    pub to: String,
}

pub fn merge_actresses(
    source_id: i64,
    target_id: i64,
    options: MergeOptions,
) -> CommandResult<MergeResult> {
    let mut moved = Vec::new();
    let mut conflicts = Vec::new();
    let mut renamed = Vec::new();

    // Get source and target folder names
    let source_folder: Option<String> = db::with_db(|conn| {
        match conn.query_row(
            "SELECT local_folder_name FROM av_actors WHERE id = ?1",
            [source_id],
            |row| row.get::<_, Option<String>>(0),
        ) {
            Ok(v) => Ok(v),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CommandError::db(&format!("查询源演员失败: {}", e))),
        }
    })?;

    let target_folder: Option<String> = db::with_db(|conn| {
        match conn.query_row(
            "SELECT local_folder_name FROM av_actors WHERE id = ?1",
            [target_id],
            |row| row.get::<_, Option<String>>(0),
        ) {
            Ok(v) => Ok(v),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CommandError::db(&format!("查询目标演员失败: {}", e))),
        }
    })?;

    // Merge folders if both exist and merge_folders is enabled
    if options.merge_folders {
        if let (Some(src_f), Some(tgt_f)) = (&source_folder, &target_folder) {
            let base_dir = get_actor_base_dir()?;
            let src_path = base_dir.join(src_f);
            let tgt_path = base_dir.join(tgt_f);

            if src_path.exists() && src_path.is_dir() {
                // Move files from source to target
                match move_folder_contents(&src_path, &tgt_path, &options.conflict_policy, &mut moved, &mut conflicts, &mut renamed) {
                    Ok(_) => {
                        // Remove empty source folder
                        if !options.dry_run {
                            let _ = fs::remove_dir(&src_path);
                        }
                    }
                    Err(e) => {
                        return Ok(MergeResult {
                            success: false,
                            moved_files: moved,
                            conflicts,
                            renamed_files: renamed,
                            error: Some(format!("文件夹合并失败: {}", e)),
                        });
                    }
                }
            }
        }
    }

    // Merge database records (aliases, group memberships, movie associations)
    if !options.dry_run {
        // Use existing merge function for DB records
        super::actress_sync::merge_actresses(source_id, target_id)?;
    }

    Ok(MergeResult {
        success: true,
        moved_files: moved,
        conflicts,
        renamed_files: renamed,
        error: None,
    })
}

fn move_folder_contents(
    src: &Path,
    dst: &Path,
    conflict_policy: &str,
    moved: &mut Vec<String>,
    conflicts: &mut Vec<String>,
    renamed: &mut Vec<FileRename>,
) -> Result<(), std::io::Error> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_file = entry.path();
        let file_name = src_file.file_name().unwrap_or_default().to_string_lossy();

        let dst_file = dst.join(file_name.as_ref());

        if dst_file.exists() {
            match conflict_policy {
                "overwrite" => {
                    fs::copy(&src_file, &dst_file)?;
                    moved.push(src_file.to_string_lossy().to_string());
                }
                "skip" => {
                    conflicts.push(src_file.to_string_lossy().to_string());
                    continue;
                }
                _ => {
                    // Rename: add suffix
                    let stem = src_file
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy();
                    let ext = src_file.extension().unwrap_or_default().to_string_lossy();
                    let new_name = format!("{}_1.{}", stem, ext);
                    let new_dst = dst.join(&new_name);

                    fs::copy(&src_file, &new_dst)?;
                    renamed.push(FileRename {
                        from: file_name.to_string(),
                        to: new_name,
                    });
                    moved.push(src_file.to_string_lossy().to_string());
                }
            }
        } else {
            fs::copy(&src_file, &dst_file)?;
            moved.push(src_file.to_string_lossy().to_string());
        }
    }

    Ok(())
}

// ─── Detect duplicate actresses ───

#[derive(Serialize)]
pub struct DuplicatePair {
    pub id1: i64,
    pub id2: i64,
    pub similarity: f64,
}

pub fn detect_duplicate_actresses(_threshold: Option<f64>) -> CommandResult<Vec<DuplicatePair>> {
    let threshold = _threshold.unwrap_or(0.8);
    let mut pairs = Vec::new();

    // Simple name-based similarity detection
    db::with_db(|conn| {
        let mut stmt = conn.prepare("SELECT id, name FROM av_actors ORDER BY name")?;
        let actors: Vec<(i64, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        for i in 0..actors.len() {
            for j in i + 1..actors.len() {
                let sim = name_similarity(&actors[i].1, &actors[j].1);
                if sim >= threshold {
                    pairs.push(DuplicatePair {
                        id1: actors[i].0,
                        id2: actors[j].0,
                        similarity: sim,
                    });
                }
            }
        }

        Ok(())
    })?;

    Ok(pairs)
}

fn name_similarity(a: &str, b: &str) -> f64 {
    let a = a.to_lowercase();
    let b = b.to_lowercase();

    if a == b {
        return 1.0;
    }

    // Simple Levenshtein-based similarity
    let dist = levenshtein_distance(&a, &b);
    let max_len = a.len().max(b.len()) as f64;
    if max_len == 0.0 {
        return 1.0;
    }
    1.0 - (dist as f64 / max_len)
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let len_a = a_chars.len();
    let len_b = b_chars.len();

    let mut dp = vec![vec![0usize; len_b + 1]; len_a + 1];
    for i in 0..=len_a { dp[i][0] = i; }
    for j in 0..=len_b { dp[0][j] = j; }

    for i in 1..=len_a {
        for j in 1..=len_b {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[len_a][len_b]
}

// ─── Refresh actress avatar from local folder ───

pub fn refresh_actress_avatar(actress_id: i64) -> CommandResult<()> {
    let folder_name: Option<String> = db::with_db(|conn| {
        match conn.query_row(
            "SELECT local_folder_name FROM av_actors WHERE id = ?1",
            [actress_id],
            |row| row.get::<_, Option<String>>(0),
        ) {
            Ok(v) => Ok(v),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CommandError::db(&format!("查询失败: {}", e))),
        }
    })?;

    if let Some(folder) = folder_name {
        let base_dir = get_actor_base_dir()?;
        let folder_path = base_dir.join(&folder);

        if folder_path.exists() {
            if let Some(img_path) = find_first_image(&folder_path) {
                db::with_db(|conn| {
                    conn.execute(
                        "UPDATE av_actors SET avatar_local = ?1 WHERE id = ?2",
                        rusqlite::params![img_path, actress_id],
                    )?;
                    Ok(())
                })?;
            }
        }
    }
    Ok(())
}

// ─── Sync actress with local folder ───

pub fn sync_actress_with_local_folder(actress_id: i64) -> CommandResult<()> {
    // Update avatar and folder name from local folder
    refresh_actress_avatar(actress_id)?;

    let folder_name: Option<String> = db::with_db(|conn| {
        match conn.query_row(
            "SELECT local_folder_name FROM av_actors WHERE id = ?1",
            [actress_id],
            |row| row.get::<_, Option<String>>(0),
        ) {
            Ok(v) => Ok(v),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CommandError::db(&format!("查询失败: {}", e))),
        }
    })?;

    if let Some(folder) = folder_name {
        let base_dir = get_actor_base_dir()?;
        let folder_path = base_dir.join(&folder);

        if folder_path.exists() {
            let file_count = fs::read_dir(&folder_path)
                .map(|entries| entries.filter(|e| e.as_ref().map(|e| e.path().is_file()).unwrap_or(false)).count())
                .unwrap_or(0);

            log::info!(
                "同步演员 {} (id={}) 的本地文件夹: {} 个文件",
                folder,
                actress_id,
                file_count
            );
        }
    }
    Ok(())
}


