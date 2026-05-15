use crate::utils::error::CommandError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

static CLIENT: once_cell::sync::Lazy<Client> = once_cell::sync::Lazy::new(|| {
    Client::builder()
        .cookie_store(true)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client")
});

static COOKIE: once_cell::sync::Lazy<Mutex<Option<String>>> = once_cell::sync::Lazy::new(|| Mutex::new(None));

const API_BASE: &str = "https://webapi.115.com";

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
    let url = format!("{}/qrcode?appid=web", API_BASE);
    let resp = CLIENT.get(&url).send().await?;
    let json: serde_json::Value = resp.json().await?;

    let state = json["data"]["state"].as_str().unwrap_or("0");
    let uid = json["data"]["uid"].as_str().unwrap_or("");

    if state != "1" {
        return Err(CommandError::network("获取二维码失败"));
    }

    Ok(QrCodeResult {
        qrcode_url: format!("https://qrcode.115.com/api?appid=web&uid={}", uid),
        uid: uid.to_string(),
    })
}

#[derive(Serialize)]
pub struct LoginStatusResult {
    pub status: String,
    pub cookie: Option<String>,
}

pub async fn login_status(uid: &str) -> Result<LoginStatusResult, CommandError> {
    let url = format!("{}/qrcode/status?uid={}&appid=web", API_BASE, uid);
    let resp = CLIENT.get(&url).send().await?;
    let json: serde_json::Value = resp.json().await?;

    let status_code = json["data"]["status"].as_i64().unwrap_or(-1);
    let status = match status_code {
        0 => "waiting",
        1 => "scanned",
        2 => "authorized",
        -2 => "expired",
        _ => "waiting",
    };

    let cookie = if status == "authorized" {
        json["data"]["cookie"].as_str().map(|s| s.to_string())
    } else {
        None
    };

    Ok(LoginStatusResult {
        status: status.to_string(),
        cookie,
    })
}

// ─── File Operations ───

#[derive(Serialize, Deserialize, Clone)]
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
    let url = format!("{}/files/list?limit=50&offset=0&cid=0", API_BASE);

    let resp = CLIENT.get(&url)
        .header("Cookie", &cookie)
        .send().await?;
    let json: serde_json::Value = resp.json().await?;

    if json["code"].as_i64() != Some(0) {
        return Err(CommandError::network("获取目录列表失败"));
    }

    let items = json["data"]["data"].as_array().unwrap_or(&vec![]).iter()
        .map(|item| FileInfo {
            cid: item["cid"].as_str().unwrap_or("").to_string(),
            name: item["n"].as_str().unwrap_or("未知").to_string(),
            is_dir: item["fid"].as_i64().unwrap_or(0) == 0, // folders have fid=0
            size: item["s"].as_i64().unwrap_or(0),
            update_time: item["t"].as_i64().unwrap_or(0),
            file_id: item["fid"].as_str().map(|s| s.to_string()),
        })
        .collect();

    Ok(items)
}

pub async fn get_files(cid: &str, page: i64, page_size: i64) -> Result<(Vec<FileInfo>, i64), CommandError> {
    let cookie = get_cookie().ok_or_else(|| CommandError::unauthorized("未登录115网盘"))?;
    let offset = (page - 1) * page_size;
    let url = format!(
        "{}/files/list?limit={}&offset={}&cid={}",
        API_BASE, page_size, offset, cid
    );

    let resp = CLIENT.get(&url)
        .header("Cookie", &cookie)
        .send().await?;
    let json: serde_json::Value = resp.json().await?;

    if json["code"].as_i64() != Some(0) {
        return Err(CommandError::network("获取文件列表失败"));
    }

    let total = json["data"]["count"].as_i64().unwrap_or(0);
    let items = json["data"]["data"].as_array().unwrap_or(&vec![]).iter()
        .map(|item| FileInfo {
            cid: item["cid"].as_str().unwrap_or("").to_string(),
            name: item["n"].as_str().unwrap_or("未知").to_string(),
            is_dir: item["fid"].as_i64().unwrap_or(0) == 0,
            size: item["s"].as_i64().unwrap_or(0),
            update_time: item["t"].as_i64().unwrap_or(0),
            file_id: item["fid"].as_str().map(|s| s.to_string()),
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
    let url = format!("{}/files/download?fid={}", API_BASE, file_id);

    let resp = CLIENT.get(&url)
        .header("Cookie", &cookie)
        .send().await?;
    let json: serde_json::Value = resp.json().await?;

    if json["code"].as_i64() != Some(0) {
        return Err(CommandError::network("获取播放链接失败"));
    }

    let play_url = json["data"]["url"].as_str()
        .unwrap_or("")
        .to_string();

    // Links typically expire in 1 hour
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
