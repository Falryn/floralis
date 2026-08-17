//! Bangumi (bgm.tv) 集成模块
//!
//! 提供游戏搜索和封面下载功能，无需 API Key，国内可直连

use serde::{Deserialize, Serialize};
use std::fs;
use tauri::Manager;

use crate::helpers::build_http_agent;

/// Bangumi 搜索结果项
#[derive(Serialize, Deserialize, Debug)]
pub struct BangumiResult {
    pub id: i64,
    #[serde(rename = "nameCn")]
    pub name_cn: Option<String>,
    pub name: String,
    pub images: Option<BangumiImages>,
    #[serde(rename = "infobox")]
    pub infobox: Option<serde_json::Value>,
    pub summary: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BangumiImages {
    pub grid: Option<String>,
    pub large: Option<String>,
    pub common: Option<String>,
    pub medium: Option<String>,
    pub small: Option<String>,
}

#[derive(Deserialize)]
struct BangumiSearchResponse {
    data: Vec<BangumiResult>,
}

/// 搜索 Bangumi 游戏数据库
///
/// type=4 为游戏条目，返回最多 5 个匹配结果
/// 注意：Bangumi 搜索 API 必须使用 POST + JSON body
#[tauri::command]
pub fn search_bangumi(query: String) -> Result<Vec<BangumiResult>, String> {
    let body = serde_json::json!({
        "keyword": query,
        "filter": { "type": [4] },
        "limit": 5
    });

    let agent = build_http_agent();
    let resp = agent.post("https://api.bgm.tv/v0/search/subjects")
        .set("Content-Type", "application/json")
        .set("User-Agent", "Floralis/0.1 (https://github.com/Echon/floralis)")
        .send_string(&body.to_string())
        .map_err(|e| format!("Bangumi 请求失败: {}", e))?;

    let search_resp: BangumiSearchResponse =
        resp.into_json().map_err(|e| format!("Bangumi 解析失败: {}", e))?;

    let results: Vec<BangumiResult> = search_resp.data.into_iter().take(5).collect();
    Ok(results)
}

/// 下载 Bangumi 游戏封面
///
/// 将封面图片保存到应用数据目录，返回本地文件路径
#[tauri::command]
pub fn download_bangumi_cover(
    url: String,
    game_id: i64,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let covers_dir = app_data_dir.join("covers");
    fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;

    let ext = if url.contains(".png") {
        "png"
    } else if url.contains(".webp") {
        "webp"
    } else {
        "jpg"
    };
    let dest = covers_dir.join(format!("cover_{}.{}", game_id, ext));

    let agent = build_http_agent();
    let resp = agent.get(&url)
        .set("User-Agent", "Floralis/0.1 (https://github.com/Echon/floralis)")
        .call()
        .map_err(|e| format!("Bangumi 封面下载失败: {}", e))?;

    let mut reader = resp.into_reader();
    let mut file = fs::File::create(&dest).map_err(|e| e.to_string())?;
    std::io::copy(&mut reader, &mut file).map_err(|e| e.to_string())?;

    Ok(dest.to_string_lossy().to_string())
}
