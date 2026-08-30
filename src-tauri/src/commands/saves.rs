//! 存档备份 / 恢复相关命令
//!
//! 备份以目录形式存放：`{备份根目录}\{game_id}\{unix_ts}\`，
//! 目录内附 `meta.json` 记录备注、统计信息与是否自动备份。
//! 列表通过扫描目录重建，不引入新的数据表。

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use crate::helpers::{copy_dir_recursive, dir_stats};
use crate::models::{AppState, SaveBackupInfo};

/// 每游戏最多保留的备份份数
const MAX_BACKUPS_PER_GAME: usize = 10;

/// 与备份目录同存的元数据
#[derive(Serialize, Deserialize)]
struct SaveBackupMeta {
    created_at: i64,
    note: String,
    file_count: u64,
    size_bytes: u64,
    is_auto: bool,
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 解析备份根目录：优先用户配置的 save_backup_dir，否则用应用数据目录下的 save_backups
fn backup_root(app: &tauri::AppHandle, db: &crate::db::Database) -> Result<PathBuf, String> {
    let settings = db.get_settings().map_err(|e| e.to_string())?;
    if !settings.save_backup_dir.is_empty() {
        let p = PathBuf::from(&settings.save_backup_dir);
        fs::create_dir_all(&p).map_err(|e| format!("备份目录创建失败: {}", e))?;
        return Ok(p);
    }
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let p = app_data_dir.join("save_backups");
    fs::create_dir_all(&p).map_err(|e| e.to_string())?;
    Ok(p)
}

fn game_backups_dir(root: &Path, game_id: i64) -> PathBuf {
    root.join(game_id.to_string())
}

/// 校验备份 ID（时间戳目录名），防止路径穿越
fn validate_backup_id(backup_id: &str) -> Result<(), String> {
    if backup_id.parse::<i64>().is_err() {
        return Err("无效的备份标识".into());
    }
    Ok(())
}

fn load_meta(dir: &Path) -> Option<SaveBackupMeta> {
    let content = fs::read_to_string(dir.join("meta.json")).ok()?;
    serde_json::from_str(&content).ok()
}

fn collect_backups(root: &Path, game_id: i64) -> Vec<SaveBackupInfo> {
    let dir = game_backups_dir(root, game_id);
    let mut list = Vec::new();
    let Ok(entries) = fs::read_dir(&dir) else {
        return list;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(|s| s.to_string()) else {
            continue;
        };
        if name.parse::<i64>().is_err() {
            continue; // 非时间戳目录，跳过
        }
        let meta = load_meta(&path);
        list.push(SaveBackupInfo {
            id: name,
            game_id,
            created_at: meta.as_ref().map(|m| m.created_at).unwrap_or(0),
            note: meta.as_ref().map(|m| m.note.clone()).unwrap_or_default(),
            file_count: meta.as_ref().map(|m| m.file_count).unwrap_or(0),
            size_bytes: meta.as_ref().map(|m| m.size_bytes).unwrap_or(0),
            is_auto: meta.as_ref().map(|m| m.is_auto).unwrap_or(false),
        });
    }
    list.sort_by_key(|b| std::cmp::Reverse(b.created_at));
    list
}

/// 创建一份备份并写入元数据；超保留份数时删除最旧
fn create_backup(
    save_path: &str,
    root: &Path,
    game_id: i64,
    note: &str,
    is_auto: bool,
) -> Result<SaveBackupInfo, String> {
    let src = Path::new(save_path);
    if save_path.is_empty() || !src.is_dir() {
        return Err("存档目录不存在，请先在编辑页探测或填写存档路径".into());
    }
    let backups_dir = game_backups_dir(root, game_id);
    fs::create_dir_all(&backups_dir).map_err(|e| e.to_string())?;

    let mut ts = unix_now();
    let mut target = backups_dir.join(ts.to_string());
    // 同一秒内多次备份时递增，避免覆盖
    while target.exists() {
        ts += 1;
        target = backups_dir.join(ts.to_string());
    }

    copy_dir_recursive(src, &target).map_err(|e| format!("备份失败: {}", e))?;

    let (file_count, size_bytes) = dir_stats(&target);
    let meta = SaveBackupMeta {
        created_at: ts,
        note: note.to_string(),
        file_count,
        size_bytes,
        is_auto,
    };
    let meta_json = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    fs::write(target.join("meta.json"), meta_json).map_err(|e| e.to_string())?;

    // 保留策略：超限时从最旧开始删除
    let all = collect_backups(root, game_id);
    if all.len() > MAX_BACKUPS_PER_GAME {
        let mut oldest_first = all;
        oldest_first.sort_by_key(|a| a.created_at);
        for old in oldest_first.iter().take(oldest_first.len() - MAX_BACKUPS_PER_GAME) {
            let _ = fs::remove_dir_all(backups_dir.join(&old.id));
        }
    }

    Ok(SaveBackupInfo {
        id: ts.to_string(),
        game_id,
        created_at: ts,
        note: note.to_string(),
        file_count,
        size_bytes,
        is_auto,
    })
}

#[tauri::command]
pub async fn backup_game_save(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    game_id: i64,
    note: Option<String>,
) -> Result<SaveBackupInfo, String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let game = db
            .get_game_by_id(game_id)
            .map_err(|e| e.to_string())?
            .ok_or("游戏不存在")?;
        let root = backup_root(&app, &db)?;
        create_backup(&game.save_path, &root, game_id, note.as_deref().unwrap_or(""), false)
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

#[tauri::command]
pub fn list_save_backups(
    app: tauri::AppHandle,
    state: State<AppState>,
    game_id: i64,
) -> Result<Vec<SaveBackupInfo>, String> {
    let root = backup_root(&app, &state.db)?;
    Ok(collect_backups(&root, game_id))
}

#[tauri::command]
pub async fn restore_game_save(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    game_id: i64,
    backup_id: String,
) -> Result<(), String> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        validate_backup_id(&backup_id)?;
        let game = db
            .get_game_by_id(game_id)
            .map_err(|e| e.to_string())?
            .ok_or("游戏不存在")?;
        if game.save_path.is_empty() {
            return Err("存档路径未设置，请先在编辑页探测或填写存档路径".into());
        }
        let root = backup_root(&app, &db)?;
        let backup_dir = game_backups_dir(&root, game_id).join(&backup_id);
        if !backup_dir.is_dir() {
            return Err("备份不存在".into());
        }

        let save_dir = PathBuf::from(&game.save_path);
        // 当前存档目录存在时，先自动留一份安全备份兜底
        if save_dir.is_dir() {
            create_backup(&game.save_path, &root, game_id, "", true)?;
        }

        // 合并覆盖：同名文件覆盖，不删除备份中没有的文件
        copy_dir_recursive(&backup_dir, &save_dir).map_err(|e| format!("恢复失败: {}", e))?;
        // meta.json 是备份元数据，不应混入存档目录
        let _ = fs::remove_file(save_dir.join("meta.json"));
        Ok(())
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

#[tauri::command]
pub fn delete_save_backup(
    app: tauri::AppHandle,
    state: State<AppState>,
    game_id: i64,
    backup_id: String,
) -> Result<(), String> {
    validate_backup_id(&backup_id)?;
    let root = backup_root(&app, &state.db)?;
    let dir = game_backups_dir(&root, game_id).join(&backup_id);
    if !dir.is_dir() {
        return Err("备份不存在".into());
    }
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())
}
