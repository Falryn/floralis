//! Steam Store 集成模块
//!
//! 提供 Steam 商店游戏搜索、封面下载和本地库扫描导入功能，无需 API Key

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{Manager, State};

use crate::helpers::{build_http_agent, find_main_exe};
use crate::models::{AppState, SteamLibraryItem};

/// Steam 搜索结果项
#[derive(Serialize, Deserialize, Debug)]
pub struct SteamResult {
    pub id: i64,
    pub name: String,
    pub tiny_image: Option<String>,
}

#[derive(Deserialize)]
struct SteamSearchResponse {
    #[serde(default)]
    items: Vec<SteamResult>,
}

/// 搜索 Steam 商店
///
/// 返回最多 5 个匹配结果，包含游戏名称和缩略图
#[tauri::command]
pub fn search_steam(query: String) -> Result<Vec<SteamResult>, String> {
    let url = format!(
        "https://store.steampowered.com/api/storesearch/?term={}&cc=cn&l=schinese",
        urlencoding::encode(&query)
    );

    let agent = build_http_agent();
    let resp = agent.get(&url)
        .set("User-Agent", "Floralis/0.1")
        .call()
        .map_err(|e| format!("Steam 请求失败: {}", e))?;

    let search_resp: SteamSearchResponse =
        resp.into_json().map_err(|e| format!("Steam 解析失败: {}", e))?;

    let results: Vec<SteamResult> = search_resp.items.into_iter().take(5).collect();
    Ok(results)
}

/// 下载 Steam 游戏封面
///
/// 优先下载 library_600x900 竖版大图，失败则回退到 header 图
#[tauri::command]
pub fn download_steam_cover(
    app_id: i64,
    game_id: i64,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let covers_dir = app_data_dir.join("covers");
    fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;

    let dest = covers_dir.join(format!("cover_{}.jpg", game_id));

    // 优先竖版大图
    let urls = [
        format!("https://cdn.akamai.steamstatic.com/steam/apps/{}/library_600x900.jpg", app_id),
        format!("https://cdn.akamai.steamstatic.com/steam/apps/{}/header.jpg", app_id),
    ];

    let agent = build_http_agent();
    for url in &urls {
        if let Ok(resp) = agent.get(url)
            .set("User-Agent", "Floralis/0.1")
            .call()
        {
            let mut reader = resp.into_reader();
            let mut file = fs::File::create(&dest).map_err(|e| e.to_string())?;
            std::io::copy(&mut reader, &mut file).map_err(|e| e.to_string())?;
            return Ok(dest.to_string_lossy().to_string());
        }
    }

    Err("Steam 封面下载失败：所有 URL 均不可用".to_string())
}

// ==================== 本地库扫描导入 ====================

/// 从注册表检测 Steam 安装根目录
#[tauri::command]
pub fn detect_steam_root() -> Result<String, String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WOW64_64KEY};
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags("Software\\Valve\\Steam", KEY_READ | KEY_WOW64_64KEY)
        .map_err(|_| "未检测到 Steam 注册表项".to_string())?;
    let path: String = key
        .get_value("SteamPath")
        .map_err(|_| "注册表中未找到 SteamPath".to_string())?;
    let normalized = path.replace('/', "\\");
    if Path::new(&normalized).exists() {
        Ok(normalized)
    } else {
        Err("注册表中的 Steam 路径不存在".to_string())
    }
}

/// 解析 VDF/ACF 文本中的引号键值对（仅收集同行有两个引号串的键值行）
///
/// 返回按出现顺序的 (key小写, value) 列表，保留重复键（libraryfolders.vdf 中有多个 path）。
fn parse_vdf_pairs(content: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for line in content.lines() {
        let t = line.trim();
        if !t.starts_with('"') {
            continue;
        }
        let mut parts: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut in_str = false;
        for c in t.chars() {
            if c == '"' {
                if in_str {
                    parts.push(std::mem::take(&mut cur));
                    if parts.len() >= 2 {
                        break;
                    }
                }
                in_str = !in_str;
            } else if in_str {
                cur.push(c);
            }
        }
        if parts.len() == 2 {
            pairs.push((parts[0].to_lowercase(), parts[1].clone()));
        }
    }
    pairs
}

fn has_appmanifest(dir: &Path) -> bool {
    fs::read_dir(dir)
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|e| {
                let name = e.file_name().to_string_lossy().to_lowercase();
                name.starts_with("appmanifest_") && name.ends_with(".acf")
            })
        })
        .unwrap_or(false)
}

/// 扫描 Steam 本地库：解析 steamapps 下所有 appmanifest_*.acf
///
/// 支持传入 Steam 根目录或 steamapps 目录；自动解析 libraryfolders.vdf
/// 获取额外库文件夹。已在库中的游戏（按 install_path 归一化比较）自动跳过。
/// 封面优先使用 Steam 本地缓存（appcache/librarycache/{appid}_header.jpg）。
#[tauri::command]
pub async fn scan_steam_library(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<SteamLibraryItem>, String> {
    let existing: HashSet<String> = state
        .db
        .get_all_games()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|g| !g.install_path.is_empty())
        .map(|g| g.install_path.replace('/', "\\").to_lowercase())
        .collect();

    tauri::async_runtime::spawn_blocking(move || scan_steam_library_impl(&path, &existing))
        .await
        .map_err(|e| format!("任务执行失败: {}", e))?
}

fn scan_steam_library_impl(path: &str, existing: &HashSet<String>) -> Result<Vec<SteamLibraryItem>, String> {
    let root_input = PathBuf::from(path);
    if !root_input.exists() {
        return Err("路径不存在".into());
    }
    // 确定 Steam 根目录与主 steamapps 目录
    let (steam_root, mut library_dirs): (PathBuf, Vec<PathBuf>) = {
        let sub = root_input.join("steamapps");
        if sub.is_dir() {
            (root_input.clone(), vec![sub])
        } else if has_appmanifest(&root_input) {
            let root = root_input
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| root_input.clone());
            (root, vec![root_input.clone()])
        } else {
            return Err("所选目录不是 Steam 安装目录或 steamapps 目录".into());
        }
    };
    // 解析 libraryfolders.vdf，补充额外库文件夹（VDF 中反斜杠以 \\ 转义，需还原；
    // 归一化比较避免主库自身被重复加入导致游戏重复扫描）
    let normalize_dir = |p: &Path| p.to_string_lossy().replace('/', "\\").to_lowercase();
    let mut seen_dirs: HashSet<String> = library_dirs
        .iter()
        .map(|d| normalize_dir(d))
        .collect();
    let lf = library_dirs[0].join("libraryfolders.vdf");
    if let Ok(content) = fs::read_to_string(&lf) {
        for (key, value) in parse_vdf_pairs(&content) {
            if key == "path" {
                let extra = PathBuf::from(value.replace("\\\\", "\\").replace('/', "\\")).join("steamapps");
                if extra.is_dir() && seen_dirs.insert(normalize_dir(&extra)) {
                    library_dirs.push(extra);
                }
            }
        }
    }

    let mut results = Vec::new();
    for lib in &library_dirs {
        let entries = match fs::read_dir(lib) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            let fname = p
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if !(fname.starts_with("appmanifest_") && fname.ends_with(".acf")) {
                continue;
            }
            let content = match fs::read_to_string(&p) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let mut appid: i64 = 0;
            let mut name = String::new();
            let mut installdir = String::new();
            for (k, v) in parse_vdf_pairs(&content) {
                match k.as_str() {
                    "appid" => appid = v.parse().unwrap_or(appid),
                    "name" => name = v,
                    "installdir" => installdir = v,
                    _ => {}
                }
            }
            if appid == 0 || name.is_empty() || installdir.is_empty() {
                continue;
            }
            // 排除 Steam 运行库/共享组件（非游戏）
            const REDIST_APP_IDS: &[i64] = &[228980];
            if REDIST_APP_IDS.contains(&appid)
                || installdir.eq_ignore_ascii_case("Steamworks Shared")
                || name.to_lowercase().contains("redistributable")
            {
                continue;
            }
            // 游戏实际安装在 steamapps/common/{installdir}
            let install_path = lib.join("common").join(&installdir);
            if !install_path.exists() {
                continue; // 已卸载但清单残留
            }
            let install_str = install_path.to_string_lossy().to_string();
            if existing.contains(&install_str.replace('/', "\\").to_lowercase()) {
                continue; // 已在库中，跳过
            }
            let (exe_path, _) = find_main_exe(&install_path);
            let cover_path = {
                // 新版 Steam 封面缓存为目录：librarycache/{appid}/，优先竖版大图
                let cache_dir = steam_root.join("appcache").join("librarycache").join(appid.to_string());
                let candidates = [
                    cache_dir.join("library_600x900.jpg"),
                    cache_dir.join("header.jpg"),
                ];
                candidates
                    .iter()
                    .find(|c| c.exists())
                    .map(|c| c.to_string_lossy().to_string())
                    .unwrap_or_default()
            };
            results.push(SteamLibraryItem {
                app_id: appid,
                name,
                install_path: install_str,
                exe_path,
                cover_path,
            });
        }
    }
    results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(results)
}

#[cfg(test)]
mod tests {
    /// 针对真实 Steam 目录的扫描验证（需设置 STEAM_TEST_ROOT 环境变量，否则跳过）
    #[test]
    fn scan_real_steam_library() {
        let path = std::env::var("STEAM_TEST_ROOT").unwrap_or_default();
        if path.is_empty() {
            return;
        }
        let existing = std::collections::HashSet::new();
        let items = super::scan_steam_library_impl(&path, &existing).expect("扫描失败");
        println!("found {} games", items.len());
        for it in items.iter().take(5) {
            println!(
                "{} | {} | exe={} | cover={}",
                it.name,
                it.install_path,
                !it.exe_path.is_empty(),
                !it.cover_path.is_empty()
            );
        }
        assert!(!items.is_empty(), "未扫描到任何 Steam 游戏");
    }
}
