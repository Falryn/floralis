//! 共享数据结构定义

use serde::{Deserialize, Serialize};
use crate::db::{AppSettings, Game, Group, Mod};

/// 应用全局状态
pub struct AppState {
    pub db: std::sync::Arc<crate::db::Database>,
    pub monitor: crate::playtime::PlaytimeMonitor,
}

/// 压缩包解压结果
#[derive(Serialize)]
pub struct ExtractResult {
    pub success: bool,
    pub exe_path: String,
    pub cover_path: String,
    pub detected_name: String,
    pub extract_dir: String,
    pub save_path: String,
    pub error: String,
}

/// Steam 本地库扫描出的游戏条目
#[derive(Serialize)]
pub struct SteamLibraryItem {
    pub app_id: i64,
    pub name: String,
    pub install_path: String,
    pub exe_path: String,
    pub cover_path: String,
}

/// 数据备份结构
#[derive(Serialize, Deserialize)]
pub struct BackupData {
    pub version: u32,
    pub games: Vec<Game>,
    pub groups: Vec<Group>,
    pub settings: AppSettings,
    pub passwords: Vec<String>,
    #[serde(default)]
    pub mods: Vec<Mod>,
    #[serde(default)]
    pub mod_tags: Vec<(i64, i64)>,
}

/// 游玩日历数据
#[derive(Serialize)]
pub struct PlayCalendarDay {
    pub date: String,
    pub duration: i64,
}

/// 截图
#[derive(Serialize)]
pub struct Screenshot {
    pub id: i64,
    pub path: String,
}

/// 封面完整性状态
#[derive(Serialize)]
pub struct CoverStatus {
    pub game_id: i64,
    pub game_name: String,
    pub cover_path: String,
    pub exists: bool,
}

/// 版本更新信息
#[derive(Serialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_url: String,
    pub release_notes: String,
}

/// 扫描到的 Mod
#[derive(Serialize)]
pub struct ScannedMod {
    pub name: String,
    pub path: String,
    pub mod_type: String,
}

/// 完整性体检发现的单个问题
#[derive(Serialize)]
pub struct IntegrityIssue {
    pub game_id: i64,
    pub game_name: String,
    /// 问题类型：missing_cover / missing_exe / missing_install / missing_save
    pub issue_type: String,
    /// 失效的路径
    pub path: String,
}

/// 数据完整性体检报告
#[derive(Serialize)]
pub struct IntegrityReport {
    pub total_games: usize,
    pub issues: Vec<IntegrityIssue>,
    /// 未被任何游戏引用的孤儿封面文件
    pub orphan_covers: Vec<String>,
}

/// 批量库重定位结果
#[derive(Serialize)]
pub struct RelocateReport {
    /// 成功修复的游戏数
    pub fixed: usize,
    /// 未匹配到同名文件夹的游戏名列表（含同名冲突跳过项）
    pub unmatched: Vec<String>,
}

/// 存档备份条目信息
#[derive(Serialize, Debug)]
pub struct SaveBackupInfo {
    /// 备份目录名（unix 时间戳）
    pub id: String,
    pub game_id: i64,
    pub created_at: i64,
    pub note: String,
    pub file_count: u64,
    pub size_bytes: u64,
    pub is_auto: bool,
}
