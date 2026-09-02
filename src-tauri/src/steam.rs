//! Steam Store 集成模块
//!
//! 提供 Steam 商店游戏搜索、封面下载和本地库扫描导入功能，无需 API Key

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{Manager, State};

use crate::helpers::{build_http_agent, find_main_exe, friendly_http_error};
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

/// 按指定语言/地区搜索 Steam 商店
fn search_steam_locale(query: &str, lang: &str, cc: &str) -> Result<Vec<SteamResult>, String> {
    let url = format!(
        "https://store.steampowered.com/api/storesearch/?term={}&cc={}&l={}",
        urlencoding::encode(query),
        cc,
        lang
    );
    let agent = build_http_agent();
    let resp = agent
        .get(&url)
        .set("User-Agent", "Floralis/0.1")
        .call()
        .map_err(|e| friendly_http_error("Steam", &e))?;
    let parsed: SteamSearchResponse =
        resp.into_json().map_err(|e| format!("Steam 解析失败: {}", e))?;
    Ok(parsed.items.into_iter().take(5).collect())
}

/// 同一 appid 的多语言命中合并结果
#[derive(Debug, Clone)]
pub struct SteamBilingualHit {
    pub id: i64,
    /// 非拉丁本地化名（简中/日文商店返回），缺失时为空
    pub name_cn: Option<String>,
    /// 英文名（国际商店），缺失时为空
    pub name_en: Option<String>,
}

/// 是否含假名（平假名/片假名/半角片假名）
fn contains_kana(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(c,
            '\u{3041}'..='\u{309f}' | '\u{30a1}'..='\u{30f6}' | '\u{ff66}'..='\u{ff9d}')
    })
}

/// 决定要查哪些 locale：`(语言, 国家码)`
///
/// Steam `storesearch` 的检索索引按语种隔离，非拉丁查询词只在对应 locale 里能命中——
/// 实测日文名「存在感薄い妹との簡単生活」只在 `l=japanese&cc=jp` 返回结果，
/// 用 `l=schinese` 或 `l=english` 查都是 `{"total":0}`，导致人工能搜到而我们永远 0 候选。
/// 因此按查询词文字种类追加 locale，中文/英文查询维持原有两个不变。
fn locales_for(query: &str) -> Vec<(&'static str, &'static str)> {
    let mut locales: Vec<(&'static str, &'static str)> =
        vec![("schinese", "cn"), ("english", "us")];
    if contains_kana(query) {
        locales.push(("japanese", "jp"));
    }
    locales
}

/// 多 locale 检索：逐个语种查询后按 appid 合并
///
/// Steam `storesearch` 对词元缺失分隔符很宽容（`KaijuPrincess` 能命中 `Kaiju Princess`），
/// 但每个语种只有自己的名称字段，且用日文名去查简中会返回零结果，
/// 因此各 locale 各查一遍，打分时任一语种名命中即可、展示优先取本地化名。
pub fn search_steam_bilingual(query: &str) -> Vec<SteamBilingualHit> {
    let mut out: Vec<SteamBilingualHit> = Vec::new();
    let mut lookup: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();
    for (lang, cc) in locales_for(query) {
        let items = search_steam_locale(query, lang, cc).unwrap_or_default();
        for r in items {
            match lookup.get(&r.id) {
                Some(idx) => {
                    let hit = &mut out[*idx];
                    if !r.name.is_ascii() {
                        if hit.name_cn.is_none() {
                            hit.name_cn = Some(r.name.clone());
                        }
                    } else if hit.name_en.is_none() {
                        hit.name_en = Some(r.name.clone());
                    } else if hit.name_cn.is_none() {
                        hit.name_cn = Some(r.name.clone());
                    }
                }
                None => {
                    let has_cjk = !r.name.is_ascii();
                    out.push(SteamBilingualHit {
                        id: r.id,
                        name_cn: if has_cjk { Some(r.name.clone()) } else { None },
                        name_en: if has_cjk { None } else { Some(r.name.clone()) },
                    });
                    lookup.insert(r.id, out.len() - 1);
                }
            }
        }
    }
    out
}

/// appdetails 拉到的补充信息
pub struct SteamDetail {
    pub name: String,
    pub short_description: String,
}

/// 取商店详情页（简中优先，回退英文），用于补全展示名与简介
///
/// 注意：不要加 `filters=` 参数——带了它 Steam 会返回 `success: true` 但 `data` 为空对象，
/// 名称与简介全部丢失；只能拉全量响应后取需要的字段。
pub fn fetch_steam_detail(app_id: i64) -> Option<SteamDetail> {
    let agent = build_http_agent();
    for lang in ["schinese", "english"] {
        let url = format!(
            "https://store.steampowered.com/api/appdetails?appids={}&l={}",
            app_id, lang
        );
        let Ok(resp) = agent.get(&url).set("User-Agent", "Floralis/0.1").call() else {
            continue;
        };
        let Ok(value) = resp.into_json::<serde_json::Value>() else {
            continue;
        };
        let data = value
            .get(app_id.to_string())
            .filter(|v| v.get("success").and_then(|s| s.as_bool()).unwrap_or(false))
            .and_then(|v| v.get("data"));
        if let Some(data) = data {
            let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if name.trim().is_empty() {
                continue;
            }
            let short = strip_html(
                data
                    .get("short_description")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
            );
            return Some(SteamDetail {
                name,
                short_description: short,
            });
        }
    }
    None
}

/// Steam 简介字段含 HTML 标签与实体，入库存纯文本
fn strip_html(raw: &str) -> String {
    let without_tags: String = {
        let mut out = String::with_capacity(raw.len());
        let mut depth = 0usize;
        for c in raw.chars() {
            match c {
                '<' => depth += 1,
                '>' => depth = depth.saturating_sub(1),
                _ if depth == 0 => out.push(c),
                _ => {}
            }
        }
        out
    };
    without_tags
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// 搜索 Steam 商店
///
/// 返回最多 5 个匹配结果，包含游戏名称和缩略图
#[tauri::command]
pub fn search_steam(query: String) -> Result<Vec<SteamResult>, String> {
    search_steam_locale(&query, "schinese", "cn")
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
            let exe_path = find_main_exe(&install_path).exe_path;
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
    results.sort_by_key(|a| a.name.to_lowercase());
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kana_query_also_hits_japanese_locale() {
        // 假名查询必须多查一次日文区，否则日文名永远 0 候选
        assert_eq!(
            locales_for("存在感薄い妹との簡単生活"),
            vec![("schinese", "cn"), ("english", "us"), ("japanese", "jp")]
        );
        // 中文/英文查询维持两个 locale 不变（不多花一次请求）
        assert_eq!(locales_for("怠惰的怪兽公主"), vec![("schinese", "cn"), ("english", "us")]);
        assert_eq!(locales_for("Kaiju Princess"), vec![("schinese", "cn"), ("english", "us")]);
        // 纯汉字日文标题无假名时不额外查（汉字本身在简中区能命中）
        assert!(!contains_kana("種付委員"));
        assert!(contains_kana("オシゴト"));
    }

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
