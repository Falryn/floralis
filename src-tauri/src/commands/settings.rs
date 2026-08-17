//! 设置、密码、文件 I/O、窗口相关命令

use std::fs;
use std::path::Path;
use std::process::Command;

use tauri::{Manager, State};

use crate::helpers::copy_custom_image_to_internal;
use crate::models::AppState;
use crate::db::AppSettings;

// ==================== Settings ====================

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<AppSettings, String> {
    state.db.get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(
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
pub fn save_update_repo(state: State<AppState>, update_repo: String) -> Result<(), String> {
    state
        .db
        .save_setting("update_repo", &update_repo)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_setting(state: State<AppState>, key: String, value: String) -> Result<(), String> {
    state.db.save_setting(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_custom_image(
    app: tauri::AppHandle,
    state: State<AppState>,
    key: String,
    path: String,
) -> Result<String, String> {
    if path.is_empty() {
        state.db.save_setting(&key, "").map_err(|e| e.to_string())?;
        return Ok(String::new());
    }
    if !Path::new(&path).exists() {
        return Err("源文件不存在".into());
    }
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let internal = copy_custom_image_to_internal(&path, &key, &app_data_dir)?;
    state.db.save_setting(&key, &internal).map_err(|e| e.to_string())?;
    Ok(internal)
}

#[tauri::command]
pub fn save_theme(state: State<AppState>, theme: String) -> Result<(), String> {
    state.db.save_setting("theme", &theme).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_close_behavior(state: State<AppState>, close_behavior: String) -> Result<(), String> {
    state.db.save_setting("close_behavior", &close_behavior).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_image_data(path: String) -> Result<String, String> {
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
pub fn start_window_resize(window: tauri::Window, direction: String) -> Result<(), String> {
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
pub fn test_seven_zip(path: String) -> bool {
    Path::new(&path).exists()
        && Command::new(&path).arg("--help").output().is_ok()
}

// ==================== Passwords ====================

#[tauri::command]
pub fn get_passwords(state: State<AppState>) -> Result<Vec<String>, String> {
    state.db.get_passwords().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_password(state: State<AppState>, password: String) -> Result<(), String> {
    state.db.add_password(&password).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_password(state: State<AppState>, password: String) -> Result<(), String> {
    state
        .db
        .remove_password(&password)
        .map_err(|e| e.to_string())
}

// ==================== File I/O ====================

#[tauri::command]
pub fn write_text_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

// ==================== Path / Explorer ====================

#[tauri::command]
pub fn check_path_exists(path: String) -> bool {
    Path::new(&path).exists()
}

#[tauri::command]
pub fn open_in_explorer(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("路径不存在: {}", path));
    }
    let mut cmd = Command::new("explorer");
    if p.is_file() {
        cmd.arg(format!("/select,{}", path));
    } else {
        cmd.arg(&path);
    }
    cmd.spawn().map_err(|e| e.to_string())?;
    Ok(())
}

// ==================== Window ====================

#[tauri::command]
pub fn force_close(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}
