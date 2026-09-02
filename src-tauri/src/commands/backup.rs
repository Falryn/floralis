//! 数据备份/导入、批量操作、解压、版本更新相关命令

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tauri::{Emitter, Manager, State};

use crate::helpers::{compare_versions, copy_cover_to_internal, find_cover_image, find_main_exe, find_save_directory};
use crate::models::{AppState, BackupData, ExtractResult, UpdateInfo};

// ==================== Extract ====================

#[tauri::command]
pub async fn extract_game(
    state: State<'_, AppState>,
    archive_path: String,
    dest_path: Option<String>,
    password: Option<String>,
) -> Result<ExtractResult, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        extract_game_impl(&db, archive_path, dest_path, password)
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

fn extract_game_impl(
    db: &crate::db::Database,
    archive_path: String,
    dest_path: Option<String>,
    password: Option<String>,
) -> Result<ExtractResult, String> {
    let settings = db.get_settings().map_err(|e| e.to_string())?;

    if settings.seven_zip_path.is_empty() {
        return Ok(ExtractResult {
            success: false,
            exe_path: String::new(),
            cover_path: String::new(),
            detected_name: String::new(),
            extract_dir: String::new(),
            save_path: String::new(),
            name_candidates: Vec::new(),
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
            name_candidates: Vec::new(),
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
        .arg(format!("-o{}", extract_dir.display()))
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
            name_candidates: Vec::new(),
            error: format!(
                "解压失败: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        });
    }

    let found = find_main_exe(&extract_dir);
    let cover_path = find_cover_image(&extract_dir);
    let save_path = find_save_directory(&found.save_hint, &extract_dir.to_string_lossy());

    Ok(ExtractResult {
        success: true,
        exe_path: found.exe_path,
        cover_path,
        detected_name: found.detected_name,
        extract_dir: extract_dir.to_string_lossy().to_string(),
        save_path,
        error: String::new(),
        name_candidates: found.name_candidates,
    })
}

// ==================== Batch Extract ====================

/// 批量解压多个压缩包，通过事件报告进度
///
/// 密码策略优化：先尝试无密码（大多数场景），失败后再尝试已保存密码
#[tauri::command]
pub async fn batch_extract_games(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    archive_paths: Vec<String>,
    dest_path: Option<String>,
    passwords: Vec<String>,
) -> Result<Vec<ExtractResult>, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        batch_extract_games_impl(&app, &db, archive_paths, dest_path, passwords)
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

fn batch_extract_games_impl(
    app: &tauri::AppHandle,
    db: &crate::db::Database,
    archive_paths: Vec<String>,
    dest_path: Option<String>,
    passwords: Vec<String>,
) -> Result<Vec<ExtractResult>, String> {
    let settings = db.get_settings().map_err(|e| e.to_string())?;

    if settings.seven_zip_path.is_empty() {
        return Err("请先在设置中配置7z路径".into());
    }

    let dest = if let Some(d) = dest_path {
        PathBuf::from(d)
    } else if !settings.default_extract_path.is_empty() {
        PathBuf::from(&settings.default_extract_path)
    } else {
        return Err("未指定解压路径".into());
    };

    let total = archive_paths.len();
    let mut results = Vec::with_capacity(total);

    for (idx, archive_path) in archive_paths.iter().enumerate() {
        let file_name = Path::new(archive_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // 发送进度事件
        let _ = app.emit("extract-progress", serde_json::json!({
            "current": idx + 1,
            "total": total,
            "name": file_name,
        }));

        let archive_stem = Path::new(archive_path)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let extract_dir = dest.join(&archive_stem);
        fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;

        // 密码策略：先无密码，再逐个尝试已保存密码
        let mut password_list: Vec<Option<String>> = vec![None];
        for pwd in &passwords {
            password_list.push(Some(pwd.clone()));
        }

        let mut result: Option<ExtractResult> = None;
        for pwd in &password_list {
            let mut cmd = Command::new(&settings.seven_zip_path);
            cmd.arg("x")
                .arg("-y")
                .arg("-bsp0")  // 静默模式，减少输出开销
                .arg(format!("-o{}", extract_dir.display()))
                .arg(archive_path);
            if let Some(p) = pwd {
                cmd.arg(format!("-p{}", p));
            }

            let output = match cmd.output() {
                Ok(o) => o,
                Err(e) => {
                    result = Some(ExtractResult {
                        success: false,
                        exe_path: String::new(),
                        cover_path: String::new(),
                        detected_name: String::new(),
                        extract_dir: String::new(),
                        save_path: String::new(),
                        name_candidates: Vec::new(),
                        error: format!("7z执行失败: {}", e),
                    });
                    break;
                }
            };

            if output.status.success() {
                let found = find_main_exe(&extract_dir);
                let cover_path = find_cover_image(&extract_dir);
                let save_path = find_save_directory(&found.save_hint, &extract_dir.to_string_lossy());
                result = Some(ExtractResult {
                    success: true,
                    exe_path: found.exe_path,
                    cover_path,
                    detected_name: found.detected_name,
                    extract_dir: extract_dir.to_string_lossy().to_string(),
                    save_path,
                    error: String::new(),
                    name_candidates: found.name_candidates,
                });
                break;
            }
            // 密码错误时继续尝试下一个
        }

        results.push(result.unwrap_or(ExtractResult {
            success: false,
            exe_path: String::new(),
            cover_path: String::new(),
            detected_name: String::new(),
            extract_dir: String::new(),
            save_path: String::new(),
            name_candidates: Vec::new(),
            error: "所有密码均失败".into(),
        }));
    }

    Ok(results)
}

// ==================== Batch Operations ====================

#[tauri::command]
pub fn batch_delete_games(state: State<AppState>, ids: Vec<i64>) -> Result<(), String> {
    state.db.batch_delete_games(&ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_set_game_group(
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
pub fn batch_set_game_status(
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
pub fn batch_set_game_rating(
    state: State<AppState>,
    game_ids: Vec<i64>,
    rating: i64,
) -> Result<(), String> {
    state
        .db
        .batch_set_game_rating(&game_ids, rating)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_set_game_favorite(
    state: State<AppState>,
    game_ids: Vec<i64>,
    favorite: bool,
) -> Result<(), String> {
    state
        .db
        .batch_set_game_favorite(&game_ids, favorite)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn batch_scan_covers(app: tauri::AppHandle, state: State<'_, AppState>, game_ids: Vec<i64>) -> Result<u32, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        batch_scan_covers_impl(&app, &db, &app_data_dir, game_ids)
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

fn batch_scan_covers_impl(app: &tauri::AppHandle, db: &crate::db::Database, app_data_dir: &Path, game_ids: Vec<i64>) -> Result<u32, String> {
    let total = game_ids.len();
    let mut count = 0u32;
    for (idx, gid) in game_ids.iter().enumerate() {
        let _ = app.emit("scan-covers-progress", serde_json::json!({ "current": idx + 1, "total": total }));
        let game = match db.get_game_by_id(*gid) {
            Ok(Some(g)) => g,
            _ => continue,
        };
        if game.install_path.is_empty() {
            continue;
        }
        let cover = find_cover_image(Path::new(&game.install_path));
        if !cover.is_empty() {
            match copy_cover_to_internal(&cover, game.id, app_data_dir) {
                Ok(internal_path) => {
                    let _ = db.update_game_cover(game.id, &internal_path);
                }
                Err(_) => {
                    let _ = db.update_game_cover(game.id, &cover);
                }
            }
            count += 1;
        }
    }
    Ok(count)
}

// ==================== Data Export / Import ====================

#[tauri::command]
pub fn export_data(state: State<AppState>) -> Result<String, String> {
    let games = state.db.get_all_games().map_err(|e| e.to_string())?;
    let groups = state.db.get_all_groups().map_err(|e| e.to_string())?;
    let settings = state.db.get_settings().map_err(|e| e.to_string())?;
    let passwords = state.db.get_passwords().map_err(|e| e.to_string())?;
    let mods = state.db.get_all_mods().map_err(|e| e.to_string())?;
    let mut mod_tags: Vec<(i64, i64)> = Vec::new();
    for m in &mods {
        let tags = state.db.get_mod_tags(m.id).map_err(|e| e.to_string())?;
        for t in &tags {
            mod_tags.push((m.id, t.id));
        }
    }
    let backup = BackupData {
        version: 2,
        games,
        groups,
        settings,
        passwords,
        mods,
        mod_tags,
    };
    serde_json::to_string_pretty(&backup).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_data(state: State<AppState>, json: String) -> Result<(), String> {
    let backup: BackupData = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let db = &state.db;

    db.clear_all().map_err(|e| e.to_string())?;

    // Restore groups
    for g in &backup.groups {
        db.add_group(&g.name).map_err(|e| e.to_string())?;
    }
    let new_groups = db.get_all_groups().map_err(|e| e.to_string())?;
    let mut name_to_new_id: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for ng in &new_groups {
        name_to_new_id.insert(ng.name.clone(), ng.id);
    }
    let mut ordered_ids: Vec<i64> = Vec::new();
    for g in &backup.groups {
        if let Some(&new_id) = name_to_new_id.get(&g.name) {
            ordered_ids.push(new_id);
        }
    }
    if !ordered_ids.is_empty() {
        db.reorder_groups(&ordered_ids).map_err(|e| e.to_string())?;
    }

    // Restore games
    let old_groups = &backup.groups;
    let mut old_id_to_new_id: std::collections::HashMap<i64, i64> = std::collections::HashMap::new();
    for og in old_groups {
        if let Some(&new_id) = name_to_new_id.get(&og.name) {
            old_id_to_new_id.insert(og.id, new_id);
        }
    }
    for game in &backup.games {
        let new_group_id = game.group_id.and_then(|old_id| old_id_to_new_id.get(&old_id).copied());
        let new_id = db.add_game(
            &game.name, new_group_id, &game.install_path, &game.exe_path,
            &game.launch_args, &game.cover_path, &game.save_path, &game.notes,
            &game.script_path, &game.script_args,
        )
        .map_err(|e| e.to_string())?;
        if game.is_favorite {
            db.set_game_favorite(new_id, true).map_err(|e| e.to_string())?;
        }
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

    // Restore mods
    let mut old_mod_id_to_new_id: std::collections::HashMap<i64, i64> = std::collections::HashMap::new();
    for m in &backup.mods {
        let new_game_id = if let Some(old_gid) = m.game_id {
            backup.games.iter().find(|g| g.id == old_gid).and_then(|g| {
                db.get_all_games().ok().and_then(|games| {
                    games.iter().find(|ng| ng.install_path == g.install_path && ng.name == g.name).map(|ng| ng.id)
                })
            })
        } else {
            None
        };
        let new_id = db.add_mod(
            &m.name, &m.description, &m.mod_path, &m.install_path,
            new_game_id, &m.game_dir, &m.version, &m.author,
            m.is_enabled, m.sort_order, &m.category, &m.source_url,
            &m.cover_path, &m.mod_type, &m.original_name,
        ).map_err(|e| e.to_string())?;
        old_mod_id_to_new_id.insert(m.id, new_id);
    }
    // Restore mod_tags
    let mut mod_tag_map: std::collections::HashMap<i64, Vec<i64>> = std::collections::HashMap::new();
    for (old_mod_id, tag_id) in &backup.mod_tags {
        if let Some(&new_mod_id) = old_mod_id_to_new_id.get(old_mod_id) {
            mod_tag_map.entry(new_mod_id).or_default().push(*tag_id);
        }
    }
    for (new_mod_id, tag_ids) in &mod_tag_map {
        db.set_mod_tags(*new_mod_id, tag_ids).map_err(|e| e.to_string())?;
    }

    Ok(())
}

// ==================== Database Backup ====================

/// 备份核心逻辑：WAL 刷盘后拷贝数据库文件，仅保留最近 5 份
fn perform_db_backup(app: &tauri::AppHandle, state: &AppState) -> Result<String, String> {
    use std::time::SystemTime;

    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let backup_dir = app_data_dir.join("backups");
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();

    let backup_path = backup_dir.join(format!("floralis_backup_{}.db", timestamp));

    // WAL 模式下未 checkpoint 的数据仍在 -wal 文件中，先刷回主文件再拷贝
    state.db.checkpoint_wal().map_err(|e| e.to_string())?;
    let db_path = state.db.get_db_path();
    fs::copy(&db_path, &backup_path).map_err(|e| format!("备份失败: {}", e))?;

    // Clean up old backups (keep only last 5)
    let mut backups: Vec<_> = fs::read_dir(&backup_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "db"))
        .collect();

    backups.sort_by_key(|e| std::cmp::Reverse(e.metadata().ok().and_then(|m| m.modified().ok()).unwrap_or(SystemTime::UNIX_EPOCH)));

    for old_backup in backups.into_iter().skip(5) {
        let _ = fs::remove_file(old_backup.path());
    }

    Ok(backup_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn backup_database(app: tauri::AppHandle, state: State<AppState>) -> Result<String, String> {
    perform_db_backup(&app, &state)
}

/// 启动时静默执行的每日自动备份：
/// - 设置 auto_backup 关闭时直接跳过（默认开启）
/// - 距上次自动备份不足 24 小时时跳过（时间戳存于 settings.last_auto_backup）
/// - 成功返回备份路径，跳过时返回 None；失败仅记录日志不打断启动
#[tauri::command]
pub fn run_auto_backup(app: tauri::AppHandle, state: State<AppState>) -> Result<Option<String>, String> {
    use std::time::SystemTime;

    let settings = state.db.get_settings().map_err(|e| e.to_string())?;
    if settings.auto_backup != "true" {
        return Ok(None);
    }

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let last: u64 = state.db.get_setting("last_auto_backup").map_err(|e| e.to_string())?
        .parse()
        .unwrap_or(0);
    const DAY_SECS: u64 = 24 * 3600;
    if now.saturating_sub(last) < DAY_SECS {
        return Ok(None);
    }

    let path = perform_db_backup(&app, &state)?;
    state.db.save_setting("last_auto_backup", &now.to_string()).map_err(|e| e.to_string())?;
    Ok(Some(path))
}

// ==================== Update Check ====================

#[tauri::command]
pub fn check_for_update(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    let current_version = app.config().version.clone()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "0.0.0".to_string());

    let settings = app.state::<AppState>().db.get_settings().map_err(|e| e.to_string())?;
    let repo = if settings.update_repo.is_empty() {
        "Falryn/floralis"
    } else {
        &settings.update_repo
    };

    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);

    let resp: Result<ureq::Response, ureq::Error> = crate::helpers::build_http_agent()
        .get(&url)
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
        Err(e) => Err(crate::helpers::friendly_http_error("更新检查", &e))
    }
}
