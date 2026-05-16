use crate::db::{self, queries};
use crate::services::secure_config;
use crate::utils::error::CommandResult;
use std::collections::HashMap;

#[tauri::command]
pub fn get_config(key: String) -> Result<Option<String>, crate::utils::error::CommandError> {
    db::with_db(|conn| queries::get_config(conn, &key))
}

#[tauri::command]
pub fn set_config(key: String, value: String) -> Result<(), crate::utils::error::CommandError> {
    db::with_db(|conn| queries::set_config(conn, &key, &value))
}

#[tauri::command]
pub fn get_all_config() -> Result<HashMap<String, String>, crate::utils::error::CommandError> {
    db::with_db(|conn| {
        let rows = queries::get_all_config(conn)?;
        let mut map = HashMap::new();
        for (k, v) in rows {
            map.insert(k, v);
        }
        Ok(map)
    })
}

#[tauri::command]
pub fn set_secure_config(key: String, value: String) -> Result<(), crate::utils::error::CommandError> {
    secure_config::set_secure_config(&key, &value)
}

#[tauri::command]
pub fn get_secure_config(key: String) -> Result<Option<String>, crate::utils::error::CommandError> {
    secure_config::get_secure_config(&key)
}

#[tauri::command]
pub fn clear_secure_config(key: String) -> Result<(), crate::utils::error::CommandError> {
    secure_config::clear_secure_config(&key)
}

/// Read a local image file and return as base64 data URL
#[tauri::command]
pub fn read_image_base64(path: String) -> Result<String, crate::utils::error::CommandError> {
    let bytes = std::fs::read(&path)
        .map_err(|e| crate::utils::error::CommandError::internal(&format!("读取图片失败: {}", e)))?;
    let ext = std::path::Path::new(&path).extension()
        .and_then(|e| e.to_str()).unwrap_or("jpg");
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
    Ok(format!("data:image/{};base64,{}", ext, b64))
}

/// List all image files in a directory
#[tauri::command]
pub fn list_folder_images(dir_path: String) -> Result<Vec<String>, crate::utils::error::CommandError> {
    let exts = ["jpg", "jpeg", "png", "webp", "gif", "bmp"];
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if exts.contains(&ext.to_lowercase().as_str()) {
                        files.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    // Sort by filename
    files.sort();
    Ok(files)
}
