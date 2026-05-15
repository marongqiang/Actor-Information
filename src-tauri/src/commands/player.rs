use crate::db::{self, queries};
use crate::utils::error::CommandError;
use serde::Serialize;

#[derive(Serialize)]
pub struct ProgressResponse {
    pub progress: i64,
    pub duration: i64,
    pub is_finished: i32,
}

#[tauri::command]
pub fn get_progress(file_id: String) -> Result<ProgressResponse, CommandError> {
    db::with_db(|conn| {
        let p = queries::get_progress(conn, &file_id)?;
        Ok(ProgressResponse {
            progress: p.progress,
            duration: p.duration,
            is_finished: p.is_finished,
        })
    })
}

#[tauri::command]
pub fn save_progress(
    file_id: String,
    progress: i64,
    duration: i64,
) -> Result<(), CommandError> {
    db::with_db(|conn| {
        queries::save_progress_row(conn, &file_id, progress, duration)
    })
}

#[tauri::command]
pub fn end_playback(file_id: String) -> Result<(), CommandError> {
    db::with_db(|conn| {
        queries::end_playback(conn, &file_id)
    })
}
