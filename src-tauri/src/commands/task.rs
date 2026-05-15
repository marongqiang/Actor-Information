use crate::services::task_manager;
use crate::utils::error::CommandResult;
use serde::Serialize;

#[derive(Serialize)]
pub struct TaskResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub task_type: String,
    pub status: String,
    pub progress: i32,
    pub result: Option<String>,
    pub error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[tauri::command]
pub fn get_pending_tasks() -> Result<Vec<TaskResponse>, crate::utils::error::CommandError> {
    let tasks = task_manager::get_pending_tasks()?;
    let response = tasks.into_iter().map(|t| TaskResponse {
        id: t.id,
        task_type: t.task_type,
        status: t.status,
        progress: t.progress,
        result: t.result,
        error: t.error,
        created_at: t.created_at,
        updated_at: t.updated_at,
    }).collect();
    Ok(response)
}

#[tauri::command]
pub fn resume_task(task_id: String) -> Result<(), crate::utils::error::CommandError> {
    task_manager::resume_task(&task_id)
}

#[tauri::command]
pub fn cancel_task(task_id: String) -> Result<(), crate::utils::error::CommandError> {
    task_manager::cancel_task(&task_id)
}
