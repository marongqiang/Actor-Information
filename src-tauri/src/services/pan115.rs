use crate::utils::error::CommandError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

static CLIENT: once_cell::sync::Lazy<Client> = once_cell::sync::Lazy::new(|| {
    Client::builder()
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(30))
        .danger_accept_invalid_certs(false)
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .expect("Failed to build HTTP client")
});

static COOKIE: once_cell::sync::Lazy<Mutex<Option<String>>> = once_cell::sync::Lazy::new(|| Mutex::new(None));
static EMPTY_ARR: once_cell::sync::Lazy<Vec<serde_json::Value>> = once_cell::sync::Lazy::new(Vec::new);

// 115 API endpoints
const QRCODE_API: &str = "https://qrcodeapi.115.com/api/1.0/web/1.0/qrcode";
const QRCODE_STATUS_API: &str = "https://qrcodeapi.115.com/getstatus";
const WEBAPI_BASE: &str = "https://webapi.115.com";
const PROAPI_BASE: &str = "https://proapi.115.com";

pub fn set_cookie(cookie: &str) {
    let mut c = COOKIE.lock().unwrap();
    *c = Some(cookie.to_string());
}

pub fn get_cookie() -> Option<String> {
    COOKIE.lock().unwrap().clone()
}

pub fn clear_cookie() {
    let mut c = COOKIE.lock().unwrap();
    *c = None;
}

pub fn has_cookie() -> bool {
    COOKIE.lock().unwrap().is_some()
}

// ─── QR Code Login ───

#[derive(Serialize)]
pub struct QrCodeResult {
    pub qrcode_url: String,
    pub uid: String,
}

pub async fn login_qrcode() -> Result<QrCodeResult, CommandError> {
    // Use the 115 QR code API to get a login QR code
    let resp = CLIENT
        .get(QRCODE_API)
        .send()
        .await
        .map_err(|e| CommandError::network(&format!("请求二维码失败: {}", e)))?;

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();

    log::info!("QR code API response status: {}, body_preview: {}", status, &body_text[..body_text.len().min(200)]);

    if !status.is_success() {
        return Err(CommandError::network(&format!(
            "获取二维码HTTP错误: {}",
            status
        )));
    }

    // Parse the response
    let json: serde_json::Value = serde_json::from_str(&body_text)
        .map_err(|e| CommandError::network(&format!("解析二维码响应失败: {}，原始响应: {}", e, &body_text[..body_text.len().min(100)])))?;

    // Check for error code
    if let Some(code) = json["code"].as_i64() {
        if code != 0 {
            let msg = json["message"].as_str().unwrap_or("未知错误");
            return Err(CommandError::network(&format!("获取二维码失败: {}", msg)));
        }
    }

    // Extract QR code data
    let data = &json["data"];
    let uid_val = data["uid"].as_str()
        .or_else(|| data["qrcode"].as_str())
        .unwrap_or("");

    if uid_val.is_empty() {
        return Err(CommandError::network(&format!(
            "未获取到二维码UID，响应: {}",
            serde_json::to_string_pretty(&json).unwrap_or_default()
        )));
    }

    // The QR code image URL
    let qrcode_url = data["qrcode"].as_str()
        .or_else(|| data["qrcode_url"].as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}/qrcode?uid={}", QRCODE_STATUS_API, uid_val));

    Ok(QrCodeResult {
        qrcode_url,
        uid: uid_val.to_string(),
    })
}

#[derive(Serialize)]
pub struct LoginStatusResult {
    pub status: String,
    pub cookie: Option<String>,
}

pub async fn login_status(uid: &str) -> Result<LoginStatusResult, CommandError> {
    // Poll the QR code status
    let url = format!("{}?uid={}", QRCODE_STATUS_API, uid);

    let resp = CLIENT
        .get(&url)
        .header("Referer", "https://115.com/")
        .send()
        .await
        .map_err(|e| CommandError::network(&format!("查询登录状态失败: {}", e)))?;

    let body_text = resp.text().await.unwrap_or_default();

    let json: serde_json::Value = serde_json::from_str(&body_text)
        .map_err(|e| CommandError::network(&format!("解析登录状态响应失败: {}", e)))?;

    // The status code from 115 API
    let status_code: i64;
    let mut cookie: Option<String> = None;

    if let Some(code) = json["code"].as_i64() {
        status_code = code;
    } else if let Some(s) = json["status"].as_i64() {
        status_code = s;
    } else {
        status_code = json["data"]["status"].as_i64().unwrap_or(0);
    }

    // Map status codes
    // -1: waiting, 0: scanned/confirmed, 1: logged in, 2: expired/cancelled
    let status = match status_code {
        -1 | 0 => "waiting",
        1 => "scanned",
        2 => "authorized",
        -2 | 4 => "expired",
        _ => "waiting",
    };

    // When authorized, get the cookie from response
    if status == "authorized" {
        // The cookie might be in the response JSON
        cookie = json["data"]["cookie"].as_str().map(|s| s.to_string());

        // If not in JSON body, check if cookies were set automatically
        if cookie.is_none() {
            // For 115, after scanning, we need to do a separate request to get the full cookie
            // The scanning confirmation sets cookies in the cookie store
            log::info!("扫码已确认，尝试获取完整cookie");
        }
    }

    Ok(LoginStatusResult {
        status: status.to_string(),
        cookie,
    })
}

// ─── Cookie-Based Login (alternative method) ───

pub async fn login_with_cookie(cookie_string: String) -> Result<(), CommandError> {
    // Validate cookie by making a test request
    let test_url = format!("{}/files/list?limit=1&offset=0&cid=0", WEBAPI_BASE);
    let resp = CLIENT
        .get(&test_url)
        .header("Cookie", &cookie_string)
        .send()
        .await
        .map_err(|e| CommandError::network(&format!("验证cookie失败: {}", e)))?;

    let json: serde_json::Value = resp.json().await
        .map_err(|_| CommandError::unauthorized("Cookie无效或已过期"))?;

    if json["code"].as_i64() != Some(0) {
        return Err(CommandError::unauthorized("Cookie验证失败"));
    }

    set_cookie(&cookie_string);
    log::info!("通过Cookie直接登录成功");
    Ok(())
}

// ─── File Operations ───

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileInfo {
    pub cid: String,
    pub name: String,
    pub is_dir: bool,
    pub size: i64,
    pub update_time: i64,
    pub file_id: Option<String>,
}

pub async fn list_root() -> Result<Vec<FileInfo>, CommandError> {
    let cookie = get_cookie().ok_or_else(|| CommandError::unauthorized("未登录115网盘"))?;

    let resp = CLIENT
        .get(&format!("{}/files/list?limit=50&offset=0&cid=0", WEBAPI_BASE))
        .header("Cookie", &cookie)
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await
        .map_err(|e| CommandError::network(&format!("解析目录列表失败: {}", e)))?;

    if json["code"].as_i64() != Some(0) && json["state"].as_i64() != Some(0) && json["state"].as_bool() != Some(true) {
        let msg = json["message"].as_str().unwrap_or("未知错误");
        return Err(CommandError::network(&format!("获取目录列表失败: {}", msg)));
    }

    let data_array = json["data"]["data"].as_array()
        .or_else(|| json["data"].as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&*EMPTY_ARR);

    let items = data_array.iter()
        .map(|item| FileInfo {
            cid: item["cid"].as_str().unwrap_or("").to_string(),
            name: item["n"].as_str()
                .or_else(|| item["name"].as_str())
                .unwrap_or("未知")
                .to_string(),
            is_dir: item["fid"].as_i64().unwrap_or(0) == 0 || item["fid"].is_null(),
            size: item["s"].as_i64()
                .or_else(|| item["size"].as_i64())
                .unwrap_or(0),
            update_time: item["t"].as_i64()
                .or_else(|| item["update_time"].as_i64())
                .unwrap_or(0),
            file_id: item["fid"].as_str().map(|s| s.to_string())
                .or_else(|| item["file_id"].as_str().map(|s| s.to_string())),
        })
        .collect();

    Ok(items)
}

pub async fn get_files(cid: &str, page: i64, page_size: i64) -> Result<(Vec<FileInfo>, i64), CommandError> {
    let cookie = get_cookie().ok_or_else(|| CommandError::unauthorized("未登录115网盘"))?;
    let offset = (page - 1) * page_size;
    let url = format!(
        "{}/files/list?limit={}&offset={}&cid={}",
        WEBAPI_BASE, page_size, offset, cid
    );

    let resp = CLIENT
        .get(&url)
        .header("Cookie", &cookie)
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await
        .map_err(|e| CommandError::network(&format!("解析文件列表失败: {}", e)))?;

    if json["code"].as_i64() != Some(0) && json["state"].as_bool() != Some(true) {
        let msg = json["message"].as_str().unwrap_or("未知错误");
        return Err(CommandError::network(&format!("获取文件列表失败: {}", msg)));
    }

    let total = json["data"]["count"].as_i64()
        .or_else(|| json["count"].as_i64())
        .unwrap_or(0);

    let data_array = json["data"]["data"].as_array()
        .or_else(|| json["data"].as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&*EMPTY_ARR);

    let items = data_array.iter()
        .map(|item| FileInfo {
            cid: item["cid"].as_str().unwrap_or("").to_string(),
            name: item["n"].as_str()
                .or_else(|| item["name"].as_str())
                .unwrap_or("未知")
                .to_string(),
            is_dir: item["fid"].as_i64().unwrap_or(0) == 0 || item["fid"].is_null(),
            size: item["s"].as_i64()
                .or_else(|| item["size"].as_i64())
                .unwrap_or(0),
            update_time: item["t"].as_i64()
                .or_else(|| item["update_time"].as_i64())
                .unwrap_or(0),
            file_id: item["fid"].as_str().map(|s| s.to_string())
                .or_else(|| item["file_id"].as_str().map(|s| s.to_string())),
        })
        .collect();

    Ok((items, total))
}

#[derive(Serialize)]
pub struct PlayUrlResult {
    pub url: String,
    pub expire_at: i64,
}

pub async fn get_play_url(file_id: &str) -> Result<PlayUrlResult, CommandError> {
    let cookie = get_cookie().ok_or_else(|| CommandError::unauthorized("未登录115网盘"))?;

    // First get the download info
    let info_url = format!("{}/files/download?fid={}", PROAPI_BASE, file_id);
    let resp = CLIENT
        .get(&info_url)
        .header("Cookie", &cookie)
        .send()
        .await?;

    let json: serde_json::Value = resp.json().await
        .map_err(|e| CommandError::network(&format!("解析播放链接失败: {}", e)))?;

    if json["code"].as_i64() != Some(0) && json["state"].as_bool() != Some(true) {
        let msg = json["message"].as_str().unwrap_or("未知错误");
        return Err(CommandError::network(&format!("获取播放链接失败: {}", msg)));
    }

    let play_url = json["data"]["url"].as_str()
        .or_else(|| json["data"]["play_url"].as_str())
        .unwrap_or("")
        .to_string();

    if play_url.is_empty() {
        return Err(CommandError::network("未获取到播放链接"));
    }

    // Links typically expire in 1 hour (3600 seconds)
    let expire_at = crate::db::now_ts() + 3600;

    // Cache the URL in the database
    crate::db::with_db(|conn| {
        conn.execute(
            "UPDATE movies SET last_play_url = ?1, last_play_url_expire = ?2 WHERE file_id = ?3",
            rusqlite::params![play_url, expire_at, file_id],
        )?;
        Ok(())
    })?;

    Ok(PlayUrlResult { url: play_url, expire_at })
}

pub async fn refresh_play_url(file_id: &str) -> Result<PlayUrlResult, CommandError> {
    get_play_url(file_id).await
}
