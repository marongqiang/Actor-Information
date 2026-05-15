use crate::db;
use crate::db::queries::TaskRow;
use crate::utils::error::{CommandError, CommandResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Create a new scan or scrape task
pub fn create_task(task_type: &str, target_ids: &[String]) -> CommandResult<String> {
    let task_id = Uuid::new_v4().to_string();
    let now = db::now_ts();
    let target_ids_json = serde_json::to_string(target_ids)
        .map_err(|e| CommandError::internal(&format!("序列化失败: {}", e)))?;

    let task = TaskRow {
        id: task_id.clone(),
        task_type: task_type.to_string(),
        target_ids: target_ids_json,
        status: "pending".to_string(),
        progress: 0,
        result: None,
        created_at: now,
        updated_at: now,
        error: None,
        checkpoint: None,
    };

    db::with_db(|conn| db::queries::insert_task(conn, &task))?;

    log::info!("创建任务: {} (类型: {})", task_id, task_type);
    Ok(task_id)
}

/// Update task progress
pub fn update_task_progress(task_id: &str, progress: i32) -> CommandResult<()> {
    let now = db::now_ts();
    db::with_db(|conn| {
        conn.execute(
            "UPDATE tasks SET progress = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![progress, now, task_id],
        )?;
        Ok(())
    })
}

/// Mark task as completed
pub fn complete_task(task_id: &str, result: Option<&str>) -> CommandResult<()> {
    let now = db::now_ts();
    db::with_db(|conn| {
        conn.execute(
            "UPDATE tasks SET status = 'completed', progress = 100, result = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![result, now, task_id],
        )?;
        Ok(())
    })
}

/// Mark task as failed
pub fn fail_task(task_id: &str, error: &str) -> CommandResult<()> {
    let now = db::now_ts();
    db::with_db(|conn| {
        conn.execute(
            "UPDATE tasks SET status = 'failed', error = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![error, now, task_id],
        )?;
        Ok(())
    })
}

/// Pause a running task
pub fn pause_task(task_id: &str) -> CommandResult<()> {
    let now = db::now_ts();
    db::with_db(|conn| {
        conn.execute(
            "UPDATE tasks SET status = 'paused', updated_at = ?1 WHERE id = ?2 AND status = 'running'",
            rusqlite::params![now, task_id],
        )?;
        Ok(())
    })
}

/// Resume a paused task
pub fn resume_task(task_id: &str) -> CommandResult<()> {
    let now = db::now_ts();
    db::with_db(|conn| {
        conn.execute(
            "UPDATE tasks SET status = 'running', updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, task_id],
        )?;
        Ok(())
    })
}

/// Cancel a task
pub fn cancel_task(task_id: &str) -> CommandResult<()> {
    let now = db::now_ts();
    db::with_db(|conn| {
        conn.execute(
            "UPDATE tasks SET status = 'failed', error = '用户取消', updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, task_id],
        )?;
        Ok(())
    })
}

/// Get all pending/running/paused tasks (for recovery)
pub fn get_pending_tasks() -> CommandResult<Vec<TaskRow>> {
    db::with_db(|conn| db::queries::get_pending_tasks(conn))
}
