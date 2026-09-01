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

/// 恢复核心逻辑（可单测）：校验 → 安全备份兜底 → 合并覆盖 → 清理 meta
fn restore_core(root: &Path, game_id: i64, backup_id: &str, save_path: &str) -> Result<(), String> {
    validate_backup_id(backup_id)?;
    if save_path.is_empty() {
        return Err("存档路径未设置，请先在编辑页探测或填写存档路径".into());
    }
    let backup_dir = game_backups_dir(root, game_id).join(backup_id);
    if !backup_dir.is_dir() {
        return Err("备份不存在".into());
    }

    let save_dir = PathBuf::from(save_path);
    // 当前存档目录存在时，先自动留一份安全备份兜底
    if save_dir.is_dir() {
        create_backup(save_path, root, game_id, "", true)?;
    }

    // 合并覆盖：同名文件覆盖，不删除备份中没有的文件
    copy_dir_recursive(&backup_dir, &save_dir).map_err(|e| format!("恢复失败: {}", e))?;
    // meta.json 是备份元数据，不应混入存档目录
    let _ = fs::remove_file(save_dir.join("meta.json"));
    Ok(())
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
        let game = db
            .get_game_by_id(game_id)
            .map_err(|e| e.to_string())?
            .ok_or("游戏不存在")?;
        let root = backup_root(&app, &db)?;
        restore_core(&root, game_id, &backup_id, &game.save_path)
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

/// 删除备份核心逻辑（可单测）
fn delete_backup_core(root: &Path, game_id: i64, backup_id: &str) -> Result<(), String> {
    validate_backup_id(backup_id)?;
    let dir = game_backups_dir(root, game_id).join(backup_id);
    if !dir.is_dir() {
        return Err("备份不存在".into());
    }
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_save_backup(
    app: tauri::AppHandle,
    state: State<AppState>,
    game_id: i64,
    backup_id: String,
) -> Result<(), String> {
    let root = backup_root(&app, &state.db)?;
    delete_backup_core(&root, game_id, &backup_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static SEQ: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "floralis_saves_test_{}_{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup_dir(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    /// 造一个含嵌套子目录的存档目录
    fn make_save_dir(base: &Path) -> PathBuf {
        let save = base.join("save");
        fs::create_dir_all(save.join("sub")).unwrap();
        fs::write(save.join("a.sav"), b"save-a").unwrap();
        fs::write(save.join("sub/b.sav"), b"save-b").unwrap();
        save
    }

    #[test]
    fn test_validate_backup_id_rejects_traversal() {
        assert!(validate_backup_id("../etc").is_err());
        assert!(validate_backup_id("123abc").is_err());
        assert!(validate_backup_id("").is_err());
        assert!(validate_backup_id("1725200000").is_ok());
    }

    #[test]
    fn test_create_backup_requires_existing_save_dir() {
        let root = temp_dir();
        let err = create_backup("", &root, 1, "", false).unwrap_err();
        assert!(err.contains("存档目录不存在"));
        let err = create_backup("/no/such/dir", &root, 1, "", false).unwrap_err();
        assert!(err.contains("存档目录不存在"));
        cleanup_dir(&root);
    }

    #[test]
    fn test_create_backup_copies_nested_files_with_meta() {
        let base = temp_dir();
        let save = make_save_dir(&base);
        let root = base.join("backups");

        let info = create_backup(&save.to_string_lossy(), &root, 7, "手动", false).unwrap();
        assert_eq!(info.game_id, 7);
        assert_eq!(info.note, "手动");
        assert!(!info.is_auto);
        assert_eq!(info.file_count, 2);
        assert_eq!(info.size_bytes, 12);

        let backup_dir = game_backups_dir(&root, 7).join(&info.id);
        assert_eq!(fs::read(backup_dir.join("a.sav")).unwrap(), b"save-a");
        assert_eq!(fs::read(backup_dir.join("sub/b.sav")).unwrap(), b"save-b");
        assert!(backup_dir.join("meta.json").exists());
        cleanup_dir(&base);
    }

    #[test]
    fn test_create_backup_same_second_does_not_overwrite() {
        let base = temp_dir();
        let save = make_save_dir(&base);
        let root = base.join("backups");

        let first = create_backup(&save.to_string_lossy(), &root, 1, "", false).unwrap();
        let second = create_backup(&save.to_string_lossy(), &root, 1, "", false).unwrap();
        assert_ne!(first.id, second.id);
        assert!(game_backups_dir(&root, 1).join(&first.id).is_dir());
        assert!(game_backups_dir(&root, 1).join(&second.id).is_dir());
        cleanup_dir(&base);
    }

    #[test]
    fn test_retention_keeps_at_most_max_backups() {
        let base = temp_dir();
        let save = make_save_dir(&base);
        let root = base.join("backups");

        let mut ids = Vec::new();
        for _ in 0..=MAX_BACKUPS_PER_GAME {
            let info = create_backup(&save.to_string_lossy(), &root, 1, "", false).unwrap();
            ids.push(info);
        }
        let remaining = collect_backups(&root, 1);
        assert_eq!(remaining.len(), MAX_BACKUPS_PER_GAME);
        // 最旧的一份应被淘汰
        assert!(!game_backups_dir(&root, 1).join(&ids[0].id).is_dir());
        assert!(game_backups_dir(&root, 1).join(ids.last().unwrap().id.as_str()).is_dir());
        cleanup_dir(&base);
    }

    #[test]
    fn test_collect_backups_skips_junk_and_tolerates_missing_meta() {
        let base = temp_dir();
        let root = base.join("backups");
        let game_dir = game_backups_dir(&root, 1);
        fs::create_dir_all(&game_dir).unwrap();

        // 合法备份但缺 meta.json
        fs::create_dir_all(game_dir.join("100")).unwrap();
        // 非时间戳目录与散落文件应被忽略
        fs::create_dir_all(game_dir.join("not-a-ts")).unwrap();
        fs::write(game_dir.join("stray.txt"), b"x").unwrap();
        // 有 meta 的备份，排序靠前
        fs::create_dir_all(game_dir.join("200")).unwrap();
        fs::write(
            game_dir.join("200/meta.json"),
            r#"{"created_at":200,"note":"n","file_count":1,"size_bytes":2,"is_auto":true}"#,
        )
        .unwrap();

        let list = collect_backups(&root, 1);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, "200");
        assert!(list[0].is_auto);
        assert_eq!(list[1].id, "100");
        assert_eq!(list[1].created_at, 0);
        cleanup_dir(&base);
    }

    #[test]
    fn test_restore_core_rejects_bad_input() {
        let base = temp_dir();
        let root = base.join("backups");
        fs::create_dir_all(&root).unwrap();

        assert!(restore_core(&root, 1, "../evil", "/x").is_err());
        assert!(restore_core(&root, 1, "999999", "/x").unwrap_err().contains("备份不存在"));
        assert!(restore_core(&root, 1, "999999", "").unwrap_err().contains("存档路径未设置"));
        cleanup_dir(&base);
    }

    #[test]
    fn test_restore_core_makes_safety_backup_and_merges() {
        let base = temp_dir();
        let save = make_save_dir(&base);
        let root = base.join("backups");

        // 先做一次手动备份，然后改动存档
        let info = create_backup(&save.to_string_lossy(), &root, 1, "原始", false).unwrap();
        fs::write(save.join("a.sav"), b"CORRUPTED").unwrap();
        fs::write(save.join("extra.sav"), b"should-survive").unwrap();

        restore_core(&root, 1, &info.id, &save.to_string_lossy()).unwrap();

        // 同名文件被覆盖回备份内容
        assert_eq!(fs::read(save.join("a.sav")).unwrap(), b"save-a");
        // 备份中没有的文件不被删除（合并语义）
        assert_eq!(fs::read(save.join("extra.sav")).unwrap(), b"should-survive");
        // meta.json 不泄漏进存档目录
        assert!(!save.join("meta.json").exists());

        // 恢复前自动生成的安全备份保留了损坏前的现场
        let auto: Vec<_> = collect_backups(&root, 1).into_iter().filter(|b| b.is_auto).collect();
        assert_eq!(auto.len(), 1);
        let safety_dir = game_backups_dir(&root, 1).join(&auto[0].id);
        assert_eq!(fs::read(safety_dir.join("a.sav")).unwrap(), b"CORRUPTED");
        assert!(safety_dir.join("extra.sav").exists());
        cleanup_dir(&base);
    }

    #[test]
    fn test_restore_core_without_existing_save_dir_skips_safety_backup() {
        let base = temp_dir();
        let save = make_save_dir(&base);
        let root = base.join("backups");
        let info = create_backup(&save.to_string_lossy(), &root, 1, "", false).unwrap();

        // 存档目录被删后再恢复
        fs::remove_dir_all(&save).unwrap();
        restore_core(&root, 1, &info.id, &save.to_string_lossy()).unwrap();

        assert_eq!(fs::read(save.join("a.sav")).unwrap(), b"save-a");
        assert!(collect_backups(&root, 1).iter().all(|b| !b.is_auto));
        cleanup_dir(&base);
    }

    #[test]
    fn test_delete_backup_core() {
        let base = temp_dir();
        let save = make_save_dir(&base);
        let root = base.join("backups");
        let info = create_backup(&save.to_string_lossy(), &root, 1, "", false).unwrap();

        assert!(delete_backup_core(&root, 1, "1a/../x").is_err());
        assert!(delete_backup_core(&root, 1, "42").unwrap_err().contains("备份不存在"));

        delete_backup_core(&root, 1, &info.id).unwrap();
        assert!(!game_backups_dir(&root, 1).join(&info.id).exists());
        cleanup_dir(&base);
    }
}
