//! IGDB (Internet Game Database) 集成模块
//! 
//! 提供游戏搜索和封面下载功能，使用 Twitch OAuth 认证

use serde::{Deserialize, Serialize};
use std::fs;
use tauri::Manager;

use crate::helpers::{build_http_agent, friendly_http_error};

/// IGDB 游戏搜索结果
#[derive(Serialize, Deserialize, Debug)]
pub struct IgdbResult {
    pub id: i64,
    pub name: String,
    pub cover: Option<IgdbCover>,
    pub summary: Option<String>,
    pub first_release_date: Option<i64>,
}

/// IGDB 封面图片信息
#[derive(Serialize, Deserialize, Debug)]
pub struct IgdbCover {
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct IgdbTokenResponse {
    access_token: String,
    expires_in: u64,
}

/// 获取 Twitch OAuth 访问令牌 (client_credentials 流程)
fn get_igdb_token(client_id: &str, client_secret: &str) -> Result<String, String> {
    let agent = build_http_agent();
    let resp = agent.post("https://id.twitch.tv/oauth2/token")
        .set("Content-Type", "application/x-www-form-urlencoded")
        .send_string(&format!(
            "client_id={}&client_secret={}&grant_type=client_credentials",
            client_id, client_secret
        ))
        .map_err(|e| friendly_http_error("IGDB 认证", &e))?;

    let token_resp: IgdbTokenResponse =
        resp.into_json().map_err(|e| format!("IGDB token parse failed: {}", e))?;
    Ok(token_resp.access_token)
}

/// 搜索 IGDB 游戏数据库
/// 
/// 返回最多 5 个匹配结果，包含游戏名称、封面、简介等信息
#[tauri::command]
pub fn search_igdb(
    query: String,
    client_id: String,
    client_secret: String,
) -> Result<Vec<IgdbResult>, String> {
    let token = get_igdb_token(&client_id, &client_secret)?;

    let body = format!(
        "fields id,name,cover.url,summary,first_release_date; \
         where name ~ *\"{}\"*; limit 5;",
        query.replace('"', "\\\"")
    );

    let agent = build_http_agent();
    let resp = agent.post("https://api.igdb.com/v4/games")
        .set("Client-ID", &client_id)
        .set("Authorization", &format!("Bearer {}", token))
        .set("Content-Type", "text/plain")
        .send_string(&body)
        .map_err(|e| friendly_http_error("IGDB", &e))?;

    let results: Vec<IgdbResult> =
        resp.into_json().map_err(|e| format!("IGDB parse failed: {}", e))?;
    Ok(results)
}

/// 下载 IGDB 游戏封面图片
/// 
/// 将封面图片保存到应用数据目录，返回本地文件路径
#[tauri::command]
pub fn download_igdb_cover(
    url: String,
    game_id: i64,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let covers_dir = app_data_dir.join("covers");
    fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;

    // IGDB cover URLs use //images.igdb.com/ format, replace with https
    let https_url = if url.starts_with("//") {
        format!("https:{}", url)
    } else {
        url
    };

    // Request original size by removing size prefix in path
    let original_url = https_url.replace("/t_thumb/", "/t_original/");

    let ext = if original_url.contains(".png") {
        "png"
    } else if original_url.contains(".webp") {
        "webp"
    } else {
        "jpg"
    };
    let dest = covers_dir.join(format!("cover_{}.{}", game_id, ext));

    let agent = build_http_agent();
    let resp = agent.get(&original_url)
        .set("User-Agent", "Floralis/0.1")
        .call()
        .map_err(|e| friendly_http_error("IGDB 封面下载", &e))?;

    let mut reader = resp.into_reader();
    let mut file = fs::File::create(&dest).map_err(|e| e.to_string())?;
    std::io::copy(&mut reader, &mut file).map_err(|e| e.to_string())?;

    Ok(dest.to_string_lossy().to_string())
}
