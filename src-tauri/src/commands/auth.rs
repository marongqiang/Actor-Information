use crate::services::pan115;
use crate::utils::error::CommandResult;

#[tauri::command]
pub async fn login_qrcode() -> Result<pan115::QrCodeResult, crate::utils::error::CommandError> {
    pan115::login_qrcode().await
}

#[tauri::command]
pub async fn login_status(uid: String) -> Result<pan115::LoginStatusResult, crate::utils::error::CommandError> {
    pan115::login_status(&uid).await
}

#[tauri::command]
pub async fn login_cookie(cookie: String) -> Result<(), crate::utils::error::CommandError> {
    pan115::set_cookie(&cookie);
    log::info!("115登录成功, cookie: {}", crate::utils::logger::mask_cookie(&cookie));
    Ok(())
}

#[tauri::command]
pub fn logout() -> Result<(), crate::utils::error::CommandError> {
    pan115::clear_cookie();
    log::info!("已退出115登录");
    Ok(())
}

#[tauri::command]
pub fn check_token() -> Result<bool, crate::utils::error::CommandError> {
    Ok(pan115::has_cookie())
}
