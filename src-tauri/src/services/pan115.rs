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
    // Check cookie has required fields
    let has_uid = cookie_string.contains("UID=") || cookie_string.contains("uid=");
    let has_cid = cookie_string.contains("CID=") || cookie_string.contains("cid=");
    if !has_uid && !has_cid {
        return Err(CommandError::unauthorized("Cookie缺少必要字段(UID/CID)，请从浏览器复制完整Cookie"));
    }

    // Validate by calling the actual file list API
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    let test_url = format!("{}/files?aid=1&cid=0&offset=0&limit=1&show_dir=1&format=json&_={}", WEBAPI_BASE, ts);
    let resp = match CLIENT
        .get(&test_url)
        .header("Cookie", &cookie_string)
        .header("Referer", "https://115.com/")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Err(CommandError::unauthorized(&format!("网络错误: {}", e))),
    };

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    log::info!("Cookie验证 files/list status={}, preview={}", status, &body[..body.len().min(200)]);

    if !status.is_success() {
        return Err(CommandError::unauthorized(&format!("HTTP {}: Cookie无效或网络问题", status)));
    }

    // Try JSON parse
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
        let is_success = json["code"].as_i64() == Some(0)
            || json["state"].as_bool() == Some(true)
            || json["state"].as_i64() == Some(1);

        if is_success {
            set_cookie(&cookie_string);
            log::info!("Cookie验证成功");
            return Ok(());
        }

        let msg = json["message"].as_str()
            .or_else(|| json["error"].as_str())
            .unwrap_or("未知错误");
        log::warn!("Cookie验证API返回错误: {}", msg);
        return Err(CommandError::unauthorized(&format!("Cookie无效: {}", msg)));
    }

    // Non-JSON response: reject (too risky to accept HTML as valid)
    log::warn!("Cookie验证返回非JSON响应: {}", &body[..body.len().min(100)]);
    Err(CommandError::unauthorized("Cookie验证失败：服务器返回异常，请重新获取Cookie"))
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
    let (files, _) = get_files("0", 1, 200).await?;
    Ok(files)
}

pub async fn get_files(cid: &str, page: i64, page_size: i64) -> Result<(Vec<FileInfo>, i64), CommandError> {
    let cookie = get_cookie().ok_or_else(|| CommandError::unauthorized("未登录115网盘"))?;
    let offset = (page - 1) * page_size;
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();

    // 115 files API with full parameters
    let url = format!(
        "{}/files?aid=1&cid={}&o=user_ptime&asc=0&offset={}&show_dir=1&limit={}&snap=0&natsort=1&format=json&_={}",
        WEBAPI_BASE, cid, offset, page_size, ts
    );

    let resp = CLIENT
        .get(&url)
        .header("Cookie", &cookie)
        .header("Referer", "https://115.com/")
        .header("Accept", "application/json")
        .send()
        .await?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    log::info!("get_files cid={} status={} url={}", cid, status, &url[..url.len().min(120)]);
    log::debug!("get_files body: {}", &body[..body.len().min(500)]);

    // Handle non-200
    if !status.is_success() {
        return Err(CommandError::network(&format!("HTTP {}: {}", status, &body[..body.len().min(100)])));
    }

    // Parse response
    let json: serde_json::Value = match serde_json::from_str(&body) {
        Ok(j) => j,
        Err(_) => {
            log::warn!("get_files返回非JSON: {}", &body[..body.len().min(300)]);
            return Err(CommandError::network("115返回格式异常，Cookie可能已过期"));
        }
    };

    // Check success: code=0 or state=true
    let err_code = json["code"].as_i64().unwrap_or(-1);
    let state_ok = json["state"].as_bool().unwrap_or(false);
    let err_msg = json["message"].as_str().or_else(|| json["error"].as_str()).unwrap_or("");

    if err_code != 0 && !state_ok {
        log::warn!("get_files API错误 code={} msg={}", err_code, err_msg);
        if err_code == 99 || err_code == 911 || err_msg.contains("开小差") || err_msg.contains("登录") {
            return Err(CommandError::unauthorized(&format!("Cookie已过期或无效: {}", err_msg)));
        }
        return Err(CommandError::network(&format!("{} ({})", err_msg, err_code)));
    }

    let total = json["data"]["count"].as_i64()
        .or_else(|| json["count"].as_i64())
        .unwrap_or(0);

    // 115 returns files in data array or data.data (paginated)
    let data_array = json["data"]["data"].as_array()
        .or_else(|| json["data"].as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&*EMPTY_ARR);

    let items = data_array.iter()
        .map(|item| {
            let fid = item["fid"].as_str().map(|s| s.to_string())
                .or_else(|| item["file_id"].as_str().map(|s| s.to_string()))
                .or_else(|| item["fid"].as_i64().map(|v| v.to_string()));
            let is_dir = item["fid"].as_i64().unwrap_or(0) == 0
                || item["fid"].is_null()
                || fid.is_none();

            FileInfo {
                cid: item["cid"].as_str()
                    .or_else(|| item["category_id"].as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| cid.to_string()),
                name: item["n"].as_str()
                    .or_else(|| item["name"].as_str())
                    .unwrap_or("未知")
                    .to_string(),
                is_dir,
                size: item["s"].as_i64()
                    .or_else(|| item["size"].as_i64())
                    .unwrap_or(0),
                update_time: item["t"].as_i64()
                    .or_else(|| item["update_time"].as_i64())
                    .or_else(|| item["ptime"].as_i64())
                    .unwrap_or(0),
                file_id: fid,
            }
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
