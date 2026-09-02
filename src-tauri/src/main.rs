//! Floralis 桌面应用入口
//!
//! 游戏管理桌面应用，支持游戏导入、启动、分类、标签、游戏时间统计等功能
//! 使用 Tauri 2 + Vue 3 + SQLite 构建

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bangumi;
mod commands;
mod db;
mod epic;
mod helpers;
mod igdb;
mod library_watcher;
mod matcher;
mod models;
mod playtime;
mod steam;
mod vndb;

use std::fs;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
    Manager,
};

use db::Database;
use models::AppState;

/// 启动期致命错误：弹窗告知用户后退出（panic 只会静默闪退，用户无从得知原因）
fn fatal_startup_error(msg: &str) -> ! {
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::winuser::{MessageBoxW, MB_ICONERROR, MB_OK};
    let text: Vec<u16> = std::ffi::OsStr::new(msg).encode_wide().chain(std::iter::once(0)).collect();
    let caption: Vec<u16> = std::ffi::OsStr::new("花譜 Floralis 启动失败")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_ICONERROR | MB_OK,
        );
    }
    std::process::exit(1);
}

fn main() {
    if let Err(e) = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data_dir = match app.path().app_data_dir() {
                Ok(p) => p,
                Err(e) => fatal_startup_error(&format!("无法获取应用数据目录: {}", e)),
            };
            if let Err(e) = fs::create_dir_all(&app_data_dir) {
                fatal_startup_error(&format!("无法创建数据目录 {}: {}", app_data_dir.display(), e));
            }
            let db_path = app_data_dir.join("floralis.db");
            // 兼容旧版数据库文件名
            let old_db_path = app_data_dir.join("galm.db");
            if !db_path.exists() && old_db_path.exists() {
                let _ = std::fs::rename(&old_db_path, &db_path);
            }
            let db = match Database::new(&db_path) {
                Ok(db) => Arc::new(db),
                Err(e) => fatal_startup_error(&format!(
                    "数据库初始化失败: {}\n数据库文件: {}",
                    e,
                    db_path.display()
                )),
            };
            // 迁移旧版外部路径的自定义图片到数据目录（asset scope 内）
            helpers::migrate_custom_images(&db, &app_data_dir);
            // 启动游玩时长监控器，并恢复上次退出时未闭合的游玩会话
            let monitor = playtime::PlaytimeMonitor::start(db.clone(), app.handle().clone());
            playtime::recover_open_sessions(&db, &monitor);
            app.manage(AppState { db, monitor });
            app.manage(library_watcher::LibraryWatcherState::default());

            // Setup system tray
            let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            if let Some(tray) = app.tray_by_id("main-tray") {
                tray.set_menu(Some(menu))?;
                tray.set_tooltip(Some("花譜 Floralis"))?;
                tray.on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            window.show().ok();
                            window.set_focus().ok();
                        }
                    }
                });
                tray.on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                window.show().ok();
                                window.set_focus().ok();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                });
            }

            // Intercept close request: hide window instead of closing
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_clone.hide();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Settings & System
            commands::get_settings,
            commands::save_settings,
            commands::save_setting,
            commands::get_setting,
            commands::save_custom_image,
            commands::save_theme,
            commands::save_close_behavior,
            commands::load_image_data,
            commands::start_window_resize,
            commands::test_seven_zip,
            commands::get_passwords,
            commands::add_password,
            commands::remove_password,
            commands::write_text_file,
            commands::read_text_file,
            commands::check_path_exists,
            commands::open_in_explorer,
            commands::force_close,
            // Games
            commands::get_all_games,
            commands::add_game,
            commands::update_game,
            commands::delete_game,
            commands::set_game_group,
            commands::set_game_status,
            commands::set_game_rating,
            commands::set_game_favorite,
            commands::reorder_games,
            commands::launch_game,
            commands::get_launch_actions,
            commands::save_launch_actions,
            commands::get_play_sessions,
            commands::set_game_play_time,
            commands::get_play_calendar,
            commands::get_game_stats,
            commands::get_all_groups,
            commands::add_group,
            commands::rename_group,
            commands::delete_group,
            commands::reorder_groups,
            commands::get_all_tags,
            commands::add_tag,
            commands::delete_tag,
            commands::rename_tag,
            commands::get_tag_usage,
            commands::get_game_tags,
            commands::add_game_tag,
            commands::remove_game_tag,
            commands::get_all_game_tags,
            commands::get_game_screenshots,
            commands::add_game_screenshot,
            commands::delete_game_screenshot,
            commands::scan_game_cover,
            commands::scan_game_save,
            commands::scan_local_game,
            commands::scan_library_root,
            commands::generate_thumbnail,
            commands::copy_cover_to_storage,
            commands::set_game_cover,
            commands::migrate_covers_to_internal,
            commands::check_cover_integrity,
            commands::rescan_game_cover,
            commands::run_integrity_checkup,
            commands::cleanup_orphan_files,
            commands::relocate_game,
            commands::relocate_games_by_root,
            // Mods
            commands::get_all_mods,
            commands::get_mods_by_game,
            commands::get_mods_by_game_dir,
            commands::add_mod,
            commands::update_mod,
            commands::delete_mod,
            commands::toggle_mod_enabled,
            commands::check_mods_integrity,
            commands::get_mod_profiles,
            commands::create_mod_profile,
            commands::rename_mod_profile,
            commands::delete_mod_profile,
            commands::set_mod_profile_mods,
            commands::apply_mod_profile,
            commands::reorder_mods,
            commands::link_mod_to_game,
            commands::unlink_mod_from_game,
            commands::get_mod_tags,
            commands::get_all_mod_tags,
            commands::set_mod_tags,
            commands::copy_mod_cover_to_storage,
            commands::scan_mod_directory,
            commands::extract_mod_files,
            commands::copy_mod_files,
            // Backup & Batch
            commands::extract_game,
            commands::batch_extract_games,
            commands::batch_delete_games,
            commands::batch_set_game_group,
            commands::batch_set_game_status,
            commands::batch_set_game_rating,
            commands::batch_set_game_favorite,
            commands::batch_scan_covers,
            commands::export_data,
            commands::import_data,
            commands::backup_database,
            commands::run_auto_backup,
            commands::backup_game_save,
            commands::list_save_backups,
            commands::restore_game_save,
            commands::delete_save_backup,
            commands::check_for_update,
            commands::save_update_repo,
            // Library watcher
            library_watcher::start_library_watch,
            library_watcher::stop_library_watch,
            // External APIs
            vndb::search_vndb,
            vndb::download_vndb_cover,
            igdb::search_igdb,
            igdb::download_igdb_cover,
            bangumi::search_bangumi,
            bangumi::download_bangumi_cover,
            steam::search_steam,
            steam::download_steam_cover,
            steam::detect_steam_root,
            steam::scan_steam_library,
            matcher::match_game_metadata,
            matcher::take_unreachable_sources,
            epic::detect_epic_manifests_dir,
            epic::scan_epic_library,
        ])
        .run(tauri::generate_context!())
    {
        fatal_startup_error(&format!("启动应用失败: {}", e));
    }
}
