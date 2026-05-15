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
