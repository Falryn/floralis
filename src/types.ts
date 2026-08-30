/**
 * 前端类型定义
 * 
 * 与 Rust 后端数据结构保持一致
 */

/** 游戏数据 */
export interface Game {
  id: number;
  name: string;
  group_id: number | null;
  install_path: string;
  exe_path: string;
  launch_args: string;
  cover_path: string;
  save_path: string;
  notes: string;
  script_path: string;
  script_args: string;
  total_play_time: number;
  last_played_at: string | null;
  status: string;
  rating: number;
  sort_order: number;
  default_mod_dir: string;
  mod_naming_pattern: string;
  mod_uses_load_order: boolean;
  tracked_process_name: string;
  is_favorite: boolean;
}

/** 游戏分组 */
export interface Group {
  id: number;
  name: string;
  sort_order: number;
}

/** 应用设置 */
export interface AppSettings {
  seven_zip_path: string;
  default_extract_path: string;
  custom_banner: string;
  custom_sidebar_bg: string;
  custom_empty_illustration: string;
  theme: string;
  update_repo: string;
  close_behavior: string;
  igdb_client_id: string;
  igdb_client_secret: string;
  image_blur: string;
  banner_blur: string;
  banner_brightness: string;
  sidebar_blur: string;
  sidebar_brightness: string;
  auto_backup: string;
  save_backup_dir: string;
}

/** 版本更新信息 */
export interface UpdateInfo {
  available: boolean;
  current_version: string;
  latest_version: string;
  release_url: string;
  release_notes: string;
}

/** 压缩包解压结果 */
export interface ExtractResult {
  success: boolean;
  exe_path: string;
  cover_path: string;
  detected_name: string;
  extract_dir: string;
  save_path: string;
  error: string;
}

/** Steam 本地库扫描出的游戏条目 */
export interface SteamLibraryItem {
  app_id: number;
  name: string;
  install_path: string;
  exe_path: string;
  cover_path: string;
}

/** 游戏会话记录 */
export interface PlaySession {
  id: number;
  game_id: number;
  start_time: string;
  end_time: string | null;
  duration_seconds: number;
}

/** 标签 */
export interface Tag {
  id: number;
  name: string;
}

/** 附加启动入口（一个游戏可配置多个，如汉化版、配置工具、不同参数） */
export interface LaunchAction {
  id: number;
  game_id: number;
  name: string;
  program_path: string;
  args: string;
  sort_order: number;
}

export interface TagUsage {
  id: number;
  name: string;
  game_count: number;
  mod_count: number;
}

/** 存档备份条目 */
export interface SaveBackupInfo {
  id: string;
  game_id: number;
  created_at: number;
  note: string;
  file_count: number;
  size_bytes: number;
  is_auto: boolean;
}

/** Mod 模组数据 */
export interface Mod {
  id: number;
  name: string;
  description: string;
  mod_path: string;
  install_path: string;
  game_id: number | null;
  game_dir: string;
  version: string;
  author: string;
  is_enabled: boolean;
  sort_order: number;
  category: string;
  source_url: string;
  cover_path: string;
  mod_type: string;
  original_name: string;
  created_at: string;
  updated_at: string;
}

/** 扫描到的 Mod（未导入） */
export interface ScannedMod {
  name: string;
  path: string;
  mod_type: string;
}

/** Mod 配置文件（按游戏维护多套启用组合） */
export interface ModProfile {
  id: number;
  game_id: number;
  name: string;
  mod_ids: number[];
  created_at: string;
  updated_at: string;
}

/** 应用配置文件的执行结果 */
export interface ApplyProfileResult {
  changed: number;
  failures: string[];
}

/** Mod 扫描进度 */
export interface ModScanProgress {
  current: number;
  total: number;
  name: string;
}

/** 完整性体检发现的单个问题 */
export interface IntegrityIssue {
  game_id: number;
  game_name: string;
  /** missing_cover / missing_exe / missing_install / missing_save */
  issue_type: string;
  path: string;
}

/** 数据完整性体检报告 */
export interface IntegrityReport {
  total_games: number;
  issues: IntegrityIssue[];
  orphan_covers: string[];
}

/** 批量库重定位结果 */
export interface RelocateReport {
  /** 成功修复的游戏数 */
  fixed: number;
  /** 未匹配到同名文件夹的游戏名列表（含同名冲突跳过项） */
  unmatched: string[];
}
