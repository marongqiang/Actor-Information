use crate::db;
use crate::services::pan115;
use crate::utils::error::CommandResult;
use std::time::Duration;

/// Check and refresh play URLs that are about to expire.
/// Should be called periodically (e.g., every 4 minutes).
pub async fn refresh_expiring_urls() -> CommandResult<u64> {
    let now = db::now_ts();
    let threshold = now + 600; // URLs expiring within 10 minutes

    // Find URLs that need refreshing
    let expiring: Vec<(String, i64)> = db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT file_id, last_play_url_expire FROM movies
             WHERE last_play_url IS NOT NULL AND last_play_url_expire < ?1 AND last_play_url_expire > 0"
        )?;
        let rows = stmt.query_map([threshold], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?.filter_map(|r| r.ok()).collect();
        Ok(rows)
    })?;

    let count = expiring.len() as u64;
    for (file_id, _expire) in expiring {
        match pan115::refresh_play_url(&file_id).await {
            Ok(result) => {
                log::debug!("续期播放链接: {} -> 过期时间: {}", file_id, result.expire_at);
            }
            Err(e) => {
                log::warn!("续期失败 {}: {}", file_id, e);
            }
        }
        // Rate limit: small delay between requests
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    Ok(count)
}
