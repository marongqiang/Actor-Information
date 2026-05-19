use std::process::{Child, Command};
use std::sync::Mutex;
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

static SERVER: Mutex<Option<Child>> = Mutex::new(None);
const DEFAULT_PORT: u16 = 9588;

/// Get the base URL for the MetaTube server
pub fn base_url() -> String {
    format!("http://127.0.0.1:{}", DEFAULT_PORT)
}

/// Start the MetaTube server if not already running
pub fn start_server() -> Result<(), String> {
    let mut guard = SERVER.lock().map_err(|e| format!("锁失败: {}", e))?;
    if guard.is_some() {
        return Ok(()); // already running
    }

    let exe_path = std::env::current_exe()
        .map_err(|e| format!("获取exe路径失败: {}", e))?
        .parent()
        .ok_or("无父目录")?
        .join("metatube-server.exe");

    if !exe_path.exists() {
        // Try the SDK build directory as fallback
        let alt_path = std::path::Path::new("D:/Media Library/metatube-sdk-go-main/metatube-server.exe");
        if alt_path.exists() {
            log::info!("MetaTube: 使用备用路径 {}", alt_path.display());
        } else {
            return Err(format!("MetaTube 服务端未找到，请先编译: {}", exe_path.display()));
        }
    }

    let path = if exe_path.exists() { &exe_path } else {
        &std::path::PathBuf::from("D:/Media Library/metatube-sdk-go-main/metatube-server.exe")
    };

    // Read proxy config from DB
    let enabled = crate::db::with_db(|c| {
        crate::db::queries::get_config(c, "proxy_enabled")
    }).ok().flatten().unwrap_or_default();
    let host = crate::db::with_db(|c| {
        crate::db::queries::get_config(c, "proxy_host")
    }).ok().flatten().unwrap_or_default();
    let port = crate::db::with_db(|c| {
        crate::db::queries::get_config(c, "proxy_port")
    }).ok().flatten().unwrap_or_default();
    log::info!("MetaTube: proxy_enabled={} host={} port={}", enabled, host, port);
    let proxy_url = if enabled == "true" && !host.is_empty() {
        Some(format!("http://{}:{}", host, port))
    } else {
        None
    };

    let log_dir = crate::db::get_data_dir().join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("metatube.log");
    let log_file = std::fs::File::create(&log_path)
        .unwrap_or_else(|_| std::fs::File::create("metatube.log").unwrap());

    log::info!("MetaTube: 启动服务 {} --port={} proxy={:?}", path.display(), DEFAULT_PORT, proxy_url);

    let mut cmd = Command::new(path);
    cmd.arg("--port").arg(DEFAULT_PORT.to_string())
       .creation_flags(CREATE_NO_WINDOW)
       .stdout(std::process::Stdio::from(log_file.try_clone().unwrap()))
       .stderr(std::process::Stdio::from(log_file));

    if let Some(ref proxy) = proxy_url {
        cmd.env("HTTP_PROXY", proxy)
           .env("HTTPS_PROXY", proxy)
           .env("MT_PROVIDER__PROXY", proxy); // MetaTube global provider proxy
    }

    let child = cmd.spawn()
        .map_err(|e| format!("启动MetaTube失败: {}", e))?;

    *guard = Some(child);
    log::info!("MetaTube: 服务已启动, 端口={}", DEFAULT_PORT);
    Ok(())
}

/// Stop the MetaTube server
pub fn stop_server() {
    if let Ok(mut guard) = SERVER.lock() {
        if let Some(ref mut child) = *guard {
            let _ = child.kill();
            let _ = child.wait();
            log::info!("MetaTube: 服务已停止");
        }
        *guard = None;
    }
}

/// Check if the server is running and healthy
pub fn health_check() -> bool {
    match reqwest::blocking::get(format!("{}/", base_url())) {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}
