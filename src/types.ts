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
