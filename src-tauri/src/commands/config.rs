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

/// Read a local image file and return as base64 data URL
#[tauri::command]
pub fn read_image_base64(path: String) -> Result<String, crate::utils::error::CommandError> {
    let bytes = std::fs::read(&path)
        .map_err(|e| crate::utils::error::CommandError::internal(&format!("读取图片失败: {}", e)))?;
    let ext = std::path::Path::new(&path).extension()
        .and_then(|e| e.to_str()).unwrap_or("jpg");
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
    Ok(format!("data:image/{};base64,{}", ext, b64))
}

/// List all image files in a directory
#[tauri::command]
pub fn list_folder_images(dir_path: String) -> Result<Vec<String>, crate::utils::error::CommandError> {
    let exts = ["jpg", "jpeg", "png", "webp", "gif", "bmp"];
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if exts.contains(&ext.to_lowercase().as_str()) {
                        files.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    // Sort by filename
    files.sort();
    Ok(files)
}

/// Translate text using DeepSeek API (primary) or Google Translate (fallback)
#[tauri::command]
pub fn translate_text(text: String) -> Result<String, crate::utils::error::CommandError> {
    // Try DeepSeek first
    let deepseek_key = crate::services::secure_config::get_secure_config("deepseek_api_key")
        .ok().flatten().unwrap_or_default();
    if !deepseek_key.is_empty() {
        log::info!("翻译: 使用DeepSeek");
        let client = reqwest::blocking::Client::new();
        let body = serde_json::json!({
            "model": "deepseek-chat",
            "messages": [
                {"role": "system", "content": "你是日本AV影片片名本地化翻译专家。将用户输入的日文片名改写成简体中文。\n\n规则：\n1. 完全按中文母语者阅读习惯改写，不要保留日语句式（如'被放置'、'てしまう'等）。\n2. 语序改为中文叙事顺序，通常先说结果或主视角动作（如'发现……'、'妻子被……'）。\n3. 使用AV标题常用中文词汇：'丈夫'或'家暴男'、'人妻'、'全裸'、'鬼鬼祟祟'、'视而不见'。\n4. 保留原文的感叹号、问号、省略号，强化冲击力和悬念感。\n5. 不要直译，不要出现过多'～了'，不要有日语腔。\n\n只返回翻译结果，不要任何解释或评价。"},
                {"role": "user", "content": text}
            ],
            "max_tokens": 100,
            "temperature": 0.3
        });
        match client.post("https://api.deepseek.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", deepseek_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .timeout(std::time::Duration::from_secs(15))
            .send()
        {
            Ok(resp) => {
                if let Ok(json) = resp.json::<serde_json::Value>() {
                    if let Some(choice) = json["choices"][0]["message"]["content"].as_str() {
                        let result = choice.trim().to_string();
                        if !result.is_empty() && result != text {
                            return Ok(result);
                        }
                    }
                }
            }
            Err(e) => log::warn!("DeepSeek翻译失败: {}, 回退到Google", e),
        }
    }

    // Fallback: Google Translate
    log::info!("翻译: 使用Google Translate");
    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=zh-CN&dt=t&q={}",
        encode_uri(&text)
    );
    let resp = reqwest::blocking::get(&url)
        .map_err(|e| crate::utils::error::CommandError::network(&e.to_string()))?;
    let body = resp.text()
        .map_err(|e| crate::utils::error::CommandError::network(&e.to_string()))?;
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| crate::utils::error::CommandError::internal(&format!("翻译解析失败: {}", e)))?;
    let result = json[0][0][0].as_str()
        .unwrap_or(&text)
        .to_string();
    Ok(result)
}

fn encode_uri(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            result.push(b as char);
        } else {
            result.push_str(&format!("%{:02X}", b));
        }
    }
    result
}

#[tauri::command]
pub fn open_devtools(window: tauri::WebviewWindow) {
    window.open_devtools();
}

/// Update MetaTube SDK: git pull + go build
#[tauri::command]
pub fn update_metatube_sdk() -> Result<String, crate::utils::error::CommandError> {
    use std::process::Command;

    let sdk_dir = std::env::current_dir()
        .unwrap_or_default()
        .parent()
        .map(|p| p.join("metatube-sdk-go-main"))
        .unwrap_or_default();

    if !sdk_dir.exists() {
        return Err(crate::utils::error::CommandError::invalid_input(
            "MetaTube SDK 目录不存在，请先 git clone",
        ));
    }

    // Step 1: git pull
    let git_output = Command::new("git")
        .args(["-C", &sdk_dir.to_string_lossy()])
        .arg("pull")
        .output()
        .map_err(|e| crate::utils::error::CommandError::internal(&format!("git pull 失败: {}", e)))?;

    let git_msg = String::from_utf8_lossy(&git_output.stdout).to_string();
    log::info!("git pull: {}", git_msg.trim());

    if !git_output.status.success() {
        let err = String::from_utf8_lossy(&git_output.stderr);
        return Err(crate::utils::error::CommandError::internal(&format!(
            "git pull 失败: {}", err
        )));
    }

    // Step 2: go build
    let go_output = Command::new("go")
        .arg("build")
        .arg("-o")
        .arg("metatube-server.exe")
        .arg("./cmd/server/")
        .current_dir(&sdk_dir)
        .output()
        .map_err(|e| crate::utils::error::CommandError::internal(&format!("go build 失败: {}", e)))?;

    if !go_output.status.success() {
        let err = String::from_utf8_lossy(&go_output.stderr);
        return Err(crate::utils::error::CommandError::internal(&format!(
            "go build 失败: {}", err
        )));
    }

    let exe_path = sdk_dir.join("metatube-server.exe");
    Ok(format!(
        "MetaTube SDK 更新成功！\ngit: {}\n编译产物: {}",
        git_msg.trim(),
        exe_path.display()
    ))
}
