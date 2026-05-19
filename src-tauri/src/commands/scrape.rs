use crate::services::scrape_manager;
use crate::utils::error::{CommandError, CommandResult};
use serde::Serialize;
use std::sync::OnceLock;
use tauri::Emitter;
static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

pub fn set_app_handle(handle: tauri::AppHandle) {
    let _ = APP_HANDLE.set(handle);
}

#[derive(Serialize)]
pub struct BatchScrapeResult {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
}

#[derive(Clone, serde::Serialize)]
pub struct ScrapeProgress {
    pub current: usize,
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub file_name: String,
}

#[tauri::command]
pub async fn scrape_batch(file_ids: Vec<String>) -> Result<BatchScrapeResult, crate::utils::error::CommandError> {
    log::info!("scrape_batch 被调用: {} 个文件", file_ids.len());
    let sources: Vec<String> = crate::db::with_db(|conn| crate::db::queries::get_config(conn, "scrape_sources"))
        .ok().flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| vec!["metatube".into(), "javbus".into()]);

    let total = file_ids.len();
    let app_handle = APP_HANDLE.get().cloned();

    // Mark all as scraping + set scrape_log pending
    for fid in &file_ids {
        let fid2 = fid.clone();
        crate::db::with_db(move |conn| {
            conn.execute("UPDATE movies SET scrape_status=1 WHERE file_id=?1", [fid2.as_str()])?;
            crate::db::queries::set_scrape_pending(conn, &fid2)?;
            Ok(())
        }).ok();
    }

    let result = tokio::task::spawn_blocking(move || {
        let mut success = 0usize;
        let mut failed = 0usize;
        for (i, fid) in file_ids.iter().enumerate() {
            let file_name = crate::db::with_db(|conn| {
                conn.query_row("SELECT file_name FROM movies WHERE file_id=?1", [fid.as_str()], |r| r.get::<_,String>(0))
                    .map_err(|e| CommandError::db(&e.to_string()))
            }).unwrap_or_default();

            log::info!("[{}/{}] 刮削 {} ...", i+1, total, file_name);

            let (ok, err_msg) = match scrape_manager::scrape_file(fid, &sources) {
                Ok(results) if !results.is_empty() => {
                    match scrape_manager::apply_scrape_result(fid, &results[0]) {
                        Ok(_) => (true, None),
                        Err(e) => (false, Some(format!("{}", e))),
                    }
                }
                Err(e) => (false, Some(format!("{}", e))),
                _ => (false, Some("无结果".into())),
            };
            if ok { success += 1; } else { failed += 1; }
            let fid2 = fid.clone();
            let err2 = err_msg.clone();
            crate::db::with_db(move |c| {
                let st: i32 = if ok { 2 } else { 3 };
                c.execute("UPDATE movies SET scrape_status=?1 WHERE file_id=?2", rusqlite::params![st, fid2.as_str()])?;
                crate::db::queries::set_scrape_done(c, &fid2, ok, err2.as_deref())?;
                Ok(())
            }).ok();

            if let Some(ref handle) = app_handle {
                let _ = handle.emit("scrape-progress", ScrapeProgress {
                    current: i + 1, total, success, failed,
                    file_name: file_name.clone(),
                });
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        (success, failed)
    }).await.map_err(|e| crate::utils::error::CommandError::internal(&e.to_string()))?;

    let (success, failed) = result;
    log::info!("批量刮削完成: {}/{} 成功", success, total);
    Ok(BatchScrapeResult { total, success, failed })
}

#[tauri::command]
pub async fn start_scrape(_file_ids: Vec<String>) -> Result<String, crate::utils::error::CommandError> {
    Ok("scrape_task_0".into())
}
#[tauri::command] pub fn pause_scrape(_task_id: String) -> Result<(), crate::utils::error::CommandError> { Ok(()) }
#[tauri::command] pub fn resume_scrape(_task_id: String) -> Result<(), crate::utils::error::CommandError> { Ok(()) }
#[tauri::command] pub async fn manual_scrape(file_id: String, _keyword: String) -> Result<Vec<scrape_manager::ScrapeResult>, crate::utils::error::CommandError> {
    scrape_manager::scrape_file(&file_id, &["tmdb".into(), "javbus".into()])
}
#[tauri::command] pub async fn select_scrape_result(file_id: String, result_idx: usize) -> Result<(), crate::utils::error::CommandError> {
    let results = scrape_manager::scrape_file(&file_id, &["tmdb".into(), "javbus".into()])?;
    if let Some(r) = results.get(result_idx) { scrape_manager::apply_scrape_result(&file_id, r)?; Ok(()) }
    else { Err(crate::utils::error::CommandError::invalid_input("无效")) }
}
#[tauri::command] pub async fn test_source(url: String) -> Result<TestSourceResult, crate::utils::error::CommandError> {
    let start = std::time::Instant::now();
    let c = reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build().map_err(|e| crate::utils::error::CommandError::network(&e.to_string()))?;
    let resp = c.get(&url).send().await?;
    Ok(TestSourceResult { status: resp.status().as_u16(), time_ms: start.elapsed().as_millis() as u64 })
}
#[derive(Serialize)] pub struct TestSourceResult { pub status: u16, pub time_ms: u64 }
