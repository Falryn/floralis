//! Mod 管理相关命令

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{Emitter, Manager, State};

use crate::helpers::copy_file_to_appdata;
use crate::models::{AppState, ScannedMod};
use crate::db::{Mod, ModProfile, Tag};

// ==================== Mod CRUD ====================

#[tauri::command]
pub fn get_all_mods(state: State<AppState>) -> Result<Vec<Mod>, String> {
    state.db.get_all_mods().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_mods_by_game(state: State<AppState>, game_id: i64) -> Result<Vec<Mod>, String> {
    state.db.get_mods_by_game(game_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_mods_by_game_dir(state: State<AppState>, game_dir: String) -> Result<Vec<Mod>, String> {
    state.db.get_mods_by_game_dir(&game_dir).map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn add_mod(
    state: State<AppState>,
    name: String,
    description: String,
    mod_path: String,
    install_path: String,
    game_id: Option<i64>,
    game_dir: String,
    version: String,
    author: String,
    is_enabled: bool,
    sort_order: i32,
    category: String,
    source_url: String,
    cover_path: String,
    mod_type: String,
    original_name: String,
) -> Result<i64, String> {
    state
        .db
        .add_mod(&name, &description, &mod_path, &install_path, game_id, &game_dir, &version, &author, is_enabled, sort_order, &category, &source_url, &cover_path, &mod_type, &original_name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_mod(
    state: State<AppState>,
    id: i64,
    name: String,
    description: String,
    mod_path: String,
    install_path: String,
    game_id: Option<i64>,
    game_dir: String,
    version: String,
    author: String,
    is_enabled: bool,
    sort_order: i32,
    category: String,
    source_url: String,
    cover_path: String,
    mod_type: String,
    original_name: String,
) -> Result<(), String> {
    state
        .db
        .update_mod(id, &name, &description, &mod_path, &install_path, game_id, &game_dir, &version, &author, is_enabled, sort_order, &category, &source_url, &cover_path, &mod_type, &original_name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_mod(state: State<AppState>, id: i64) -> Result<(), String> {
    state.db.delete_mod(id).map_err(|e| e.to_string())
}

// ==================== Enable / Disable ====================

#[tauri::command]
pub fn toggle_mod_enabled(state: State<AppState>, id: i64) -> Result<(), String> {
    let mod_item = state.db.get_mod_by_id(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Mod 不存在".to_string())?;
    set_mod_enabled_state(&state, &mod_item, !mod_item.is_enabled)
}

/// 将单个 mod 置为指定的启用/禁用状态（含文件重命名），幂等
fn set_mod_enabled_state(state: &State<AppState>, mod_item: &Mod, target: bool) -> Result<(), String> {
    let id = mod_item.id;
    let current_path = PathBuf::from(&mod_item.mod_path);

    if !target {
        // 禁用：加 .off 后缀
        let off_path = PathBuf::from(format!("{}.off", mod_item.mod_path));
        if current_path.exists() {
            fs::rename(&current_path, &off_path)
                .map_err(|e| format!("禁用失败，无法重命名: {}", e))?;
            state.db.update_mod_path(id, &off_path.to_string_lossy()).map_err(|e| e.to_string())?;
        }
        state.db.set_mod_enabled(id, false).map_err(|e| e.to_string())?;
    } else {
        // 启用：去掉 .off，按公式计算正确文件名
        let stripped = mod_item.mod_path.strip_suffix(".off").unwrap_or(&mod_item.mod_path);
        let off_path = PathBuf::from(&mod_item.mod_path);

        let target_name = compute_active_filename(state, mod_item)?;
        let parent = PathBuf::from(stripped).parent().unwrap_or(Path::new("")).to_path_buf();
        let target_path = parent.join(&target_name);

        if off_path.exists() {
            if target_path.exists() && target_path != off_path {
                return Err(format!("启用失败，目标文件已存在: {}", target_name));
            }
            fs::rename(&off_path, &target_path)
                .map_err(|e| format!("启用失败，无法恢复文件: {}", e))?;
            state.db.update_mod_path(id, &target_path.to_string_lossy()).map_err(|e| e.to_string())?;
        } else if current_path.exists() {
            if target_path != current_path {
                fs::rename(&current_path, &target_path)
                    .map_err(|e| format!("启用失败: {}", e))?;
            }
            state.db.update_mod_path(id, &target_path.to_string_lossy()).map_err(|e| e.to_string())?;
        }
        state.db.set_mod_enabled(id, true).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 根据游戏配置计算 mod 的活跃文件名
fn compute_active_filename(state: &State<AppState>, mod_item: &Mod) -> Result<String, String> {
    let original = if mod_item.original_name.is_empty() {
        PathBuf::from(mod_item.mod_path.strip_suffix(".off").unwrap_or(&mod_item.mod_path))
            .file_name().unwrap_or_default().to_string_lossy().to_string()
    } else {
        mod_item.original_name.clone()
    };

    let game = match mod_item.game_id {
        Some(gid) => state.db.get_game_by_id(gid).map_err(|e| e.to_string())?,
        None => return Ok(original),
    };
    let game = match game {
        Some(g) => g,
        None => return Ok(original),
    };

    if game.mod_naming_pattern.is_empty() && !game.mod_uses_load_order {
        return Ok(original);
    }

    let is_folder = mod_item.mod_type == "folder";

    if is_folder {
        let stem = original.trim_end_matches(".off");
        let named = if game.mod_naming_pattern.is_empty() {
            stem.to_string()
        } else {
            game.mod_naming_pattern.replace("{name}", stem)
        };
        if game.mod_uses_load_order {
            Ok(format!("{:03}_{}", mod_item.sort_order, named))
        } else {
            Ok(named)
        }
    } else {
        let p = PathBuf::from(&original);
        let stem = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();

        let named = if game.mod_naming_pattern.is_empty() {
            format!("{}{}", stem, ext)
        } else {
            format!("{}{}", game.mod_naming_pattern.replace("{name}", &stem), ext)
        };

        if game.mod_uses_load_order {
            Ok(format!("{:03}_{}", mod_item.sort_order, named))
        } else {
            Ok(named)
        }
    }
}

// ==================== Integrity Check ====================

/// 检查所有 mod 文件是否存在，返回缺失文件的 mod id 列表
#[tauri::command]
pub fn check_mods_integrity(state: State<AppState>) -> Result<Vec<i64>, String> {
    let mods = state.db.get_all_mods().map_err(|e| e.to_string())?;
    let missing = mods
        .iter()
        .filter(|m| m.mod_path.is_empty() || !Path::new(&m.mod_path).exists())
        .map(|m| m.id)
        .collect();
    Ok(missing)
}

// ==================== Mod Profiles ====================

/// 应用配置文件的执行结果
#[derive(Debug, Clone, Serialize)]
pub struct ApplyProfileResult {
    /// 实际变更启用状态的 mod 数量
    pub changed: i32,
    /// 失败的 mod 名称及原因
    pub failures: Vec<String>,
}

#[tauri::command]
pub fn get_mod_profiles(state: State<AppState>, game_id: i64) -> Result<Vec<ModProfile>, String> {
    state.db.get_profiles_by_game(game_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_mod_profile(
    state: State<AppState>,
    game_id: i64,
    name: String,
    mod_ids: Vec<i64>,
) -> Result<i64, String> {
    state.db.create_mod_profile(game_id, &name, &mod_ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_mod_profile(state: State<AppState>, id: i64, name: String) -> Result<(), String> {
    state.db.rename_mod_profile(id, &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_mod_profile(state: State<AppState>, id: i64) -> Result<(), String> {
    state.db.delete_mod_profile(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_mod_profile_mods(state: State<AppState>, profile_id: i64, mod_ids: Vec<i64>) -> Result<(), String> {
    state.db.set_profile_mods(profile_id, &mod_ids).map_err(|e| e.to_string())
}

/// 应用配置文件：将该游戏的 mod 启用状态调整为配置文件记录的组合
#[tauri::command]
pub fn apply_mod_profile(state: State<AppState>, profile_id: i64) -> Result<ApplyProfileResult, String> {
    let profile = state.db.get_profile_by_id(profile_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "配置文件不存在".to_string())?;

    let enabled_set: std::collections::HashSet<i64> = profile.mod_ids.iter().copied().collect();
    let game_mods = state.db.get_mods_by_game(profile.game_id).map_err(|e| e.to_string())?;

    let mut changed = 0i32;
    let mut failures: Vec<String> = Vec::new();

    // 先禁用不在组合中的 mod，释放文件名；再启用组合内的 mod
    for pass in [false, true] {
        for mod_item in &game_mods {
            let target = enabled_set.contains(&mod_item.id);
            if target != pass || mod_item.is_enabled == target {
                continue;
            }
            match set_mod_enabled_state(&state, mod_item, target) {
                Ok(()) => changed += 1,
                Err(e) => failures.push(format!("{}: {}", mod_item.name, e)),
            }
        }
    }

    Ok(ApplyProfileResult { changed, failures })
}

// ==================== Reorder ====================

#[tauri::command]
pub fn reorder_mods(state: State<AppState>, mod_ids: Vec<i64>) -> Result<(), String> {
    state.db.reorder_mods(&mod_ids).map_err(|e| e.to_string())?;

    for (i, id) in mod_ids.iter().enumerate() {
        let mod_item = match state.db.get_mod_by_id(*id).map_err(|e| e.to_string())? {
            Some(m) => m,
            None => continue,
        };
        if !mod_item.is_enabled {
            continue;
        }
        let needs_rename = match mod_item.game_id {
            Some(gid) => {
                state.db.get_game_by_id(gid).map_err(|e| e.to_string())?
                    .is_some_and(|g| g.mod_uses_load_order)
            }
            None => false,
        };
        if !needs_rename {
            continue;
        }
        let mut updated_mod = mod_item.clone();
        updated_mod.sort_order = i as i32;
        let target_name = compute_active_filename(&state, &updated_mod)?;
        let current = PathBuf::from(&mod_item.mod_path);
        let parent = current.parent().unwrap_or(Path::new("")).to_path_buf();
        let target = parent.join(&target_name);
        if current != target && current.exists() {
            if target.exists() {
                return Err(format!("调序失败，目标文件已存在: {}", target_name));
            }
            fs::rename(&current, &target)
                .map_err(|e| format!("调序重命名失败: {}", e))?;
            state.db.update_mod_path(*id, &target.to_string_lossy()).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// ==================== Association ====================

#[tauri::command]
pub fn link_mod_to_game(state: State<AppState>, mod_id: i64, game_id: i64) -> Result<(), String> {
    state.db.link_mod_to_game(mod_id, game_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn unlink_mod_from_game(state: State<AppState>, mod_id: i64) -> Result<(), String> {
    state.db.unlink_mod_from_game(mod_id).map_err(|e| e.to_string())
}

// ==================== Mod Tags ====================

#[tauri::command]
pub fn get_mod_tags(state: State<AppState>, mod_id: i64) -> Result<Vec<Tag>, String> {
    state.db.get_mod_tags(mod_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_all_mod_tags(state: State<AppState>) -> Result<HashMap<i64, Vec<Tag>>, String> {
    state.db.get_all_mod_tags().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_mod_tags(state: State<AppState>, mod_id: i64, tag_ids: Vec<i64>) -> Result<(), String> {
    state.db.set_mod_tags(mod_id, &tag_ids).map_err(|e| e.to_string())
}

// ==================== Mod Cover ====================

#[tauri::command]
pub fn copy_mod_cover_to_storage(
    app: tauri::AppHandle,
    source_path: String,
    mod_id: Option<i64>,
) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    copy_file_to_appdata(&source_path, "mod_covers", "mod_cover", mod_id, &app_data_dir)
}

// ==================== Scan & Import ====================

/// 递归收集目录下所有 .pak 文件（跳过 .off 后缀）
fn collect_pak_files(dir: &Path, results: &mut Vec<ScannedMod>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            collect_pak_files(&path, results);
        } else if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("pak")) {
            let name_str = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name_str.ends_with(".off") {
                continue;
            }
            let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
            results.push(ScannedMod {
                name,
                path: path.to_string_lossy().to_string(),
                mod_type: "file".to_string(),
            });
        }
    }
}

/// 收集目录下的子文件夹作为文件夹型 mod
fn collect_folder_mods(dir: &Path, results: &mut Vec<ScannedMod>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            let folder_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if folder_name.ends_with(".off") {
                continue;
            }
            results.push(ScannedMod {
                name: folder_name.clone(),
                path: path.to_string_lossy().to_string(),
                mod_type: "folder".to_string(),
            });
        }
    }
}

/// 递归查找名为 "mods" 的文件夹
fn find_mods_dirs(dir: &Path, results: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            let folder_name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
            if folder_name == "mods" || folder_name == "mod" {
                results.push(path.clone());
            }
            find_mods_dirs(&path, results);
        }
    }
}

#[tauri::command]
pub async fn scan_mod_directory(app: tauri::AppHandle, dir_path: String, scan_mode: Option<String>) -> Result<Vec<ScannedMod>, String> {
    let dir = PathBuf::from(&dir_path);
    if !dir.exists() || !dir.is_dir() {
        return Err("目录不存在".into());
    }

    let mode = scan_mode.unwrap_or_else(|| "file".to_string());

    let _ = app.emit("mod-scan-progress", serde_json::json!({
        "current": 0,
        "total": 1,
        "name": "正在扫描...",
    }));

    let mut mods_dirs: Vec<PathBuf> = Vec::new();
    find_mods_dirs(&dir, &mut mods_dirs);

    let mut found_mods: Vec<ScannedMod> = Vec::new();
    let scan_targets: Vec<&Path> = if mods_dirs.is_empty() {
        vec![dir.as_path()]
    } else {
        mods_dirs.iter().map(|p| p.as_path()).collect()
    };

    for target in &scan_targets {
        if mode == "folder" {
            collect_folder_mods(target, &mut found_mods);
        } else {
            collect_pak_files(target, &mut found_mods);
        }
    }

    found_mods.sort_by(|a, b| a.path.cmp(&b.path));
    found_mods.dedup_by(|a, b| a.path == b.path);

    let _ = app.emit("mod-scan-progress", serde_json::json!({
        "current": 1,
        "total": 1,
        "name": format!("找到 {} 个 Mod", found_mods.len()),
    }));

    Ok(found_mods)
}

#[tauri::command]
pub fn extract_mod_files(
    state: State<AppState>,
    archive_paths: Vec<String>,
    dest_dir: String,
) -> Result<Vec<ScannedMod>, String> {
    use std::process::Command;

    let settings = state.db.get_settings().map_err(|e| e.to_string())?;
    if settings.seven_zip_path.is_empty() {
        return Err("请先在设置中配置 7z 路径".into());
    }

    let dest = PathBuf::from(&dest_dir);
    fs::create_dir_all(&dest).map_err(|e| format!("无法创建目标目录: {}", e))?;

    for archive in &archive_paths {
        let mut cmd = Command::new(&settings.seven_zip_path);
        cmd.arg("x")
            .arg("-y")
            .arg(format!("-o{}", dest.display()))
            .arg(archive);
        let output = cmd.output().map_err(|e| format!("7z 执行失败: {}", e))?;
        if !output.status.success() {
            return Err(format!(
                "解压 {} 失败: {}",
                archive,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    let mut pak_files: Vec<ScannedMod> = Vec::new();
    collect_pak_files(&dest, &mut pak_files);
    Ok(pak_files)
}

#[tauri::command]
pub fn copy_mod_files(
    file_paths: Vec<String>,
    dest_dir: String,
) -> Result<Vec<ScannedMod>, String> {
    let dest = PathBuf::from(&dest_dir);
    fs::create_dir_all(&dest).map_err(|e| format!("无法创建目标目录: {}", e))?;

    let mut results: Vec<ScannedMod> = Vec::new();
    for src in &file_paths {
        let src_path = PathBuf::from(src);
        let file_name = src_path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let dest_path = dest.join(&file_name);
        fs::copy(&src_path, &dest_path)
            .map_err(|e| format!("复制 {} 失败: {}", file_name, e))?;
        let name = src_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        results.push(ScannedMod {
            name,
            path: dest_path.to_string_lossy().to_string(),
            mod_type: "file".to_string(),
        });
    }
    Ok(results)
}
