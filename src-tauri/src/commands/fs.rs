use crate::services::{pan115, scanner};
use crate::utils::error::CommandResult;
use serde::Serialize;

#[derive(Serialize)]
pub struct RootItem {
    pub cid: String,
    pub name: String,
}

#[tauri::command]
pub async fn list_root() -> Result<Vec<RootItem>, crate::utils::error::CommandError> {
    let items = pan115::list_root().await?;
    let roots = items.into_iter()
        .filter(|f| f.is_dir)
        .map(|f| RootItem { cid: f.cid, name: f.name })
        .collect();
    Ok(roots)
}

#[derive(Serialize)]
pub struct FileItemResponse {
    pub files: Vec<pan115::FileInfo>,
    pub total: i64,
}

#[tauri::command]
pub async fn get_files(
    path: String,
    page: i64,
    page_size: i64,
) -> Result<FileItemResponse, crate::utils::error::CommandError> {
    let (files, total) = pan115::get_files(&path, page, page_size).await?;
    Ok(FileItemResponse { files, total })
}

#[tauri::command]
pub async fn scan_directory(
    path: String,
    depth: i32,
    mode: String,
) -> Result<scanner::ScanResult, crate::utils::error::CommandError> {
    scanner::scan_directory(&path, depth, &mode).await
}

#[tauri::command]
pub async fn get_play_url(
    file_id: String,
) -> Result<pan115::PlayUrlResult, crate::utils::error::CommandError> {
    pan115::get_play_url(&file_id).await
}

#[tauri::command]
pub async fn refresh_play_url(
    file_id: String,
) -> Result<pan115::PlayUrlResult, crate::utils::error::CommandError> {
    pan115::refresh_play_url(&file_id).await
}

#[tauri::command]
pub async fn export_list() -> Result<String, crate::utils::error::CommandError> {
    // Export movie list as JSON
    let movies: Vec<serde_json::Value> = crate::db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT file_id, title, year, file_name, file_size FROM movies WHERE is_hidden = 0 ORDER BY title"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "file_id": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "year": row.get::<_, Option<i32>>(2)?,
                "file_name": row.get::<_, String>(3)?,
                "file_size": row.get::<_, Option<i64>>(4)?,
            }))
        })?.filter_map(|r| r.ok()).collect::<Vec<_>>();
        Ok(rows)
    })?;

    let export_dir = crate::db::get_data_dir().join("exports");
    std::fs::create_dir_all(&export_dir).ok();
    let filename = format!("movies_{}.json", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let path = export_dir.join(&filename);
    let json = serde_json::to_string_pretty(&movies)
        .map_err(|e| crate::utils::error::CommandError::internal(&format!("序列化失败: {}", e)))?;
    std::fs::write(&path, &json)
        .map_err(|e| crate::utils::error::CommandError::internal(&format!("写入文件失败: {}", e)))?;

    Ok(path.to_string_lossy().to_string())
}
