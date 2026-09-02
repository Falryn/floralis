//! Epic Games 库扫描模块
//!
//! 从 %ProgramData%\Epic\EpicGamesLauncher\Data\Manifests\*.item 读取官方游戏名、
//! 安装路径与主程序。清单不含封面图：封面由导入结果页"一键匹配"补全。

use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use tauri::State;

use crate::helpers::find_main_exe;
use crate::models::{AppState, EpicLibraryItem};

/// 检测 Epic 清单目录（%ProgramData%\Epic\EpicGamesLauncher\Data\Manifests）
#[tauri::command]
pub fn detect_epic_manifests_dir() -> Result<String, String> {
    let program_data =
        std::env::var("ProgramData").map_err(|_| "无法读取 ProgramData 环境变量".to_string())?;
    let dir = PathBuf::from(program_data)
        .join("Epic")
        .join("EpicGamesLauncher")
        .join("Data")
        .join("Manifests");
    if dir.is_dir() {
        Ok(dir.to_string_lossy().to_string())
    } else {
        Err("未找到 Epic Games 清单目录（可能未安装 Epic Games Launcher）".into())
    }
}

/// 扫描 Epic 本地库：解析清单目录下所有 *.item
///
/// 跳过未完成的安装、非应用条目与 DLC（AppName != MainGameAppName）。
/// 已在库中的游戏（按 install_path 归一化比较）自动跳过。
#[tauri::command]
pub async fn scan_epic_library(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<EpicLibraryItem>, String> {
    let existing: HashSet<String> = state
        .db
        .get_all_games()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|g| !g.install_path.is_empty())
        .map(|g| g.install_path.replace('/', "\\").to_lowercase())
        .collect();

    tauri::async_runtime::spawn_blocking(move || scan_epic_library_impl(&path, &existing))
        .await
        .map_err(|e| format!("任务执行失败: {}", e))?
}

fn scan_epic_library_impl(
    path: &str,
    existing: &HashSet<String>,
) -> Result<Vec<EpicLibraryItem>, String> {
    let dir = PathBuf::from(path);
    if !dir.is_dir() {
        return Err("清单目录不存在".into());
    }
    let entries = fs::read_dir(&dir).map_err(|e| format!("无法读取清单目录: {}", e))?;

    let mut results = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        let fname = p
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if !fname.ends_with(".item") {
            continue;
        }
        let content = match fs::read_to_string(&p) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let json: Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let name = json.get("DisplayName").and_then(|v| v.as_str()).unwrap_or("");
        let install_location = json.get("InstallLocation").and_then(|v| v.as_str()).unwrap_or("");
        if name.is_empty() || install_location.is_empty() {
            continue;
        }
        // 未完成安装 / 非应用条目
        if json.get("bIsIncompleteInstall").and_then(|v| v.as_bool()).unwrap_or(false) {
            continue;
        }
        if !json.get("bIsApplication").and_then(|v| v.as_bool()).unwrap_or(true) {
            continue;
        }
        // DLC / 附加内容：AppName 从属于某个主游戏
        let app_name = json.get("AppName").and_then(|v| v.as_str()).unwrap_or("");
        let main_app = json.get("MainGameAppName").and_then(|v| v.as_str()).unwrap_or("");
        if !main_app.is_empty() && app_name != main_app {
            continue;
        }

        let install_path = PathBuf::from(install_location);
        if !install_path.exists() {
            continue; // 已卸载但清单残留
        }
        let install_str = install_path.to_string_lossy().to_string();
        if existing.contains(&install_str.replace('/', "\\").to_lowercase()) {
            continue;
        }

        // 优先使用清单声明的启动程序，缺失时回退到通用主程序检测
        let exe_path = json
            .get("LaunchExecutable")
            .and_then(|v| v.as_str())
            .map(|rel| install_path.join(rel))
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| find_main_exe(&install_path).exe_path);

        results.push(EpicLibraryItem {
            name: name.to_string(),
            install_path: install_str,
            exe_path,
        });
    }
    results.sort_by_key(|a| a.name.to_lowercase());
    Ok(results)
}

#[cfg(test)]
mod tests {
    /// 针对真实 Epic 清单目录的扫描验证（需设置 EPIC_TEST_MANIFESTS 环境变量，否则跳过）
    #[test]
    fn scan_real_epic_library() {
        let path = std::env::var("EPIC_TEST_MANIFESTS").unwrap_or_default();
        if path.is_empty() {
            return;
        }
        let existing = std::collections::HashSet::new();
        let items = super::scan_epic_library_impl(&path, &existing).expect("扫描失败");
        println!("found {} games", items.len());
        for it in items.iter().take(5) {
            println!("{} | {} | exe={}", it.name, it.install_path, !it.exe_path.is_empty());
        }
        assert!(!items.is_empty(), "未扫描到任何 Epic 游戏");
    }
}
