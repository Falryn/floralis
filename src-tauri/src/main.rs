//! Floralis 桌面应用主模块
//! 
//! 游戏管理桌面应用，支持游戏导入、启动、分类、标签、游戏时间统计等功能
//! 使用 Tauri 2 + Vue 3 + SQLite 构建

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod vndb;
mod igdb;

use db::{Database, AppSettings, Game, Group, PlaySession, Tag, GameStats};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
    Manager, State,
};

/// 应用全局状态
struct AppState {
    db: Arc<Database>,
}

/// 压缩包解压结果
#[derive(Serialize)]
struct ExtractResult {
    success: bool,
    exe_path: String,
    cover_path: String,
    detected_name: String,
    extract_dir: String,
    save_path: String,
    error: String,
}

/// 数据备份结构
#[derive(Serialize, Deserialize)]
struct BackupData {
    version: u32,
    games: Vec<Game>,
    groups: Vec<Group>,
    settings: AppSettings,
    passwords: Vec<String>,
}

// ==================== Settings ====================

#[tauri::command]
fn get_settings(state: State<AppState>) -> Result<AppSettings, String> {
    state.db.get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
fn save_settings(
    state: State<AppState>,
    seven_zip_path: String,
    default_extract_path: String,
) -> Result<(), String> {
    state
        .db
        .save_setting("seven_zip_path", &seven_zip_path)
        .map_err(|e| e.to_string())?;
    state
        .db
        .save_setting("default_extract_path", &default_extract_path)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn save_update_repo(state: State<AppState>, update_repo: String) -> Result<(), String> {
    state
        .db
        .save_setting("update_repo", &update_repo)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_custom_image(state: State<AppState>, key: String, path: String) -> Result<(), String> {
    state.db.save_setting(&key, &path).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_theme(state: State<AppState>, theme: String) -> Result<(), String> {
    state.db.save_setting("theme", &theme).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_close_behavior(state: State<AppState>, close_behavior: String) -> Result<(), String> {
    state.db.save_setting("close_behavior", &close_behavior).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_image_data(path: String) -> Result<String, String> {
    use std::io::Read;
    if path.is_empty() || !Path::new(&path).exists() {
        return Ok(String::new());
    }
    let mut file = fs::File::open(&path).map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    let ext = Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_lowercase();
    let mime = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "image/png",
    };
    // Encode as base64
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(buffer.len() * 4 / 3 + 4);
    result.push_str("data:");
    result.push_str(mime);
    result.push_str(";base64,");
    for chunk in buffer.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    Ok(result)
}

#[tauri::command]
fn start_window_resize(window: tauri::Window, direction: String) -> Result<(), String> {
    let dir = match direction.as_str() {
        "north" => tauri_runtime::ResizeDirection::North,
        "south" => tauri_runtime::ResizeDirection::South,
        "east" => tauri_runtime::ResizeDirection::East,
        "west" => tauri_runtime::ResizeDirection::West,
        "north-east" => tauri_runtime::ResizeDirection::NorthEast,
        "north-west" => tauri_runtime::ResizeDirection::NorthWest,
        "south-east" => tauri_runtime::ResizeDirection::SouthEast,
        "south-west" => tauri_runtime::ResizeDirection::SouthWest,
        _ => return Err("Invalid direction".into()),
    };
    window.start_resize_dragging(dir).map_err(|e| e.to_string())
}

#[tauri::command]
fn test_seven_zip(path: String) -> bool {
    Path::new(&path).exists()
        && Command::new(&path).arg("--help").output().is_ok()
}

// ==================== Passwords ====================

#[tauri::command]
fn get_passwords(state: State<AppState>) -> Result<Vec<String>, String> {
    state.db.get_passwords().map_err(|e| e.to_string())
}

#[tauri::command]
fn add_password(state: State<AppState>, password: String) -> Result<(), String> {
    state.db.add_password(&password).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_password(state: State<AppState>, password: String) -> Result<(), String> {
    state
        .db
        .remove_password(&password)
        .map_err(|e| e.to_string())
}

// ==================== File I/O ====================

#[tauri::command]
fn write_text_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_text_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

// ==================== Backup ====================

#[tauri::command]
fn export_data(state: State<AppState>) -> Result<String, String> {
    let games = state.db.get_all_games().map_err(|e| e.to_string())?;
    let groups = state.db.get_all_groups().map_err(|e| e.to_string())?;
    let settings = state.db.get_settings().map_err(|e| e.to_string())?;
    let passwords = state.db.get_passwords().map_err(|e| e.to_string())?;
    let backup = BackupData {
        version: 1,
        games,
        groups,
        settings,
        passwords,
    };
    serde_json::to_string_pretty(&backup).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_data(state: State<AppState>, json: String) -> Result<(), String> {
    let backup: BackupData = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let db = &state.db;

    // Clear existing data
    db.clear_all().map_err(|e| e.to_string())?;

    // Restore groups
    for g in &backup.groups {
        db.add_group(&g.name).map_err(|e| e.to_string())?;
    }
    // Re-fetch groups to get new IDs, then update sort_order
    let new_groups = db.get_all_groups().map_err(|e| e.to_string())?;
    let mut name_to_new_id: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for ng in &new_groups {
        name_to_new_id.insert(ng.name.clone(), ng.id);
    }
    // Restore sort_order
    let mut ordered_ids: Vec<i64> = Vec::new();
    for g in &backup.groups {
        if let Some(&new_id) = name_to_new_id.get(&g.name) {
            ordered_ids.push(new_id);
        }
    }
    if !ordered_ids.is_empty() {
        db.reorder_groups(&ordered_ids).map_err(|e| e.to_string())?;
    }

    // Restore games (with group_id remapping)
    let old_groups = &backup.groups;
    let mut old_id_to_new_id: std::collections::HashMap<i64, i64> = std::collections::HashMap::new();
    for og in old_groups {
        if let Some(&new_id) = name_to_new_id.get(&og.name) {
            old_id_to_new_id.insert(og.id, new_id);
        }
    }
    for game in &backup.games {
        let new_group_id = game.group_id.and_then(|old_id| old_id_to_new_id.get(&old_id).copied());
        db.add_game(
            &game.name,
            new_group_id,
            &game.install_path,
            &game.exe_path,
            &game.launch_args,
            &game.cover_path,
            &game.save_path,
            &game.notes,
            &game.script_path,
            &game.script_args,
        )
        .map_err(|e| e.to_string())?;
    }

    // Restore settings
    let s = &backup.settings;
    db.save_setting("seven_zip_path", &s.seven_zip_path).map_err(|e| e.to_string())?;
    db.save_setting("default_extract_path", &s.default_extract_path).map_err(|e| e.to_string())?;
    db.save_setting("custom_banner", &s.custom_banner).map_err(|e| e.to_string())?;
    db.save_setting("custom_sidebar_bg", &s.custom_sidebar_bg).map_err(|e| e.to_string())?;
    db.save_setting("custom_empty_illustration", &s.custom_empty_illustration).map_err(|e| e.to_string())?;
    db.save_setting("theme", &s.theme).map_err(|e| e.to_string())?;

    // Restore passwords
    for pwd in &backup.passwords {
        db.add_password(pwd).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
fn batch_delete_games(state: State<AppState>, ids: Vec<i64>) -> Result<(), String> {
    state.db.batch_delete_games(&ids).map_err(|e| e.to_string())
}

#[tauri::command]
fn batch_set_game_group(
    state: State<AppState>,
    game_ids: Vec<i64>,
    group_id: Option<i64>,
) -> Result<(), String> {
    state
        .db
        .batch_set_game_group(&game_ids, group_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn batch_scan_covers(app: tauri::AppHandle, state: State<AppState>, game_ids: Vec<i64>) -> Result<u32, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let mut count = 0u32;
    for gid in &game_ids {
        let game = match state.db.get_game_by_id(*gid) {
            Ok(Some(g)) => g,
            _ => continue,
        };
        if game.install_path.is_empty() {
            continue;
        }
        let cover = find_cover_image(Path::new(&game.install_path));
        if !cover.is_empty() {
            match copy_cover_to_internal(&cover, game.id, &app_data_dir) {
                Ok(internal_path) => {
                    let _ = state.db.update_game_cover(game.id, &internal_path);
                }
                Err(_) => {
                    let _ = state.db.update_game_cover(game.id, &cover);
                }
            }
            count += 1;
        }
    }
    Ok(count)
}

// ==================== Batch Operations ====================

#[tauri::command]
fn batch_set_game_status(
    state: State<AppState>,
    game_ids: Vec<i64>,
    status: String,
) -> Result<(), String> {
    state
        .db
        .batch_set_game_status(&game_ids, &status)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn batch_set_game_rating(
    state: State<AppState>,
    game_ids: Vec<i64>,
    rating: i64,
) -> Result<(), String> {
    state
        .db
        .batch_set_game_rating(&game_ids, rating)
        .map_err(|e| e.to_string())
}

// ==================== Extract ====================

#[tauri::command]
fn extract_game(
    state: State<AppState>,
    archive_path: String,
    dest_path: Option<String>,
    password: Option<String>,
) -> Result<ExtractResult, String> {
    let settings = state.db.get_settings().map_err(|e| e.to_string())?;

    if settings.seven_zip_path.is_empty() {
        return Ok(ExtractResult {
            success: false,
            exe_path: String::new(),
            cover_path: String::new(),
            detected_name: String::new(),
            extract_dir: String::new(),
            save_path: String::new(),
            error: "请先在设置中配置7z路径".into(),
        });
    }

    let dest = if let Some(d) = dest_path {
        PathBuf::from(d)
    } else if !settings.default_extract_path.is_empty() {
        PathBuf::from(&settings.default_extract_path)
    } else {
        return Ok(ExtractResult {
            success: false,
            exe_path: String::new(),
            cover_path: String::new(),
            detected_name: String::new(),
            extract_dir: String::new(),
            save_path: String::new(),
            error: "未指定解压路径".into(),
        });
    };

    let archive_stem = Path::new(&archive_path)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let extract_dir = dest.join(&archive_stem);
    fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;

    let mut cmd = Command::new(&settings.seven_zip_path);
    cmd.arg("x")
        .arg("-y")
        .arg("-o")
        .arg(&extract_dir)
        .arg(&archive_path);
    if let Some(pwd) = &password {
        cmd.arg(format!("-p{}", pwd));
    }

    let output = cmd.output().map_err(|e| format!("7z执行失败: {}", e))?;
    if !output.status.success() {
        return Ok(ExtractResult {
            success: false,
            exe_path: String::new(),
            cover_path: String::new(),
            detected_name: String::new(),
            extract_dir: String::new(),
            save_path: String::new(),
            error: format!(
                "解压失败: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        });
    }

    let (exe_path, detected_name) = find_main_exe(&extract_dir);
    let cover_path = find_cover_image(&extract_dir);
    let save_path = find_save_directory(&detected_name, &extract_dir.to_string_lossy());

    Ok(ExtractResult {
        success: true,
        exe_path,
        cover_path,
        detected_name,
        extract_dir: extract_dir.to_string_lossy().to_string(),
        save_path,
        error: String::new(),
    })
}

// ==================== Path Validation ====================

#[tauri::command]
fn check_path_exists(path: String) -> bool {
    std::path::Path::new(&path).exists()
}

// ==================== Thumbnail Generation ====================

#[tauri::command]
fn generate_thumbnail(app: tauri::AppHandle, source_path: String, game_id: i64) -> Result<String, String> {
    use image::imageops::FilterType;
    
    if source_path.is_empty() {
        return Err("源路径为空".to_string());
    }
    
    let src = Path::new(&source_path);
    if !src.exists() {
        return Err("源文件不存在".to_string());
    }
    
    let thumb_dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("thumbnails");
    fs::create_dir_all(&thumb_dir).map_err(|e| e.to_string())?;
    
    let thumb_path = thumb_dir.join(format!("game_{}.jpg", game_id));
    
    // 缩略图已存在则直接返回，避免重复生成
    if thumb_path.exists() {
        return Ok(thumb_path.to_string_lossy().to_string());
    }
    
    let img = image::open(src).map_err(|e| format!("无法打开图片: {}", e))?;
    let thumbnail = img.resize_to_fill(300, 400, FilterType::Lanczos3);
    
    thumbnail.save(&thumb_path).map_err(|e| format!("保存缩略图失败: {}", e))?;
    
    Ok(thumb_path.to_string_lossy().to_string())
}

// ==================== Database Backup ====================

#[tauri::command]
fn backup_database(app: tauri::AppHandle, state: State<AppState>) -> Result<String, String> {
    use std::time::SystemTime;
    
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let backup_dir = app_data_dir.join("backups");
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    
    let backup_path = backup_dir.join(format!("floralis_backup_{}.db", timestamp));
    
    // Get the database path from the state
    let db_path = state.db.get_db_path();
    
    // Copy the database file
    fs::copy(&db_path, &backup_path).map_err(|e| format!("备份失败: {}", e))?;
    
    // Clean up old backups (keep only last 5)
    let mut backups: Vec<_> = fs::read_dir(&backup_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "db"))
        .collect();
    
    backups.sort_by_key(|e| std::cmp::Reverse(e.metadata().ok().and_then(|m| m.modified().ok()).unwrap_or(SystemTime::UNIX_EPOCH)));
    
    for old_backup in backups.into_iter().skip(5) {
        let _ = fs::remove_file(old_backup.path());
    }
    
    Ok(backup_path.to_string_lossy().to_string())
}

// ==================== Cover Copy ====================

#[tauri::command]
fn copy_cover_to_storage(
    app: tauri::AppHandle,
    source_path: String,
    game_id: Option<i64>,
) -> Result<String, String> {
    use std::io::Write;
    if source_path.is_empty() || !Path::new(&source_path).exists() {
        return Err("源文件不存在".into());
    }
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let covers_dir = app_data_dir.join("covers");
    fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;

    // Generate unique filename: game_id or hash + extension
    let ext = Path::new(&source_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_lowercase();
    let filename = if let Some(id) = game_id {
        format!("cover_{}.{}", id, ext)
    } else {
        // Use timestamp + random for temporary naming
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("cover_{}.{}", timestamp, ext)
    };
    let dest = covers_dir.join(&filename);

    // Copy file
    let mut src_file = fs::File::open(&source_path).map_err(|e| e.to_string())?;
    let mut dst_file = fs::File::create(&dest).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut src_file, &mut buf).map_err(|e| e.to_string())?;
    dst_file.write_all(&buf).map_err(|e| e.to_string())?;

    Ok(dest.to_string_lossy().to_string())
}

// ==================== Game CRUD ====================

#[tauri::command]
fn get_all_games(state: State<AppState>) -> Result<Vec<Game>, String> {
    state.db.get_all_games().map_err(|e| e.to_string())
}

#[tauri::command]
fn add_game(
    state: State<AppState>,
    name: String,
    group_id: Option<i64>,
    install_path: String,
    exe_path: String,
    launch_args: String,
    cover_path: String,
    save_path: String,
    notes: String,
    script_path: String,
    script_args: String,
) -> Result<i64, String> {
    state
        .db
        .add_game(
            &name,
            group_id,
            &install_path,
            &exe_path,
            &launch_args,
            &cover_path,
            &save_path,
            &notes,
            &script_path,
            &script_args,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn update_game(
    state: State<AppState>,
    id: i64,
    name: String,
    group_id: Option<i64>,
    install_path: String,
    exe_path: String,
    launch_args: String,
    cover_path: String,
    save_path: String,
    notes: String,
    script_path: String,
    script_args: String,
) -> Result<(), String> {
    state
        .db
        .update_game(
            id, &name, group_id, &install_path, &exe_path, &launch_args, &cover_path, &save_path, &notes, &script_path, &script_args,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_game(state: State<AppState>, id: i64) -> Result<(), String> {
    state.db.delete_game(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_game_group(
    state: State<AppState>,
    game_id: i64,
    group_id: Option<i64>,
) -> Result<(), String> {
    state
        .db
        .set_game_group(game_id, group_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_game_status(state: State<AppState>, game_id: i64, status: String) -> Result<(), String> {
    state.db.set_game_status(game_id, &status).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_game_rating(state: State<AppState>, game_id: i64, rating: i64) -> Result<(), String> {
    state.db.set_game_rating(game_id, rating).map_err(|e| e.to_string())
}

#[tauri::command]
fn launch_game(state: State<AppState>, id: i64) -> Result<(), String> {
    let game = state
        .db
        .get_game_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or("游戏不存在")?;

    // Record play session start
    let now = chrono::Local::now();
    let start_time = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let session_id = state
        .db
        .start_play_session(id, &start_time)
        .map_err(|e| e.to_string())?;

    let db = state.db.clone();

    // If script_path is set, launch the script instead
    if !game.script_path.is_empty() && Path::new(&game.script_path).exists() {
        let mut cmd = Command::new(&game.script_path);
        if !game.script_args.is_empty() {
            for arg in game.script_args.split_whitespace() {
                cmd.arg(arg);
            }
        }
        cmd.current_dir(
            Path::new(&game.script_path)
                .parent()
                .unwrap_or(Path::new("")),
        );
        let child = cmd.spawn().map_err(|e| format!("启动失败: {}", e))?;
        // Monitor process exit in background
        std::thread::spawn(move || {
            if let Ok(output) = child.wait_with_output() {
                let _ = output;
            }
            let end = chrono::Local::now();
            let end_time = end.format("%Y-%m-%d %H:%M:%S").to_string();
            let duration = (end - now).num_seconds().max(0);
            let _ = db.end_play_session(session_id, id, &end_time, duration);
        });
        return Ok(());
    }

    // Fallback: launch exe directly
    if game.exe_path.is_empty() || !Path::new(&game.exe_path).exists() {
        return Err("游戏可执行文件不存在".into());
    }

    let mut cmd = Command::new(&game.exe_path);
    if !game.launch_args.is_empty() {
        for arg in game.launch_args.split_whitespace() {
            cmd.arg(arg);
        }
    }
    cmd.current_dir(
        Path::new(&game.exe_path)
            .parent()
            .unwrap_or(Path::new("")),
    );
    let child = cmd.spawn().map_err(|e| format!("启动失败: {}", e))?;
    // Monitor process exit in background
    std::thread::spawn(move || {
        if let Ok(output) = child.wait_with_output() {
            let _ = output;
        }
        let end = chrono::Local::now();
        let end_time = end.format("%Y-%m-%d %H:%M:%S").to_string();
        let duration = (end - now).num_seconds().max(0);
        let _ = db.end_play_session(session_id, id, &end_time, duration);
    });
    Ok(())
}

#[tauri::command]
fn get_play_sessions(state: State<AppState>, game_id: i64, limit: Option<i64>) -> Result<Vec<PlaySession>, String> {
    state
        .db
        .get_play_sessions(game_id, limit.unwrap_or(10))
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct PlayCalendarDay {
    date: String,
    duration: i64,
}

#[tauri::command]
fn get_play_calendar(state: State<AppState>, year: i32, month: u32) -> Result<Vec<PlayCalendarDay>, String> {
    let data = state.db.get_play_calendar(year, month).map_err(|e| e.to_string())?;
    Ok(data
        .into_iter()
        .map(|(date, duration)| PlayCalendarDay { date, duration })
        .collect())
}

// ==================== Tags ====================

#[tauri::command]
fn get_all_tags(state: State<AppState>) -> Result<Vec<Tag>, String> {
    state.db.get_all_tags().map_err(|e| e.to_string())
}

#[tauri::command]
fn add_tag(state: State<AppState>, name: String) -> Result<i64, String> {
    state.db.add_tag(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_tag(state: State<AppState>, id: i64) -> Result<(), String> {
    state.db.delete_tag(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn rename_tag(state: State<AppState>, id: i64, name: String) -> Result<(), String> {
    state.db.rename_tag(id, &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_game_tags(state: State<AppState>, game_id: i64) -> Result<Vec<Tag>, String> {
    state.db.get_game_tags(game_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_game_tag(state: State<AppState>, game_id: i64, tag_id: i64) -> Result<(), String> {
    state.db.add_game_tag(game_id, tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_game_tag(state: State<AppState>, game_id: i64, tag_id: i64) -> Result<(), String> {
    state.db.remove_game_tag(game_id, tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_all_game_tags(state: State<AppState>) -> Result<HashMap<i64, Vec<Tag>>, String> {
    state.db.get_all_game_tags().map_err(|e| e.to_string())
}

// ==================== Screenshots ====================

#[derive(Serialize)]
struct Screenshot {
    id: i64,
    path: String,
}

#[tauri::command]
fn get_game_screenshots(state: State<AppState>, game_id: i64) -> Result<Vec<Screenshot>, String> {
    let screenshots = state.db.get_game_screenshots(game_id).map_err(|e| e.to_string())?;
    Ok(screenshots
        .into_iter()
        .map(|(id, path)| Screenshot { id, path })
        .collect())
}

#[tauri::command]
fn add_game_screenshot(state: State<AppState>, game_id: i64, path: String) -> Result<i64, String> {
    state.db.add_game_screenshot(game_id, &path).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_game_screenshot(state: State<AppState>, screenshot_id: i64) -> Result<(), String> {
    state.db.delete_game_screenshot(screenshot_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn scan_game_cover(app: tauri::AppHandle, state: State<AppState>, id: i64) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let game = state
        .db
        .get_game_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or("游戏不存在".to_string())?;
    let cover = find_cover_image(Path::new(&game.install_path));
    if !cover.is_empty() {
        match copy_cover_to_internal(&cover, id, &app_data_dir) {
            Ok(internal_path) => {
                state.db.update_game_cover(id, &internal_path).map_err(|e| e.to_string())?;
                return Ok(internal_path);
            }
            Err(_) => {
                state.db.update_game_cover(id, &cover).map_err(|e| e.to_string())?;
                return Ok(cover);
            }
        }
    }
    Ok(cover)
}

#[tauri::command]
fn scan_game_save(state: State<AppState>, id: i64) -> Result<String, String> {
    let game = state
        .db
        .get_game_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or("游戏不存在")?;
    let save_dir = find_save_directory(&game.name, &game.install_path);
    if !save_dir.is_empty() {
        state
            .db
            .update_game_save_path(id, &save_dir)
            .map_err(|e| e.to_string())?;
    }
    Ok(save_dir)
}

#[tauri::command]
fn scan_local_game(dir_path: String) -> Result<ExtractResult, String> {
    let dir = PathBuf::from(&dir_path);
    if !dir.exists() || !dir.is_dir() {
        return Err("目录不存在".into());
    }

    let (exe_path, detected_name) = find_main_exe(&dir);
    let cover_path = find_cover_image(&dir);
    let save_path = find_save_directory(&detected_name, &dir_path);

    Ok(ExtractResult {
        success: true,
        exe_path,
        cover_path,
        detected_name,
        extract_dir: dir_path,
        save_path,
        error: String::new(),
    })
}

#[tauri::command]
fn get_game_stats(state: State<AppState>) -> Result<GameStats, String> {
    state.db.get_game_stats().map_err(|e| e.to_string())
}

// ==================== Cover Migration & Integrity ====================

/// Check if a cover path points inside the app data covers directory
fn is_internal_cover(path: &str, app_data_dir: &Path) -> bool {
    if path.is_empty() { return false; }
    let covers_dir = app_data_dir.join("covers");
    Path::new(path).starts_with(&covers_dir)
}

/// Copy a cover file to the app data covers dir, return new path
fn copy_cover_to_internal(source: &str, game_id: i64, app_data_dir: &Path) -> Result<String, String> {
    use std::io::Write;
    if source.is_empty() || !Path::new(source).exists() {
        return Err("源文件不存在".into());
    }
    let covers_dir = app_data_dir.join("covers");
    fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;
    let ext = Path::new(source)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_lowercase();
    let filename = format!("cover_{}.{}", game_id, ext);
    let dest = covers_dir.join(&filename);
    // Don't re-copy if already the same file
    if Path::new(source).canonicalize().ok() == dest.canonicalize().ok() {
        return Ok(dest.to_string_lossy().to_string());
    }
    let mut src_file = fs::File::open(source).map_err(|e| e.to_string())?;
    let mut dst_file = fs::File::create(&dest).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut src_file, &mut buf).map_err(|e| e.to_string())?;
    dst_file.write_all(&buf).map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
fn migrate_covers_to_internal(
    app: tauri::AppHandle,
    state: State<AppState>,
) -> Result<u32, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let games = state.db.get_all_games().map_err(|e| e.to_string())?;
    let mut count = 0u32;
    for game in &games {
        if game.cover_path.is_empty() { continue; }
        if is_internal_cover(&game.cover_path, &app_data_dir) { continue; }
        // External cover - copy to internal
        match copy_cover_to_internal(&game.cover_path, game.id, &app_data_dir) {
            Ok(new_path) => {
                let _ = state.db.update_game_cover(game.id, &new_path);
                count += 1;
            }
            Err(_) => {} // Skip if copy fails (file may not exist)
        }
    }
    Ok(count)
}

#[derive(Serialize)]
struct CoverStatus {
    game_id: i64,
    game_name: String,
    cover_path: String,
    exists: bool,
}

#[tauri::command]
fn check_cover_integrity(
    app: tauri::AppHandle,
    state: State<AppState>,
) -> Result<Vec<CoverStatus>, String> {
    let _app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let games = state.db.get_all_games().map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    for game in &games {
        if game.cover_path.is_empty() {
            results.push(CoverStatus {
                game_id: game.id,
                game_name: game.name.clone(),
                cover_path: String::new(),
                exists: false,
            });
            continue;
        }
        let exists = Path::new(&game.cover_path).exists();
        results.push(CoverStatus {
            game_id: game.id,
            game_name: game.name.clone(),
            cover_path: game.cover_path.clone(),
            exists,
        });
    }
    Ok(results)
}

#[tauri::command]
fn rescan_game_cover(
    app: tauri::AppHandle,
    state: State<AppState>,
    game_id: i64,
) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let game = state.db.get_game_by_id(game_id)
        .map_err(|e| e.to_string())?
        .ok_or("游戏不存在".to_string())?;
    let cover = find_cover_image(Path::new(&game.install_path));
    if cover.is_empty() {
        return Err("未找到封面图".into());
    }
    // Copy to internal storage
    let internal_path = copy_cover_to_internal(&cover, game_id, &app_data_dir)
        .map_err(|e| e.to_string())?;
    state.db.update_game_cover(game_id, &internal_path).map_err(|e| e.to_string())?;
    Ok(internal_path)
}

// ==================== Groups ====================

#[tauri::command]
fn get_all_groups(state: State<AppState>) -> Result<Vec<Group>, String> {
    state.db.get_all_groups().map_err(|e| e.to_string())
}

#[tauri::command]
fn add_group(state: State<AppState>, name: String) -> Result<i64, String> {
    state.db.add_group(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn rename_group(state: State<AppState>, id: i64, name: String) -> Result<(), String> {
    state
        .db
        .rename_group(id, &name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_group(state: State<AppState>, id: i64) -> Result<(), String> {
    state.db.delete_group(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn reorder_groups(state: State<AppState>, ordered_ids: Vec<i64>) -> Result<(), String> {
    state.db.reorder_groups(&ordered_ids).map_err(|e| e.to_string())
}

// ==================== Helpers ====================

fn find_main_exe(dir: &Path) -> (String, String) {
    let mut search_dirs = vec![dir.to_path_buf()];

    // If only one subdirectory at root, also search inside it
    if let Ok(entries) = fs::read_dir(dir) {
        let subdirs: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.path())
            .collect();
        if subdirs.len() == 1 {
            search_dirs.push(subdirs[0].clone());
        }
    }

    let skip_names = [
        "unins", "setup", "install", "config", "crack", "patch", "update",
    ];

    for search_dir in &search_dirs {
        if let Ok(entries) = fs::read_dir(search_dir) {
            let exes: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext.eq_ignore_ascii_case("exe"))
                        .unwrap_or(false)
                })
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_lowercase();
                    !skip_names.iter().any(|s| name.contains(s))
                })
                .collect();

            if let Some(exe) = exes.first() {
                let exe_path = exe.path().to_string_lossy().to_string();
                // Use folder name as game name (more reliable than exe version info for galgames)
                let name = exe
                    .path()
                    .parent()
                    .and_then(|p| p.file_name())
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                return (exe_path, name);
            }
        }
    }

    (String::new(), String::new())
}

fn find_cover_image(dir: &Path) -> String {
    let priority_names = [
        "cover.jpg",
        "cover.png",
        "cover.webp",
        "folder.jpg",
        "folder.png",
        "thumb.jpg",
        "thumb.png",
        "icon.jpg",
        "icon.png",
    ];

    let mut search_dirs = vec![dir.to_path_buf()];
    if let Ok(entries) = fs::read_dir(dir) {
        let subdirs: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.path())
            .collect();
        if subdirs.len() == 1 {
            search_dirs.push(subdirs[0].clone());
        }
    }

    // Check priority names first
    for name in &priority_names {
        for sd in &search_dirs {
            let path = sd.join(name);
            if path.exists() {
                return path.to_string_lossy().to_string();
            }
        }
    }

    // Fallback: find first image > 50KB
    let img_exts = ["jpg", "jpeg", "png", "webp", "bmp"];
    for sd in &search_dirs {
        if let Ok(entries) = fs::read_dir(sd) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if img_exts.iter().any(|e| ext.eq_ignore_ascii_case(e)) {
                        if let Ok(meta) = fs::metadata(&path) {
                            if meta.len() > 50_000 {
                                return path.to_string_lossy().to_string();
                            }
                        }
                    }
                }
            }
        }
    }

    String::new()
}

fn find_save_directory(game_name: &str, install_path: &str) -> String {
    let save_patterns = ["save", "sav", "savedata", "data", "userdata"];

    // Check common Windows locations
    let mut base_dirs = Vec::new();
    if let Ok(v) = std::env::var("APPDATA") {
        base_dirs.push(PathBuf::from(v));
    }
    if let Ok(v) = std::env::var("LOCALAPPDATA") {
        base_dirs.push(PathBuf::from(v));
    }
    if let Ok(v) = std::env::var("USERPROFILE") {
        base_dirs.push(PathBuf::from(&v).join("Documents"));
        base_dirs.push(PathBuf::from(&v).join("Saved Games"));
    }

    for base in &base_dirs {
        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                let dir_name = entry.file_name().to_string_lossy().to_lowercase();
                if dir_name.contains(&game_name.to_lowercase()) {
                    let path = entry.path();
                    if path.is_dir() {
                        return path.to_string_lossy().to_string();
                    }
                }
                if save_patterns.iter().any(|p| dir_name.contains(p)) {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Ok(sub) = fs::read_dir(&path) {
                            for s in sub.flatten() {
                                let ext = s
                                    .path()
                                    .extension()
                                    .map(|e| e.to_string_lossy().to_lowercase())
                                    .unwrap_or_default();
                                if ["sav", "dat", "bin", "json", "db"]
                                    .iter()
                                    .any(|e| ext == *e)
                                {
                                    return path.to_string_lossy().to_string();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Check inside install directory
    if let Ok(entries) = fs::read_dir(install_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                if save_patterns.iter().any(|p| name.contains(p)) {
                    return path.to_string_lossy().to_string();
                }
            }
        }
    }

    String::new()
}

// ==================== Update Check ====================

#[derive(Serialize)]
struct UpdateInfo {
    available: bool,
    current_version: String,
    latest_version: String,
    release_url: String,
    release_notes: String,
}

#[tauri::command]
fn check_for_update(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    let current_version = app.config().version.clone()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "0.0.0".to_string());

    let settings = app.state::<AppState>().db.get_settings().map_err(|e| e.to_string())?;
    let repo = if settings.update_repo.is_empty() {
        "echon/floralis"
    } else {
        &settings.update_repo
    };

    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);

    let resp: Result<ureq::Response, ureq::Error> = ureq::get(&url)
        .set("User-Agent", "Floralis-Updater")
        .call();

    match resp {
        Ok(response) => {
            let body: serde_json::Value = response.into_json().map_err(|e| e.to_string())?;
            let tag = body["tag_name"].as_str().unwrap_or("0.0.0").to_string();
            let latest_version = tag.trim_start_matches('v').to_string();
            let release_url = body["html_url"].as_str().unwrap_or("").to_string();
            let release_notes = body["body"].as_str().unwrap_or("").to_string();

            let available = compare_versions(&current_version, &latest_version);

            Ok(UpdateInfo {
                available,
                current_version,
                latest_version,
                release_url,
                release_notes,
            })
        }
        Err(ureq::Error::Status(404, _)) => {
            Ok(UpdateInfo {
                available: false,
                current_version: current_version.clone(),
                latest_version: current_version,
                release_url: String::new(),
                release_notes: String::new(),
            })
        }
        Err(e) => Err(format!("网络请求失败: {}", e))
    }
}

/// Returns true if latest > current
fn compare_versions(current: &str, latest: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let c = parse(current);
    let l = parse(latest);
    for i in 0..l.len().max(c.len()) {
        let cv = c.get(i).copied().unwrap_or(0);
        let lv = l.get(i).copied().unwrap_or(0);
        if lv > cv { return true; }
        if lv < cv { return false; }
    }
    false
}

// ==================== Main ====================

#[tauri::command]
fn force_close(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

#[tauri::command]
fn reorder_games(state: State<AppState>, game_ids: Vec<i64>) -> Result<(), String> {
    state.db.reorder_games(&game_ids).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("无法获取应用数据目录");
            fs::create_dir_all(&app_data_dir).expect("无法创建数据目录");
            let db_path = app_data_dir.join("galm.db");
            let db = Arc::new(Database::new(&db_path).expect("数据库初始化失败"));
            app.manage(AppState { db });

            // Setup system tray
            let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            if let Some(tray) = app.tray_by_id("main-tray") {
                tray.set_menu(Some(menu))?;
                tray.set_tooltip(Some("花譜 Floralis"))?;
                tray.on_tray_icon_event(|tray, event| {
                    match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } => {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                window.show().ok();
                                window.set_focus().ok();
                            }
                        }
                        _ => {}
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
            get_settings,
            save_settings,
            save_custom_image,
            save_theme,
            save_close_behavior,
            load_image_data,
            start_window_resize,
            test_seven_zip,
            get_passwords,
            add_password,
            remove_password,
            extract_game,
            check_path_exists,
            generate_thumbnail,
            backup_database,
            get_all_games,
            add_game,
            update_game,
            delete_game,
            set_game_group,
            set_game_status,
            set_game_rating,
            launch_game,
            get_all_game_tags,
            scan_game_cover,
            scan_game_save,
            scan_local_game,
            copy_cover_to_storage,
            get_all_groups,
            add_group,
            rename_group,
            delete_group,
            reorder_groups,
            export_data,
            import_data,
            write_text_file,
            read_text_file,
            batch_delete_games,
            batch_set_game_group,
            batch_set_game_status,
            batch_set_game_rating,
            batch_scan_covers,
            get_play_sessions,
            get_play_calendar,
            check_for_update,
            save_update_repo,
            get_all_tags,
            get_game_stats,
            migrate_covers_to_internal,
            check_cover_integrity,
            rescan_game_cover,
            add_tag,
            delete_tag,
            rename_tag,
            get_game_tags,
            add_game_tag,
            remove_game_tag,
            get_game_screenshots,
            add_game_screenshot,
            delete_game_screenshot,
            force_close,
            vndb::search_vndb,
            vndb::download_vndb_cover,
            igdb::search_igdb,
            igdb::download_igdb_cover,
            reorder_games,
        ])
        .run(tauri::generate_context!())
        .expect("启动应用失败");
}
