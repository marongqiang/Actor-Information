use crate::db::{self, now_ts};
use crate::services::pan115::{self, FileInfo};
use crate::utils::error::{CommandError, CommandResult};
use serde::Serialize;
use std::collections::HashSet;
use std::time::{Duration, Instant};

#[derive(Serialize)]
pub struct ScanResult {
    pub total: i64,
    pub new: i64,
    pub updated: i64,
    pub deleted: i64,
}

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "rmvb", "flv", "wmv", "ts", "iso", "m2ts",
];

// 限流：每秒1次请求，每10次额外冷却3秒，防止115返回405
static LAST_REQUEST: once_cell::sync::Lazy<tokio::sync::Mutex<Instant>> = once_cell::sync::Lazy::new(|| tokio::sync::Mutex::new(Instant::now()));
static REQUEST_COUNT: once_cell::sync::Lazy<tokio::sync::Mutex<u32>> = once_cell::sync::Lazy::new(|| tokio::sync::Mutex::new(0));
const MIN_INTERVAL: Duration = Duration::from_millis(1000);  // 1 request/sec

async fn rate_limit() {
    let mut last = LAST_REQUEST.lock().await;
    let elapsed = last.elapsed();
    if elapsed < MIN_INTERVAL {
        tokio::time::sleep(MIN_INTERVAL - elapsed).await;
    }
    *last = Instant::now();
    // Every 10 requests, add 3s cooldown
    let mut count = REQUEST_COUNT.lock().await;
    *count += 1;
    if *count % 10 == 0 {
        drop(count);
        drop(last);
        log::info!("扫描限流: 已请求{}次，冷却3秒...", *REQUEST_COUNT.lock().await);
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

pub async fn scan_directory(
    cid: &str,
    depth: i32,
    mode: &str,
) -> Result<ScanResult, CommandError> {
    let mut all_files: Vec<FileInfo> = Vec::new();
    let mut scanned = HashSet::new();
    collect_video_files(cid, depth, 0, &mut all_files, &mut scanned).await?;

    log::info!("扫描完成: {} 个目录中共发现 {} 个视频文件", cid, all_files.len());
    let total = all_files.len() as i64;
    let mut new_count = 0i64;
    let mut updated_count = 0i64;

    for file in &all_files {
        let file_id = match &file.file_id {
            Some(id) => id.clone(),
            None => continue,
        };

        let exists = db::with_db(|conn| {
            let exists: bool = conn
                .query_row("SELECT COUNT(*) > 0 FROM movies WHERE file_id = ?1", [&file_id], |row| row.get(0))
                .unwrap_or(false);
            Ok(exists)
        })?;

        let now = now_ts();

        if !exists {
            if mode == "full" || mode == "incremental" {
                let title = file.name.rsplit('.').next()
                    .map(|ext| file.name[..file.name.len() - ext.len() - 1].to_string())
                    .unwrap_or_else(|| file.name.clone());

                let m = db::queries::InsertMovie {
                    file_id: file_id.clone(),
                    title: title.clone(),
                    original_title: None,
                    year: None,
                    poster_url: None,
                    poster_local: None,
                    backdrop_url: None,
                    overview: None,
                    rating: None,
                    runtime: None,
                    director: None,
                    genre: None,
                    file_name: file.name.clone(),
                    file_size: Some(file.size),
                    created_at: now,
                    updated_at: now,
                    is_hidden: false,
                };

                db::with_db(|conn| db::queries::insert_movie(conn, &m))?;
                new_count += 1;
            }
        } else {
            // Check if file was updated
            db::with_db(|conn| {
                conn.execute(
                    "UPDATE movies SET file_size = ?1, updated_at = ?2 WHERE file_id = ?3 AND file_size != ?1",
                    rusqlite::params![file.size, now, file_id],
                )?;
                if conn.changes() > 0 {
                    Ok(()) as CommandResult<()>
                } else {
                    Ok(())
                }
            })?;
            // We'll just count all existing as updated for simplicity in incremental mode
            if mode == "full" {
                updated_count += 1;
            }
        }
    }

    // For "full" mode, mark deleted files (files not in the current listing)
    let deleted = if mode == "full" {
        let current_ids: Vec<String> = all_files.iter()
            .filter_map(|f| f.file_id.clone())
            .collect();

        db::with_db(|conn| {
            let id_list = current_ids.iter()
                .map(|id| format!("'{}'", id.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(",");

            if !id_list.is_empty() {
                conn.execute(
                    &format!("UPDATE movies SET is_hidden = 1 WHERE file_id NOT IN ({})", id_list),
                    [],
                )?;
                Ok(conn.changes() as i64)
            } else {
                Ok(0i64)
            }
        })?
    } else {
        0
    };

    Ok(ScanResult {
        total,
        new: new_count,
        updated: updated_count,
        deleted,
    })
}

async fn collect_video_files(
    cid: &str,
    max_depth: i32,
    current_depth: i32,
    files: &mut Vec<FileInfo>,
    scanned: &mut HashSet<String>,
) -> Result<(), CommandError> {
    if current_depth > max_depth || !scanned.insert(cid.to_string()) {
        return Ok(());
    }

    rate_limit().await;
    let (items, reported_total) = match pan115::get_files(cid, 1, 200).await {
        Ok(r) => {
            log::info!("扫描目录 cid={}: {} 个项目, API报告总计={}", cid, r.0.len(), r.1);
            r
        }
        Err(e) => {
            log::warn!("跳过无法访问的目录 cid={}: {}", cid, e);
            return Ok(());
        }
    };

    // Don't trust API's total count - stop when items < page_size
    let total_pages = if items.len() < 200 { 1 } else { ((reported_total as f64 / 200.0).ceil() as i64).min(50) };
    log::debug!("cid={} 实际分页数={}", cid, total_pages);

    for page in 1..=total_pages {
        let page_items = if page == 1 {
            items.clone()
        } else {
            rate_limit().await;
            match pan115::get_files(cid, page, 200).await {
                Ok((items, _)) => {
                    if items.is_empty() { break; }
                    items
                }
                Err(e) => {
                    log::warn!("跳过目录分页 cid={} page={}: {}", cid, page, e);
                    continue;
                }
            }
        };

        for item in page_items {
            if item.is_dir {
                // Skip dirs with empty cid (invalid)
                if item.cid.is_empty() || item.cid == "0" {
                    continue;
                }
                if let Err(e) = Box::pin(collect_video_files(&item.cid, max_depth, current_depth + 1, files, scanned)).await {
                    log::warn!("跳过子目录 {}: {}", item.name, e);
                }
            } else if is_video_file(&item.name) {
                files.push(item);
            }
        }
    }

    Ok(())
}

fn is_video_file(name: &str) -> bool {
    if let Some(ext) = name.rsplit('.').next() {
        VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str())
    } else {
        false
    }
}
