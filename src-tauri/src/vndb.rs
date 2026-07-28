//! VNDB (Visual Novel Database) 集成模块
//! 
//! 提供视觉小说搜索和封面下载功能

use serde::{Deserialize, Serialize};
use std::fs;
use tauri::Manager;

/// VNDB 游戏搜索结果
#[derive(Serialize, Deserialize)]
pub struct VndbResult {
    pub id: String,
    pub title: String,
    pub image: Option<VndbImage>,
    pub description: Option<String>,
}

/// VNDB 封面图片信息
#[derive(Serialize, Deserialize)]
pub struct VndbImage {
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct VndbResponse {
    results: Vec<VndbResult>,
}

/// 搜索 VNDB 视觉小说数据库
/// 
/// 返回最多 5 个匹配结果
#[tauri::command]
pub fn search_vndb(query: String) -> Result<Vec<VndbResult>, String> {
    let body = serde_json::json!({
        "filters": ["search", "=", query],
        "fields": "id,title,image.url,description",
        "results": 5
    });

    let resp = ureq::post("https://api.vndb.org/kana/vn")
        .set("Content-Type", "application/json")
        .set("User-Agent", "Floralis/0.1")
        .send_string(&body.to_string())
        .map_err(|e| format!("VNDB 请求失败: {}", e))?;

    let vndb_resp: VndbResponse = resp.into_json().map_err(|e| format!("VNDB 解析失败: {}", e))?;
    Ok(vndb_resp.results)
}

/// 下载 VNDB 游戏封面图片
/// 
/// 将封面图片保存到应用数据目录，返回本地文件路径
#[tauri::command]
pub fn download_vndb_cover(url: String, game_id: i64, app: tauri::AppHandle) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let covers_dir = app_data_dir.join("covers");
    fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;

    let ext = if url.contains(".png") { "png" }
    else if url.contains(".webp") { "webp" }
    else { "jpg" };
    let dest = covers_dir.join(format!("cover_{}.{}", game_id, ext));

    let resp = ureq::get(&url)
        .set("User-Agent", "Floralis/0.1")
        .call()
        .map_err(|e| format!("下载失败: {}", e))?;

    let mut reader = resp.into_reader();
    let mut file = fs::File::create(&dest).map_err(|e| e.to_string())?;
    std::io::copy(&mut reader, &mut file).map_err(|e| e.to_string())?;

    Ok(dest.to_string_lossy().to_string())
}
