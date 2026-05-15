use fern::Dispatch;
use log::LevelFilter;
use std::fs;
use std::path::PathBuf;

pub fn init_logger(app_data_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // 使用exe所在目录作为日志目录（便携版方便查看）
    let log_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("logs")))
        .unwrap_or_else(|| app_data_dir.join("logs"));
    fs::create_dir_all(&log_dir)?;

    let log_file = log_dir.join("app.log");

    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] [{}] [{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(if cfg!(debug_assertions) { LevelFilter::Debug } else { LevelFilter::Info })
        .chain(fern::log_file(&log_file)?)
        .apply()?;

    // Set up panic hook to log panics
    std::panic::set_hook(Box::new(|info| {
        log::error!("应用崩溃: {}", info);
    }));

    log::info!("日志系统初始化完成, 存储路径: {}", log_file.display());
    Ok(())
}

/// Log sensitive data with masking
pub fn mask_cookie(cookie: &str) -> String {
    if cookie.len() <= 8 {
        return "****".to_string();
    }
    format!("{}****", &cookie[..8])
}

pub fn mask_api_key(key: &str) -> String {
    if key.len() <= 4 {
        return "****".to_string();
    }
    format!("{}****", &key[..4])
}
