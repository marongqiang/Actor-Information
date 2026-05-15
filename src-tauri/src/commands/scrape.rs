use crate::services::{scrape_manager, task_manager};
use crate::utils::error::CommandResult;

#[tauri::command]
pub async fn start_scrape(file_ids: Vec<String>) -> Result<String, crate::utils::error::CommandError> {
    let task_id = task_manager::create_task("scrape", &file_ids)?;
    task_manager::resume_task(&task_id)?;
    log::info!("开始刮削任务: {}, 共 {} 个文件", task_id, file_ids.len());
    Ok(task_id)
}

#[tauri::command]
pub fn pause_scrape(task_id: String) -> Result<(), crate::utils::error::CommandError> {
    task_manager::pause_task(&task_id)
}

#[tauri::command]
pub fn resume_scrape(task_id: String) -> Result<(), crate::utils::error::CommandError> {
    task_manager::resume_task(&task_id)
}

#[tauri::command]
pub async fn manual_scrape(
    file_id: String,
    keyword: String,
) -> Result<Vec<scrape_manager::ScrapeResult>, crate::utils::error::CommandError> {
    // Use default sources for manual scrape
    let sources = vec!["tmdb".to_string(), "douban".to_string(), "javbus".to_string()];
    // For manual scrape, use keyword as the search query
    scrape_manager::scrape_file(&file_id, &sources).await
}

#[tauri::command]
pub async fn select_scrape_result(
    file_id: String,
    result_idx: usize,
) -> Result<(), crate::utils::error::CommandError> {
    // Re-scrape and select the Nth result
    let sources = vec!["tmdb".to_string(), "douban".to_string(), "javbus".to_string()];
    let results = scrape_manager::scrape_file(&file_id, &sources).await?;

    if let Some(result) = results.get(result_idx) {
        scrape_manager::apply_scrape_result(&file_id, result).await?;
        Ok(())
    } else {
        Err(crate::utils::error::CommandError::invalid_input("无效的结果索引"))
    }
}

#[derive(serde::Serialize)]
pub struct TestSourceResult {
    pub status: u16,
    pub time_ms: u64,
}

#[tauri::command]
pub async fn test_source(url: String) -> Result<TestSourceResult, crate::utils::error::CommandError> {
    let start = std::time::Instant::now();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| crate::utils::error::CommandError::network(&format!("创建客户端失败: {}", e)))?;

    let resp = client.get(&url).send().await?;
    let time_ms = start.elapsed().as_millis() as u64;

    Ok(TestSourceResult {
        status: resp.status().as_u16(),
        time_ms,
    })
}
