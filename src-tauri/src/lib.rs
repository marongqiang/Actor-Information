mod commands;
mod db;
mod services;
mod utils;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logger first (will be re-initialized during setup)
    let _ = utils::logger::init_logger(&db::get_data_dir());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize logger with correct data dir
            let data_dir = app.path().app_data_dir()?;
            let _ = utils::logger::init_logger(&data_dir);

            // Initialize database (lazy, first access will trigger)
            let _ = &*db::DB;

            // Restore persisted login
            crate::services::pan115::restore_cookie();

            log::info!("智能网盘影视库 v{} 启动完成", app.package_info().version);

            // Build tray menu
            let _tray = app.tray_by_id("main");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Auth
            commands::auth::login_qrcode,
            commands::auth::login_status,
            commands::auth::login_cookie,
            commands::auth::login_cookie_direct,
            commands::auth::logout,
            commands::auth::check_token,
            // FS
            commands::fs::list_root,
            commands::fs::get_files,
            commands::fs::scan_directory,
            commands::fs::get_play_url,
            commands::fs::refresh_play_url,
            commands::fs::export_list,
            // Scrape
            commands::scrape::scrape_batch,
            commands::scrape::start_scrape,
            commands::scrape::pause_scrape,
            commands::scrape::resume_scrape,
            commands::scrape::manual_scrape,
            commands::scrape::select_scrape_result,
            commands::scrape::test_source,
            // Library
            commands::library::get_movies,
            commands::library::get_movie_detail,
            commands::library::batch_action,
            commands::library::hide_movies,
            commands::library::unhide_movies,
            commands::library::batch_set_scrape_status,
            // Groups
            commands::groups::get_groups,
            commands::groups::clean_all_groups,
            commands::groups::create_group,
            commands::groups::rename_group,
            commands::groups::delete_group,
            commands::groups::reorder_groups,
            commands::groups::add_movies_to_group,
            commands::groups::remove_movie_from_group,
            commands::groups::get_actress_groups,
            commands::groups::create_actress_group,
            commands::groups::rename_actress_group,
            commands::groups::delete_actress_group,
            commands::groups::add_actresses_to_group,
            commands::groups::remove_actress_from_group,
            // Actress
            commands::actress_merge::sync_actress_data,
            commands::actress_merge::get_actresses_by_letter,
            commands::actress_merge::get_actresses_paginated,
            commands::actress_merge::find_actress,
            commands::actress_merge::update_actress,
            commands::actress_merge::delete_actresses,
            commands::actress_merge::delete_all_actresses,
            commands::actress_merge::get_actress_aliases,
            commands::actress_merge::add_actress_alias,
            commands::actress_merge::merge_actresses,
            commands::actress_merge::scan_local_actress_folder,
            commands::actress_merge::refresh_actress_avatar,
            commands::actress_merge::confirm_actor,
            commands::actress_merge::update_actor_local_folder,
            commands::actress_merge::rename_actor_and_folder,
            commands::actress_merge::detect_duplicate_actresses,
            commands::actress_merge::get_actress_local_folder,
            commands::actress_merge::sync_actress_with_local_folder,
            // Player
            commands::player::get_progress,
            commands::player::save_progress,
            commands::player::end_playback,
            // Config
            commands::config::get_config,
            commands::config::set_config,
            commands::config::get_all_config,
            commands::config::set_secure_config,
            commands::config::get_secure_config,
            commands::config::clear_secure_config,
            // Task
            commands::task::get_pending_tasks,
            commands::task::resume_task,
            commands::task::cancel_task,
        ])
        .run(tauri::generate_context!())
        .expect("启动应用失败");
}
