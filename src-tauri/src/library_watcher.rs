//! 游戏库目录监视
//!
//! 监视用户配置的根目录，新游戏文件夹出现时防抖后通知前端（`library://dir-changed`），
//! 由前端调用现有 `scan_library_root` 完成检测与入库。

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Mutex;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, State};

/// 事件静默多久后触发通知（聚合复制等批量操作产生的事件风暴）
const DEBOUNCE_MS: u64 = 3000;

/// 通知前端的事件名
pub const EVENT_LIBRARY_DIR_CHANGED: &str = "library://dir-changed";

struct WatcherInner {
    _watcher: RecommendedWatcher,
}

#[derive(Default)]
pub struct LibraryWatcherState {
    inner: Mutex<Option<WatcherInner>>,
}

/// Windows 路径归一化：统一分隔符并转小写（仅用于比较）
fn normalize(p: &Path) -> String {
    p.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

/// 判断 path 是否为 root 的直接子目录（隐藏目录除外）
pub fn is_direct_child_dir(root: &Path, path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    if normalize(parent) != normalize(root) {
        return false;
    }
    path.file_name()
        .map(|n| !n.to_string_lossy().starts_with('.'))
        .unwrap_or(false)
}

/// 防抖窗口结束后聚合路径：归一化去重，仅保留仍存在的直接子目录。
/// 窗口内被删除/重命名走的目录在此被过滤，避免误报。
fn collect_new_child_dirs(root: &Path, paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for p in paths {
        if is_direct_child_dir(root, p) && seen.insert(normalize(p)) {
            out.push(p.clone());
        }
    }
    out
}

fn stop_inner(state: &LibraryWatcherState) {
    // take 后 drop：watcher 释放 → 回调停止发送 → 防抖线程因通道断开而退出
    let _ = state
        .inner
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take();
}

#[tauri::command]
pub fn start_library_watch(
    app: AppHandle,
    state: State<'_, LibraryWatcherState>,
    path: String,
) -> Result<(), String> {
    let root = PathBuf::from(&path);
    if !root.exists() || !root.is_dir() {
        return Err("监视目录不存在".into());
    }

    stop_inner(&state);

    let (tx, rx) = mpsc::channel::<PathBuf>();
    let mut watcher: RecommendedWatcher =
        notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                for p in event.paths {
                    let _ = tx.send(p);
                }
            }
        })
        .map_err(|e| format!("创建监视器失败: {}", e))?;
    watcher
        .watch(&root, RecursiveMode::NonRecursive)
        .map_err(|e| format!("开始监视失败: {}", e))?;

    // 防抖线程：静默 DEBOUNCE_MS 后，把期间出现的直接子目录去重上报
    let debounced_root = root.clone();
    std::thread::spawn(move || {
        let mut pending: Vec<PathBuf> = Vec::new();
        loop {
            match rx.recv_timeout(Duration::from_millis(DEBOUNCE_MS)) {
                Ok(p) => pending.push(p),
                Err(RecvTimeoutError::Timeout) => {
                    if pending.is_empty() {
                        continue;
                    }
                    let batch: Vec<PathBuf> = std::mem::take(&mut pending);
                    if !collect_new_child_dirs(&debounced_root, &batch).is_empty() {
                        let _ = app.emit(EVENT_LIBRARY_DIR_CHANGED, ());
                    }
                }
                // watcher 被 drop（停止监视或应用退出）→ 通道断开
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    *state
        .inner
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(WatcherInner { _watcher: watcher });
    Ok(())
}

#[tauri::command]
pub fn stop_library_watch(state: State<'_, LibraryWatcherState>) -> Result<(), String> {
    stop_inner(&state);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "floralis_watcher_{}_{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_is_direct_child_dir() {
        let root = temp_root("child");
        let child = root.join("NewGame");
        std::fs::create_dir_all(&child).unwrap();
        let nested = child.join("sub");
        std::fs::create_dir_all(&nested).unwrap();
        let hidden = root.join(".hidden");
        std::fs::create_dir_all(&hidden).unwrap();
        let file = root.join("readme.txt");
        std::fs::write(&file, "x").unwrap();

        assert!(is_direct_child_dir(&root, &child));
        // 深层路径不算
        assert!(!is_direct_child_dir(&root, &nested));
        // 隐藏目录不算
        assert!(!is_direct_child_dir(&root, &hidden));
        // 文件不算
        assert!(!is_direct_child_dir(&root, &file));
        // 不存在的目录不算
        assert!(!is_direct_child_dir(&root, &root.join("ghost")));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn test_normalize_equivalence() {
        assert_eq!(normalize(Path::new("C:/Games/X")), normalize(Path::new("c:\\games\\x")));
        assert_eq!(normalize(Path::new("C:\\games\\x\\")), normalize(Path::new("c:\\games\\x")));
        assert_ne!(normalize(Path::new("c:\\games\\x")), normalize(Path::new("c:\\games\\y")));
    }

    #[test]
    fn test_collect_new_child_dirs_dedupes_and_filters() {
        let root = temp_root("collect");
        let child = root.join("NewGame");
        std::fs::create_dir_all(&child).unwrap();
        let file = root.join("readme.txt");
        std::fs::write(&file, "x").unwrap();

        // 同一目录的大小写/分隔符变体应去重为一份；文件被过滤
        let variants = vec![
            child.clone(),
            root.join("newgame"),
            PathBuf::from(root.to_string_lossy().replace('\\', "/").to_string() + "/NEWGAME"),
            file.clone(),
        ];
        let result = collect_new_child_dirs(&root, &variants);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], child);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn test_collect_new_child_dirs_filters_deleted_within_window() {
        let root = temp_root("deleted");
        let child = root.join("Gone");
        std::fs::create_dir_all(&child).unwrap();

        // 事件进窗口后目录被删：聚合时不再存在，不应上报
        let mut paths = vec![child.clone()];
        std::fs::remove_dir_all(&child).unwrap();
        assert!(collect_new_child_dirs(&root, &paths).is_empty());

        // 重命名场景：旧路径失效被过滤，新路径保留
        let renamed = root.join("Renamed");
        std::fs::create_dir_all(&renamed).unwrap();
        paths.push(renamed.clone());
        let result = collect_new_child_dirs(&root, &paths);
        assert_eq!(result, vec![renamed]);

        std::fs::remove_dir_all(&root).ok();
    }
}
