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
const QRCODE_STATUS_API: &str = "https://qrcodeapi.115.com/api/1.0/web/1.0/qrcode/status";
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
    pub qrcode_url: String,      // base64 data URL for the QR image
    pub uid: String,             // unique ID for polling status
}

pub async fn login_qrcode() -> Result<QrCodeResult, CommandError> {
    // Generate a UUID for this login session
    let uid = uuid::Uuid::new_v4().to_string();

    // 115 QR code API returns the QR code PNG image directly
    // We pass the uid as a query parameter
    let url = format!("{}?uid={}", QRCODE_API, uid);

    let resp = CLIENT
        .get(&url)
        .send()
        .await
        .map_err(|e| CommandError::network(&format!("请求二维码失败: {}", e)))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(CommandError::network(&format!(
            "获取二维码HTTP错误: {}",
            status
        )));
    }

    // Read response as bytes (it's a PNG image)
    let bytes = resp.bytes().await
        .map_err(|e| CommandError::network(&format!("读取二维码图片失败: {}", e)))?;

    // Check if it's actually an image (PNG magic bytes: 89 50 4E 47)
    if bytes.len() < 4 || &bytes[..4] != b"\x89PNG" {
        // If it's not a PNG, try to parse as text/JSON for error message
        let text = String::from_utf8_lossy(&bytes);
        log::warn!("二维码API返回非图片数据: {}", &text[..text.len().min(200)]);
        return Err(CommandError::network(&format!(
            "二维码API返回异常: {}", &text[..text.len().min(100)]
        )));
    }

    // Convert PNG to base64 data URL
    let base64_img = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
    let qrcode_url = format!("data:image/png;base64,{}", base64_img);

    log::info!("QR码获取成功, uid={}, 图片大小={}bytes", uid, bytes.len());

    Ok(QrCodeResult {
        qrcode_url,
        uid,
    })
}

#[derive(Serialize)]
pub struct LoginStatusResult {
    pub status: String,
    pub cookie: Option<String>,
}

pub async fn login_status(uid: &str) -> Result<LoginStatusResult, CommandError> {
    let url = format!("{}?uid={}&appid=web", QRCODE_STATUS_API, uid);

    let resp = CLIENT
        .get(&url)
        .header("Referer", "https://115.com/")
        .send()
        .await
        .map_err(|e| CommandError::network(&format!("查询登录状态失败: {}", e)))?;

    // Get headers BEFORE consuming the body
    let headers = resp.headers().clone();
    let body_text = resp.text().await.unwrap_or_default();
    log::debug!("扫码状态原始响应: {}", &body_text[..body_text.len().min(300)]);

    let json: serde_json::Value = match serde_json::from_str(&body_text) {
        Ok(j) => j,
        Err(_) => {
            return Ok(LoginStatusResult { status: "waiting".to_string(), cookie: None });
        }
    };

    let status_code: i64 = json["data"]["status"].as_i64()
        .or_else(|| json["status"].as_i64())
        .unwrap_or(-1);

    let status = match status_code {
        0 => "scanned",
        1 => "authorized",
        2 => "expired",
        _ => "waiting",
    };

    // Extract cookie from the response or from Set-Cookie headers
    let cookie = if status == "authorized" {
        let mut cookie_str = String::new();

        if let Some(c) = json["data"]["cookie"].as_str() {
            cookie_str = c.to_string();
        }

        let set_cookies: Vec<String> = headers
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .map(|s| s.split(';').next().unwrap_or("").to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if !set_cookies.is_empty() {
            let header_cookies = set_cookies.join("; ");
            cookie_str = if cookie_str.is_empty() { header_cookies } else { format!("{}; {}", cookie_str, header_cookies) };
        }

        if cookie_str.is_empty() {
            log::warn!("扫码授权成功但未获取到Cookie，尝试直接登录...");
            // Try to make a follow-up request to get cookies
            None
        } else {
            log::info!("扫码授权成功，获取到Cookie");
            Some(cookie_str)
        }
    } else {
        None
    };

    Ok(LoginStatusResult {
        status: status.to_string(),
        cookie,
    })
}

// ─── Cookie-Based Login (alternative method) ───

pub async fn login_with_cookie(cookie_string: String) -> Result<(), CommandError> {
    // Validate cookie by testing against 115 user info API
    let test_url = format!("{}/user/info", WEBAPI_BASE);
    let resp = match CLIENT
        .get(&test_url)
        .header("Cookie", &cookie_string)
        .header("Referer", "https://115.com/")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            log::warn!("Cookie验证网络请求失败: {}", e);
            return Err(CommandError::unauthorized(&format!("网络错误: {}", e)));
        }
    };

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    log::info!("Cookie验证响应 status={}, preview={}", status, &body[..body.len().min(150)]);

    // Accept both JSON success and HTML responses (115 sometimes returns HTML)
    if status.is_success() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
            if json["code"].as_i64() == Some(0) || json["state"].as_bool() == Some(true) {
                set_cookie(&cookie_string);
                log::info!("Cookie验证成功（JSON）");
                return Ok(());
            }
            let msg = json["message"].as_str().unwrap_or("未知错误");
            return Err(CommandError::unauthorized(&format!("Cookie无效: {}", msg)));
        }
        // If response is HTML but 200 OK, cookie is likely valid
        if body.contains("115") || body.contains("user") || body.contains("UID") || body.len() > 100 {
            set_cookie(&cookie_string);
            log::info!("Cookie验证成功（HTML响应，假定有效）");
            return Ok(());
        }
    }

    Err(CommandError::unauthorized(&format!("Cookie验证失败 (HTTP {})", status)))
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
