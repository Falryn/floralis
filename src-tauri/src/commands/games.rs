//! 游戏 CRUD、启动、分组、标签、截图、统计相关命令

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use tauri::{Manager, State};

use crate::helpers::{copy_cover_to_internal, copy_file_to_appdata, find_cover_image, find_main_exe, find_save_directory, is_internal_cover};
use crate::models::{AppState, ExtractResult, IntegrityIssue, IntegrityReport, PlayCalendarDay, RelocateReport, Screenshot};
use crate::db::{Game, GameStats, Group, LaunchAction, PlaySession, Tag, TagUsage};

// ==================== Game CRUD ====================

#[tauri::command]
pub fn get_all_games(state: State<AppState>) -> Result<Vec<Game>, String> {
    state.db.get_all_games().map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn add_game(
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
            &name, group_id, &install_path, &exe_path, &launch_args,
            &cover_path, &save_path, &notes, &script_path, &script_args,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_game(
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
    default_mod_dir: String,
    mod_naming_pattern: String,
    mod_uses_load_order: bool,
    tracked_process_name: String,
) -> Result<(), String> {
    state
        .db
        .update_game(
            id, &name, group_id, &install_path, &exe_path, &launch_args,
            &cover_path, &save_path, &notes, &script_path, &script_args,
            &default_mod_dir, &mod_naming_pattern, mod_uses_load_order,
            &tracked_process_name,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_game_play_time(state: State<AppState>, game_id: i64, seconds: i64) -> Result<(), String> {
    state
        .db
        .set_game_play_time(game_id, seconds)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_game(state: State<AppState>, id: i64) -> Result<(), String> {
    state.db.delete_game(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_game_group(
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
pub fn set_game_status(state: State<AppState>, game_id: i64, status: String) -> Result<(), String> {
    state.db.set_game_status(game_id, &status).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_game_rating(state: State<AppState>, game_id: i64, rating: i64) -> Result<(), String> {
    state.db.set_game_rating(game_id, rating).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_game_favorite(state: State<AppState>, game_id: i64, favorite: bool) -> Result<(), String> {
    state.db.set_game_favorite(game_id, favorite).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_games(state: State<AppState>, game_ids: Vec<i64>) -> Result<(), String> {
    state.db.reorder_games(&game_ids).map_err(|e| e.to_string())
}

// ==================== Launch ====================

/// 以与父进程完全分离的方式启动外部进程：
/// 独立进程组、不继承控制台与 stdio 句柄。
/// 某些游戏（尤其带反作弊/启动器的网游）作为 Tauri 控制台的子进程、
/// 或继承了被占用/已关闭的控制台句柄时会立即退出，分离启动可避免该问题。
fn spawn_detached(cmd: &mut Command) -> std::io::Result<Child> {
    #[cfg(windows)]
    {
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

/// 启动外部程序：优先分离式直接 spawn，失败后回退到 `cmd /c start`
///（走 ShellExecute 语义，兼容部分受保护的可执行文件）
fn launch_external(program: &str, args: &[String], current_dir: &Path) -> Result<Child, String> {
    let mut cmd = Command::new(program);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.current_dir(current_dir);
    if let Ok(child) = spawn_detached(&mut cmd) {
        return Ok(child);
    }
    let mut fallback = Command::new("cmd");
    fallback.arg("/c").arg("start").arg("").arg(program);
    for arg in args {
        fallback.arg(arg);
    }
    fallback.current_dir(current_dir);
    spawn_detached(&mut fallback).map_err(|e| format!("启动失败: {}", e))
}

#[tauri::command]
pub fn launch_game(state: State<AppState>, id: i64, action_id: Option<i64>) -> Result<(), String> {
    let game = state
        .db
        .get_game_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or("游戏不存在")?;

    // 指定附加启动入口：直接从新表取入口启动（时长按安装目录匹配进程，天然覆盖）
    if let Some(aid) = action_id {
        let action = state
            .db
            .get_launch_action_by_id(aid)
            .map_err(|e| e.to_string())?
            .ok_or("启动入口不存在")?;
        if action.program_path.is_empty() || !Path::new(&action.program_path).exists() {
            return Err("启动入口的可执行文件不存在".into());
        }
        let now = chrono::Local::now();
        let start_time = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let session_id = state
            .db
            .start_play_session(id, &start_time)
            .map_err(|e| e.to_string())?;
        let args: Vec<String> = action
            .args
            .split_whitespace()
            .map(String::from)
            .collect();
        let current_dir = Path::new(&action.program_path)
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf();
        if let Err(e) = launch_external(&action.program_path, &args, &current_dir) {
            let _ = state.db.close_play_session(session_id, &start_time);
            return Err(e);
        }
        state.monitor.track(&game, session_id);
        return Ok(());
    }

    // 先确定启动目标并校验存在性，再开始计时会话，
    // 避免目标缺失时留下永不闭合的会话记录
    let (program, raw_args) = if !game.script_path.is_empty() && Path::new(&game.script_path).exists() {
        (game.script_path.clone(), game.script_args.clone())
    } else {
        if game.exe_path.is_empty() || !Path::new(&game.exe_path).exists() {
            return Err("游戏可执行文件不存在".into());
        }
        (game.exe_path.clone(), game.launch_args.clone())
    };

    let now = chrono::Local::now();
    let start_time = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let session_id = state
        .db
        .start_play_session(id, &start_time)
        .map_err(|e| e.to_string())?;

    let args: Vec<String> = raw_args.split_whitespace().map(String::from).collect();
    let current_dir = Path::new(&program)
        .parent()
        .unwrap_or(Path::new(""))
        .to_path_buf();
    if let Err(e) = launch_external(&program, &args, &current_dir) {
        // 启动失败：立即闭合会话，避免悬挂
        let _ = state.db.close_play_session(session_id, &start_time);
        return Err(e);
    }
    // 时长统计由监控器基于进程扫描完成，无需等待子进程退出
    state.monitor.track(&game, session_id);
    Ok(())
}

// ==================== Launch Actions ====================

#[tauri::command]
pub fn get_launch_actions(state: State<AppState>, game_id: i64) -> Result<Vec<LaunchAction>, String> {
    state
        .db
        .get_launch_actions(game_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_launch_actions(
    state: State<AppState>,
    game_id: i64,
    actions: Vec<LaunchAction>,
) -> Result<Vec<LaunchAction>, String> {
    state
        .db
        .replace_launch_actions(game_id, &actions)
        .map_err(|e| e.to_string())
}

// ==================== Play Sessions ====================

#[tauri::command]
pub fn get_play_sessions(state: State<AppState>, game_id: i64, limit: Option<i64>) -> Result<Vec<PlaySession>, String> {
    state
        .db
        .get_play_sessions(game_id, limit.unwrap_or(10))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_play_calendar(state: State<AppState>, year: i32, month: u32) -> Result<Vec<PlayCalendarDay>, String> {
    let data = state.db.get_play_calendar(year, month).map_err(|e| e.to_string())?;
    Ok(data
        .into_iter()
        .map(|(date, duration)| PlayCalendarDay { date, duration })
        .collect())
}

#[tauri::command]
pub fn get_game_stats(state: State<AppState>) -> Result<GameStats, String> {
    state.db.get_game_stats().map_err(|e| e.to_string())
}

// ==================== Groups ====================

#[tauri::command]
pub fn get_all_groups(state: State<AppState>) -> Result<Vec<Group>, String> {
    state.db.get_all_groups().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_group(state: State<AppState>, name: String) -> Result<i64, String> {
    state.db.add_group(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_group(state: State<AppState>, id: i64, name: String) -> Result<(), String> {
    state
        .db
        .rename_group(id, &name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_group(state: State<AppState>, id: i64) -> Result<(), String> {
    state.db.delete_group(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_groups(state: State<AppState>, ordered_ids: Vec<i64>) -> Result<(), String> {
    state.db.reorder_groups(&ordered_ids).map_err(|e| e.to_string())
}

// ==================== Tags ====================

#[tauri::command]
pub fn get_all_tags(state: State<AppState>) -> Result<Vec<Tag>, String> {
    state.db.get_all_tags().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_tag(state: State<AppState>, name: String) -> Result<i64, String> {
    state.db.add_tag(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_tag(state: State<AppState>, id: i64) -> Result<(), String> {
    state.db.delete_tag(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_tag(state: State<AppState>, id: i64, name: String) -> Result<(), String> {
    state.db.rename_tag(id, &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_tag_usage(state: State<AppState>) -> Result<Vec<TagUsage>, String> {
    state.db.get_tag_usage().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_game_tags(state: State<AppState>, game_id: i64) -> Result<Vec<Tag>, String> {
    state.db.get_game_tags(game_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_game_tag(state: State<AppState>, game_id: i64, tag_id: i64) -> Result<(), String> {
    state.db.add_game_tag(game_id, tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_game_tag(state: State<AppState>, game_id: i64, tag_id: i64) -> Result<(), String> {
    state.db.remove_game_tag(game_id, tag_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_all_game_tags(state: State<AppState>) -> Result<HashMap<i64, Vec<Tag>>, String> {
    state.db.get_all_game_tags().map_err(|e| e.to_string())
}

// ==================== Screenshots ====================

#[tauri::command]
pub fn get_game_screenshots(state: State<AppState>, game_id: i64) -> Result<Vec<Screenshot>, String> {
    let screenshots = state.db.get_game_screenshots(game_id).map_err(|e| e.to_string())?;
    Ok(screenshots
        .into_iter()
        .map(|(id, path)| Screenshot { id, path })
        .collect())
}

#[tauri::command]
pub fn add_game_screenshot(state: State<AppState>, game_id: i64, path: String) -> Result<i64, String> {
    state.db.add_game_screenshot(game_id, &path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_game_screenshot(state: State<AppState>, screenshot_id: i64) -> Result<(), String> {
    state.db.delete_game_screenshot(screenshot_id).map_err(|e| e.to_string())
}

// ==================== Cover Scanning ====================

#[tauri::command]
pub async fn scan_game_cover(app: tauri::AppHandle, state: State<'_, AppState>, id: i64) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        scan_game_cover_impl(&db, &app_data_dir, id)
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

fn scan_game_cover_impl(db: &crate::db::Database, app_data_dir: &Path, id: i64) -> Result<String, String> {
    let game = db
        .get_game_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or("游戏不存在".to_string())?;
    let cover = find_cover_image(Path::new(&game.install_path));
    if !cover.is_empty() {
        match copy_cover_to_internal(&cover, id, app_data_dir) {
            Ok(internal_path) => {
                db.update_game_cover(id, &internal_path).map_err(|e| e.to_string())?;
                return Ok(internal_path);
            }
            Err(_) => {
                db.update_game_cover(id, &cover).map_err(|e| e.to_string())?;
                return Ok(cover);
            }
        }
    }
    Ok(cover)
}

#[tauri::command]
pub fn scan_game_save(state: State<AppState>, id: i64) -> Result<String, String> {
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
pub fn scan_local_game(dir_path: String) -> Result<ExtractResult, String> {
    let dir = std::path::PathBuf::from(&dir_path);
    if !dir.exists() || !dir.is_dir() {
        return Err("目录不存在".into());
    }

    let found = find_main_exe(&dir);
    let cover_path = find_cover_image(&dir);
    let save_path = find_save_directory(&found.save_hint, &dir_path);

    Ok(ExtractResult {
        success: true,
        exe_path: found.exe_path,
        cover_path,
        detected_name: found.detected_name,
        extract_dir: dir_path,
        save_path,
        error: String::new(),
        name_candidates: found.name_candidates,
    })
}

/// 扫描游戏库根目录：枚举直接子文件夹，每个子文件夹识别为一个游戏（Playnite 式批量导入）
///
/// 已在库中的目录（按 install_path 归一化比较）自动跳过。
#[tauri::command]
pub async fn scan_library_root(
    state: State<'_, AppState>,
    dir_path: String,
) -> Result<Vec<ExtractResult>, String> {
    // 已导入游戏的安装目录集合（归一化：小写 + 反斜杠）
    let existing: std::collections::HashSet<String> = state
        .db
        .get_all_games()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|g| !g.install_path.is_empty())
        .map(|g| g.install_path.replace('/', "\\").to_lowercase())
        .collect();

    tauri::async_runtime::spawn_blocking(move || {
        let root = std::path::PathBuf::from(&dir_path);
        if !root.exists() || !root.is_dir() {
            return Err("目录不存在".into());
        }
        let mut dirs: Vec<std::path::PathBuf> = fs::read_dir(&root)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|e| e.path())
            .collect();
        dirs.sort();

        let mut results = Vec::new();
        for dir in dirs {
            // 跳过隐藏目录
            if dir
                .file_name()
                .map(|n| n.to_string_lossy().starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }
            let dir_str = dir.to_string_lossy().to_string();
            if existing.contains(&dir_str.replace('/', "\\").to_lowercase()) {
                continue; // 已在库中，跳过
            }
            let found = find_main_exe(&dir);
            let cover_path = find_cover_image(&dir);
            let save_path = find_save_directory(&found.save_hint, &dir_str);
            results.push(ExtractResult {
                success: true,
                exe_path: found.exe_path,
                cover_path,
                detected_name: found.detected_name,
                extract_dir: dir_str,
                save_path,
                error: String::new(),
                name_candidates: found.name_candidates,
            });
        }
        Ok(results)
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

#[tauri::command]
pub async fn generate_thumbnail(app: tauri::AppHandle, source_path: String, game_id: i64) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        generate_thumbnail_impl(&app_data_dir, source_path, game_id)
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

fn generate_thumbnail_impl(app_data_dir: &Path, source_path: String, game_id: i64) -> Result<String, String> {
    use image::imageops::FilterType;

    if source_path.is_empty() {
        return Err("源路径为空".to_string());
    }

    let src = Path::new(&source_path);
    if !src.exists() {
        return Err("源文件不存在".to_string());
    }

    let thumb_dir = app_data_dir.join("thumbnails");
    fs::create_dir_all(&thumb_dir).map_err(|e| e.to_string())?;

    let thumb_path = thumb_dir.join(format!("game_{}_v2.jpg", game_id));

    if thumb_path.exists() {
        return Ok(thumb_path.to_string_lossy().to_string());
    }

    let img = image::open(src).map_err(|e| format!("无法打开图片: {}", e))?;
    // 保持原始比例缩放（不裁切），前端根据方向决定显示方式
    let thumbnail = img.resize(300, 400, FilterType::Lanczos3);

    thumbnail.save(&thumb_path).map_err(|e| format!("保存缩略图失败: {}", e))?;

    Ok(thumb_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn copy_cover_to_storage(
    app: tauri::AppHandle,
    source_path: String,
    game_id: Option<i64>,
) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    copy_file_to_appdata(&source_path, "covers", "cover", game_id, &app_data_dir)
}

/// 设置游戏封面：将指定图片复制到内部存储、清理旧封面与缩略图缓存并更新数据库
///
/// 文件名带时间戳，保证路径变化后前端各视图能自动刷新。
#[tauri::command]
pub fn set_game_cover(
    app: tauri::AppHandle,
    state: State<AppState>,
    game_id: i64,
    source_path: String,
) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let src = Path::new(&source_path);
    if source_path.is_empty() || !src.exists() {
        return Err("源文件不存在".into());
    }
    // 源文件已是内部封面（如 Steam CDN 下载结果）：仅更新数据库，避免自我复制截断文件
    if is_internal_cover(&source_path, &app_data_dir) {
        let thumb = app_data_dir.join("thumbnails").join(format!("game_{}_v2.jpg", game_id));
        let _ = fs::remove_file(&thumb);
        state.db.update_game_cover(game_id, &source_path).map_err(|e| e.to_string())?;
        return Ok(source_path);
    }
    let covers_dir = app_data_dir.join("covers");
    fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;
    // 清理该游戏的旧封面文件（避免换扩展名后残留孤儿文件）
    let old_prefix = format!("cover_{}.", game_id);
    if let Ok(entries) = fs::read_dir(&covers_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let fname = entry.file_name().to_string_lossy().to_lowercase();
            if fname.starts_with(&old_prefix) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_lowercase();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let dest = covers_dir.join(format!("cover_{}_{}.{}", game_id, timestamp, ext));
    fs::copy(src, &dest).map_err(|e| e.to_string())?;
    // 清理缩略图缓存，否则网格视图会继续显示旧封面
    let thumb = app_data_dir.join("thumbnails").join(format!("game_{}_v2.jpg", game_id));
    let _ = fs::remove_file(&thumb);
    let stored = dest.to_string_lossy().to_string();
    state.db.update_game_cover(game_id, &stored).map_err(|e| e.to_string())?;
    Ok(stored)
}

// ==================== Cover Migration & Integrity ====================

#[tauri::command]
pub fn migrate_covers_to_internal(
    app: tauri::AppHandle,
    state: State<AppState>,
) -> Result<u32, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let games = state.db.get_all_games().map_err(|e| e.to_string())?;
    let mut count = 0u32;
    for game in &games {
        if game.cover_path.is_empty() { continue; }
        if crate::helpers::is_internal_cover(&game.cover_path, &app_data_dir) { continue; }
        if let Ok(new_path) = copy_cover_to_internal(&game.cover_path, game.id, &app_data_dir) {
            let _ = state.db.update_game_cover(game.id, &new_path);
            count += 1;
        }
    }
    Ok(count)
}

#[tauri::command]
pub fn check_cover_integrity(
    app: tauri::AppHandle,
    state: State<AppState>,
) -> Result<Vec<crate::models::CoverStatus>, String> {
    let _app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let games = state.db.get_all_games().map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    for game in &games {
        if game.cover_path.is_empty() {
            results.push(crate::models::CoverStatus {
                game_id: game.id,
                game_name: game.name.clone(),
                cover_path: String::new(),
                exists: false,
            });
            continue;
        }
        let exists = Path::new(&game.cover_path).exists();
        results.push(crate::models::CoverStatus {
            game_id: game.id,
            game_name: game.name.clone(),
            cover_path: game.cover_path.clone(),
            exists,
        });
    }
    Ok(results)
}

#[tauri::command]
pub fn rescan_game_cover(
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
    let internal_path = copy_cover_to_internal(&cover, game_id, &app_data_dir)
        .map_err(|e| e.to_string())?;
    // 封面已变化，清理缩略图缓存避免网格显示旧图
    let thumb = app_data_dir.join("thumbnails").join(format!("game_{}_v2.jpg", game_id));
    let _ = std::fs::remove_file(&thumb);
    state.db.update_game_cover(game_id, &internal_path).map_err(|e| e.to_string())?;
    Ok(internal_path)
}

// ==================== Integrity Checkup ====================

/// 数据完整性体检：检测封面/可执行文件/安装目录/存档路径失效，以及孤儿封面文件
#[tauri::command]
pub async fn run_integrity_checkup(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<IntegrityReport, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || run_integrity_checkup_impl(&db, &app_data_dir))
        .await
        .map_err(|e| format!("任务执行失败: {}", e))?
}

fn run_integrity_checkup_impl(db: &crate::db::Database, app_data_dir: &Path) -> Result<IntegrityReport, String> {
    let games = db.get_all_games().map_err(|e| e.to_string())?;
    let mut issues = Vec::new();
    let mut referenced_covers = std::collections::HashSet::new();

    for game in &games {
        let mut push_issue = |issue_type: &str, path: &str| {
            issues.push(IntegrityIssue {
                game_id: game.id,
                game_name: game.name.clone(),
                issue_type: issue_type.to_string(),
                path: path.to_string(),
            });
        };
        if !game.cover_path.is_empty() {
            referenced_covers.insert(game.cover_path.clone());
            if !Path::new(&game.cover_path).exists() {
                push_issue("missing_cover", &game.cover_path);
            }
        }
        if !game.exe_path.is_empty() && !Path::new(&game.exe_path).exists() {
            push_issue("missing_exe", &game.exe_path);
        }
        if !game.install_path.is_empty() && !Path::new(&game.install_path).exists() {
            push_issue("missing_install", &game.install_path);
        }
        if !game.save_path.is_empty() && !Path::new(&game.save_path).exists() {
            push_issue("missing_save", &game.save_path);
        }
    }

    // 检测重复游戏：多个游戏记录指向同一安装目录（归一化比较）
    let mut by_path: HashMap<String, Vec<&Game>> = HashMap::new();
    for game in &games {
        if !game.install_path.is_empty() {
            let key = game.install_path.replace('/', "\\").to_lowercase();
            by_path.entry(key).or_default().push(game);
        }
    }
    for group in by_path.values() {
        if group.len() > 1 {
            for game in group {
                issues.push(IntegrityIssue {
                    game_id: game.id,
                    game_name: game.name.clone(),
                    issue_type: "duplicate".to_string(),
                    path: game.install_path.clone(),
                });
            }
        }
    }

    // 扫描内部封面目录，找出未被任何游戏引用的孤儿文件
    let mut orphan_covers = Vec::new();
    let covers_dir = app_data_dir.join("covers");
    if covers_dir.exists() {
        if let Ok(entries) = fs::read_dir(&covers_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let p = entry.path();
                if p.is_file() {
                    let s = p.to_string_lossy().to_string();
                    if !referenced_covers.contains(&s) {
                        orphan_covers.push(s);
                    }
                }
            }
        }
    }

    Ok(IntegrityReport {
        total_games: games.len(),
        issues,
        orphan_covers,
    })
}

/// 删除指定的孤儿文件（封面等），返回成功删除的数量
#[tauri::command]
pub fn cleanup_orphan_files(paths: Vec<String>) -> Result<u32, String> {
    let mut count = 0u32;
    for p in &paths {
        if fs::remove_file(p).is_ok() {
            count += 1;
        }
    }
    Ok(count)
}

// ==================== Path Relocate（库路径重定位） ====================

/// 大小写不敏感的路径前缀剥离（Windows 盘符大小写可能不同），成功返回相对路径部分
fn strip_prefix_ci(base: &Path, p: &Path) -> Option<PathBuf> {
    if let Ok(rel) = p.strip_prefix(base) {
        return Some(rel.to_path_buf());
    }
    let bcomps: Vec<String> = base
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
        .collect();
    let pcomps: Vec<std::ffi::OsString> = p.components().map(|c| c.as_os_str().to_owned()).collect();
    if pcomps.len() <= bcomps.len() {
        return None;
    }
    for (i, b) in bcomps.iter().enumerate() {
        if pcomps[i].to_string_lossy().to_lowercase() != *b {
            return None;
        }
    }
    let mut rel = PathBuf::new();
    for c in &pcomps[bcomps.len()..] {
        rel.push(c);
    }
    Some(rel)
}

/// 执行单个游戏的重定位：install_path 换成新目录，exe/save/附加启动入口按相对路径重建
fn do_relocate(db: &crate::db::Database, game: &Game, new_install: &Path) -> Result<Game, String> {
    let old_install = Path::new(&game.install_path);

    // exe：优先相对路径重建，失败回退重新检测主程序
    let mut new_exe = String::new();
    if !game.exe_path.is_empty() {
        if let Some(rel) = strip_prefix_ci(old_install, Path::new(&game.exe_path)) {
            let cand = new_install.join(&rel);
            if cand.exists() {
                new_exe = cand.to_string_lossy().to_string();
            }
        }
    }
    if new_exe.is_empty() {
        let detected = find_main_exe(new_install);
        new_exe = detected.exe_path;
    }
    db.update_game_relocate(game.id, &new_install.to_string_lossy(), &new_exe)
        .map_err(|e| e.to_string())?;

    // save：位于旧安装目录下且新路径存在才重建，否则保留原值
    if !game.save_path.is_empty() {
        if let Some(rel) = strip_prefix_ci(old_install, Path::new(&game.save_path)) {
            let cand = new_install.join(&rel);
            if cand.exists() {
                db.update_game_save_path(game.id, &cand.to_string_lossy())
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    // 附加启动入口：同样按相对路径重建（仅新路径存在时采用）
    let actions = db.get_launch_actions(game.id).map_err(|e| e.to_string())?;
    if !actions.is_empty() {
        let mut changed = false;
        let remapped: Vec<LaunchAction> = actions
            .into_iter()
            .map(|mut a| {
                if let Some(rel) = strip_prefix_ci(old_install, Path::new(&a.program_path)) {
                    let cand = new_install.join(&rel);
                    if cand.exists() {
                        a.program_path = cand.to_string_lossy().to_string();
                        changed = true;
                    }
                }
                a
            })
            .collect();
        if changed {
            db.replace_launch_actions(game.id, &remapped)
                .map_err(|e| e.to_string())?;
        }
    }

    db.get_game_by_id(game.id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "游戏不存在".to_string())
}

/// 单个游戏重定位：用户手动指定新的安装目录
#[tauri::command]
pub async fn relocate_game(
    state: State<'_, AppState>,
    game_id: i64,
    new_install_path: String,
) -> Result<Game, String> {
    let new_install = PathBuf::from(new_install_path.trim().to_string());
    if !new_install.is_dir() {
        return Err("目标目录不存在".into());
    }
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let game = db
            .get_game_by_id(game_id)
            .map_err(|e| e.to_string())?
            .ok_or("游戏不存在")?;
        do_relocate(&db, &game, &new_install)
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

/// 批量重定位：按旧安装目录最后一级文件夹名，在用户选的新库根目录下匹配同名子文件夹
#[tauri::command]
pub async fn relocate_games_by_root(
    state: State<'_, AppState>,
    root_path: String,
) -> Result<RelocateReport, String> {
    let root = PathBuf::from(root_path.trim().to_string());
    if !root.is_dir() {
        return Err("所选库根目录不存在".into());
    }
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // 新根目录的直接子目录索引：文件夹名（小写）-> 路径列表
        let mut by_name: HashMap<String, Vec<PathBuf>> = HashMap::new();
        if let Ok(entries) = fs::read_dir(&root) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    by_name
                        .entry(e.file_name().to_string_lossy().to_lowercase())
                        .or_default()
                        .push(p);
                }
            }
        }
        let games = db.get_all_games().map_err(|e| e.to_string())?;
        let mut report = RelocateReport { fixed: 0, unmatched: Vec::new() };
        for game in &games {
            // 只处理安装目录确实失效的游戏
            if game.install_path.is_empty() || Path::new(&game.install_path).exists() {
                continue;
            }
            let leaf = match Path::new(&game.install_path).file_name() {
                Some(n) => n.to_string_lossy().to_lowercase(),
                None => {
                    report.unmatched.push(game.name.clone());
                    continue;
                }
            };
            match by_name.get(&leaf) {
                // 唯一命中同名子目录才重定位；同名冲突（多个）跳过不猜
                Some(matches) if matches.len() == 1 => {
                    do_relocate(&db, game, &matches[0])?;
                    report.fixed += 1;
                }
                _ => report.unmatched.push(game.name.clone()),
            }
        }
        Ok(report)
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
}
