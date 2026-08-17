//! 数据库模块 - 管理所有持久化数据
//! 
//! 使用 SQLite 存储游戏、分组、标签、设置、密码等数据
//! 密码使用 AES-256-GCM 加密存储，密钥通过系统密钥链管理

use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use aes_gcm::aead::Aead;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};

/// 派生 AES-256 加密密钥
/// 
/// 优先使用系统密钥链 (Windows Credential Manager) 存储密钥
/// 如果密钥链不可用，则基于主机名派生密钥（降级方案）
fn derive_key() -> [u8; 32] {
    // Try to use system keychain
    if let Ok(entry) = keyring::Entry::new("floralis", "aes-key") {
        if let Ok(stored_key) = entry.get_secret() {
            if stored_key.len() == 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&stored_key);
                return key;
            }
        }
        // 使用操作系统 CSPRNG 生成 32 字节密钥，避免时间戳可预测
        let mut key = [0u8; 32];
        if getrandom::getrandom(&mut key).is_ok() {
            let _ = entry.set_secret(&key);
            return key;
        }
        // 随机源不可用时降级到基于主机名派生
    }
    
    // Fallback: hostname-based derivation
    let host = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "floralis-default".to_string());
    let mut hasher = Sha256::new();
    hasher.update(b"floralis-pwd-key-v1:");
    hasher.update(host.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// AES-256-GCM 加密密码
/// 
/// 返回 base64(nonce + ciphertext) 格式的加密字符串
fn encrypt_password(plaintext: &str) -> std::result::Result<String, String> {
    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;

    // 使用 CSPRNG 生成 12 字节随机 nonce，避免时间戳/计数器复用风险
    let mut nonce_bytes = [0u8; 12];
    getrandom::getrandom(&mut nonce_bytes).map_err(|e| e.to_string())?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes()).map_err(|e| e.to_string())?;

    // Store as base64(nonce + ciphertext)
    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(B64.encode(&combined))
}

/// AES-256-GCM 解密密码
/// 
/// 输入格式: base64(nonce + ciphertext)
fn decrypt_password(encrypted: &str) -> std::result::Result<String, String> {
    let combined = B64.decode(encrypted).map_err(|e| format!("base64 decode error: {}", e))?;
    if combined.len() < 13 {
        return Err("encrypted data too short".to_string());
    }

    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;

    let nonce = Nonce::from_slice(&combined[..12]);
    let ciphertext = &combined[12..];

    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| format!("decrypt error: {}", e))?;
    String::from_utf8(plaintext).map_err(|e| e.to_string())
}

/// 游戏数据结构
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Game {
    pub id: i64,
    pub name: String,
    pub group_id: Option<i64>,
    pub install_path: String,
    pub exe_path: String,
    pub launch_args: String,
    pub cover_path: String,
    pub save_path: String,
    pub notes: String,
    pub script_path: String,
    pub script_args: String,
    pub total_play_time: i64,
    pub last_played_at: Option<String>,
    pub status: String,
    pub rating: i64,
    pub sort_order: i64,
    #[serde(default)]
    pub default_mod_dir: String,
    #[serde(default)]
    pub mod_naming_pattern: String,
    #[serde(default)]
    pub mod_uses_load_order: bool,
    /// 自定义追踪进程名（小写匹配，可含/不含 .exe），用于启动器与游戏本体分离等场景
    #[serde(default)]
    pub tracked_process_name: String,
}

/// 附加启动入口（一个游戏可配置多个，如汉化版、配置工具、不同参数）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LaunchAction {
    pub id: i64,
    pub game_id: i64,
    pub name: String,
    pub program_path: String,
    pub args: String,
    pub sort_order: i64,
}

/// 游戏会话记录
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlaySession {
    pub id: i64,
    pub game_id: i64,
    pub start_time: String,
    pub end_time: Option<String>,
    pub duration_seconds: i64,
}

/// 标签数据结构
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tag {
    pub id: i64,
    pub name: String,
}

/// 标签使用情况（含游戏/Mod 引用计数）
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TagUsage {
    pub id: i64,
    pub name: String,
    pub game_count: i64,
    pub mod_count: i64,
}

/// 分组数据结构
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
}

/// Mod（模组）数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mod {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub mod_path: String,
    pub install_path: String,
    pub game_id: Option<i64>,
    pub game_dir: String,
    pub version: String,
    pub author: String,
    pub is_enabled: bool,
    pub sort_order: i32,
    pub category: String,
    pub source_url: String,
    #[serde(default)]
    pub cover_path: String,
    #[serde(default = "default_mod_type")]
    pub mod_type: String,
    #[serde(default)]
    pub original_name: String,
    pub created_at: String,
    pub updated_at: String,
}

fn default_mod_type() -> String {
    "file".to_string()
}

/// Mod 配置文件（按游戏维护多套启用组合）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModProfile {
    pub id: i64,
    pub game_id: i64,
    pub name: String,
    pub mod_ids: Vec<i64>,
    pub created_at: String,
    pub updated_at: String,
}

/// 应用设置数据结构
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    pub seven_zip_path: String,
    pub default_extract_path: String,
    pub custom_banner: String,
    pub custom_sidebar_bg: String,
    pub custom_empty_illustration: String,
    pub theme: String,
    pub update_repo: String,
    pub close_behavior: String,
    pub igdb_client_id: String,
    pub igdb_client_secret: String,
    pub image_blur: String,
    pub banner_blur: String,
    pub banner_brightness: String,
    pub sidebar_blur: String,
    pub sidebar_brightness: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            seven_zip_path: String::new(),
            default_extract_path: String::new(),
            custom_banner: String::new(),
            custom_sidebar_bg: String::new(),
            custom_empty_illustration: String::new(),
            theme: "light".to_string(),
            update_repo: String::new(),
            close_behavior: "ask".to_string(),
            igdb_client_id: String::new(),
            igdb_client_secret: String::new(),
            image_blur: "0".to_string(),
            banner_blur: "0".to_string(),
            banner_brightness: "100".to_string(),
            sidebar_blur: "0".to_string(),
            sidebar_brightness: "100".to_string(),
        }
    }
}

/// 游戏统计信息
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameStats {
    pub total_games: i64,
    pub total_play_time: i64,
    pub not_played: i64,
    pub playing: i64,
    pub completed: i64,
    pub shelved: i64,
}

/// 数据库封装结构
/// 
/// 使用 Mutex 保证 SQLite 连接的线程安全
pub struct Database {
    conn: Mutex<Connection>,
    db_path: PathBuf,
}

// Safety: Connection is behind a Mutex
unsafe impl Sync for Database {}

/// 当前数据库 schema 版本号，新增迁移时递增
const SCHEMA_VERSION: i64 = 4;

/// 单条列迁移定义：(表名, 列名, 列类型)
type ColumnMigration = (&'static str, &'static str, &'static str);

/// 幂等地为表添加列：若列已存在则跳过，兼容旧库
fn add_column(conn: &Connection, table: &str, column: &str, col_type: &str) -> Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let exists = stmt
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<String>>>()?
        .iter()
        .any(|n| n == column);
    if !exists {
        conn.execute_batch(&format!("ALTER TABLE {} ADD COLUMN {} {};", table, column, col_type))?;
    }
    Ok(())
}

impl Database {
    /// 创建新的数据库连接并初始化表结构
    pub fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Database {
            conn: Mutex::new(conn),
            db_path: path.to_path_buf(),
        };
        db.init_tables()?;
        Ok(db)
    }

    pub fn get_db_path(&self) -> PathBuf {
        self.db_path.clone()
    }

    /// 获取数据库连接锁
    ///
    /// 若锁中毒（持有者线程 panic）则恢复内部连接，避免直接 panic 导致应用崩溃
    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 初始化数据库表结构
    /// 
    /// 创建所有必要的表，并执行数据库迁移（添加新列）
    fn init_tables(&self) -> Result<()> {
        let conn = self.conn();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                sort_order INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS games (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                group_id INTEGER,
                install_path TEXT NOT NULL DEFAULT '',
                exe_path TEXT NOT NULL DEFAULT '',
                launch_args TEXT NOT NULL DEFAULT '',
                cover_path TEXT NOT NULL DEFAULT '',
                save_path TEXT NOT NULL DEFAULT '',
                notes TEXT NOT NULL DEFAULT '',
                script_path TEXT NOT NULL DEFAULT '',
                script_args TEXT NOT NULL DEFAULT '',
                created_at TEXT DEFAULT (datetime('now','localtime')),
                FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE SET NULL
            );
            CREATE TABLE IF NOT EXISTS passwords (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                password TEXT NOT NULL UNIQUE,
                sort_order INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL DEFAULT ''
            );
            CREATE TABLE IF NOT EXISTS play_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id INTEGER NOT NULL,
                start_time TEXT NOT NULL,
                end_time TEXT,
                duration_seconds INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE
            );
            CREATE TABLE IF NOT EXISTS game_tags (
                game_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (game_id, tag_id),
                FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS game_screenshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
            );
CREATE TABLE IF NOT EXISTS launch_actions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                program_path TEXT NOT NULL DEFAULT '',
                args TEXT NOT NULL DEFAULT '',
                sort_order INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS mods (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                name          TEXT NOT NULL,
                description   TEXT NOT NULL DEFAULT '',
                mod_path      TEXT NOT NULL DEFAULT '',
                install_path  TEXT NOT NULL DEFAULT '',
                game_id       INTEGER,
                game_dir      TEXT NOT NULL DEFAULT '',
                version       TEXT NOT NULL DEFAULT '',
                author        TEXT NOT NULL DEFAULT '',
                is_enabled    INTEGER NOT NULL DEFAULT 1,
                sort_order    INTEGER NOT NULL DEFAULT 0,
                created_at    TEXT DEFAULT (datetime('now','localtime')),
                updated_at    TEXT DEFAULT (datetime('now','localtime')),
                FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE SET NULL
            );
            CREATE TABLE IF NOT EXISTS mod_tags (
                mod_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (mod_id, tag_id),
                FOREIGN KEY (mod_id) REFERENCES mods(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );
CREATE TABLE IF NOT EXISTS mod_profiles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now','localtime')),
                updated_at TEXT DEFAULT (datetime('now','localtime')),
                FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
            );
CREATE TABLE IF NOT EXISTS mod_profile_mods (
                profile_id INTEGER NOT NULL,
                mod_id INTEGER NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (profile_id, mod_id),
                FOREIGN KEY (profile_id) REFERENCES mod_profiles(id) ON DELETE CASCADE,
                FOREIGN KEY (mod_id) REFERENCES mods(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_mod_profiles_game_id ON mod_profiles(game_id);
            CREATE INDEX IF NOT EXISTS idx_mods_game_id ON mods(game_id);
            CREATE INDEX IF NOT EXISTS idx_mods_sort_order ON mods(sort_order);
            CREATE INDEX IF NOT EXISTS idx_mods_is_enabled ON mods(is_enabled);
            ",
        )?;
        // ===== 版本化迁移 =====
        // 通过 PRAGMA user_version 记录 schema 版本，仅在版本升级时执行；
        // add_column 幂等处理，兼容旧库已有列的情况
        let current_version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        // 格式：(目标版本, &[(表, 列, 类型)])
        let migrations: &[(i64, &[ColumnMigration])] = &[
            (1, &[
                ("games", "script_path", "TEXT NOT NULL DEFAULT ''"),
                ("games", "script_args", "TEXT NOT NULL DEFAULT ''"),
                ("games", "total_play_time", "INTEGER NOT NULL DEFAULT 0"),
                ("games", "last_played_at", "TEXT"),
                ("games", "status", "TEXT NOT NULL DEFAULT 'not_played'"),
                ("games", "rating", "INTEGER NOT NULL DEFAULT 0"),
                ("games", "sort_order", "INTEGER NOT NULL DEFAULT 0"),
                ("games", "default_mod_dir", "TEXT NOT NULL DEFAULT ''"),
                ("games", "mod_naming_pattern", "TEXT NOT NULL DEFAULT ''"),
                ("games", "mod_uses_load_order", "INTEGER NOT NULL DEFAULT 0"),
                ("mods", "category", "TEXT NOT NULL DEFAULT ''"),
                ("mods", "source_url", "TEXT NOT NULL DEFAULT ''"),
                ("mods", "cover_path", "TEXT NOT NULL DEFAULT ''"),
                ("mods", "mod_type", "TEXT NOT NULL DEFAULT 'file'"),
                ("mods", "original_name", "TEXT NOT NULL DEFAULT ''"),
            ]),
            (2, &[
                ("games", "tracked_process_name", "TEXT NOT NULL DEFAULT ''"),
            ]),
            // v3：新增 launch_actions 表（已在 init_tables 中用 CREATE TABLE IF NOT EXISTS 建表，无列迁移）
            (3, &[]),
// v4：新增 mod_profiles / mod_profile_mods 表（已在 init_tables 中用 CREATE TABLE IF NOT EXISTS 建表，无列迁移）
            (4, &[]),
        ];
        for (version, columns) in migrations {
            if *version > current_version {
                for (table, column, col_type) in *columns {
                    add_column(&conn, table, column, col_type)?;
                }
            }
        }
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        Ok(())
    }

    // ===== Statistics =====

    pub fn get_game_stats(&self) -> Result<GameStats> {
        let conn = self.conn();
        let total_games: i64 = conn.query_row("SELECT COUNT(*) FROM games", [], |r| r.get(0))?;
        let total_play_time: i64 = conn.query_row("SELECT COALESCE(SUM(total_play_time), 0) FROM games", [], |r| r.get(0))?;
        let not_played: i64 = conn.query_row("SELECT COUNT(*) FROM games WHERE status='not_played'", [], |r| r.get(0))?;
        let playing: i64 = conn.query_row("SELECT COUNT(*) FROM games WHERE status='playing'", [], |r| r.get(0))?;
        let completed: i64 = conn.query_row("SELECT COUNT(*) FROM games WHERE status='completed'", [], |r| r.get(0))?;
        let shelved: i64 = conn.query_row("SELECT COUNT(*) FROM games WHERE status='shelved'", [], |r| r.get(0))?;
        Ok(GameStats {
            total_games,
            total_play_time,
            not_played,
            playing,
            completed,
            shelved,
        })
    }

    // ===== Settings =====

    pub fn get_settings(&self) -> Result<AppSettings> {
        let conn = self.conn();
        let mut s = AppSettings::default();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (k, v) = row?;
            match k.as_str() {
                "seven_zip_path" => s.seven_zip_path = v,
                "default_extract_path" => s.default_extract_path = v,
                "custom_banner" => s.custom_banner = v,
                "custom_sidebar_bg" => s.custom_sidebar_bg = v,
                "custom_empty_illustration" => s.custom_empty_illustration = v,
                "theme" => s.theme = v,
                "update_repo" => s.update_repo = v,
                "close_behavior" => s.close_behavior = v,
                "igdb_client_id" => s.igdb_client_id = v,
                "igdb_client_secret" => s.igdb_client_secret = v,
                "image_blur" => s.image_blur = v,
                "banner_blur" => s.banner_blur = v,
                "banner_brightness" => s.banner_brightness = v,
                "sidebar_blur" => s.sidebar_blur = v,
                "sidebar_brightness" => s.sidebar_brightness = v,
                _ => {}
            }
        }
        Ok(s)
    }

    pub fn save_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    // ===== Passwords =====

    pub fn get_passwords(&self) -> Result<Vec<String>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT password FROM passwords ORDER BY sort_order, id")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let result: Vec<String> = rows.filter_map(|r| {
            let encrypted = r.ok()?;
            // Try to decrypt; if fails (legacy data), return as-is
            Some(decrypt_password(&encrypted).unwrap_or(encrypted))
        }).collect();
        Ok(result)
    }

    pub fn add_password(&self, password: &str) -> Result<()> {
        let encrypted = encrypt_password(password).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(e))))?;
        let conn = self.conn();
        let max_order: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) FROM passwords",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        conn.execute(
            "INSERT INTO passwords (password, sort_order) VALUES (?1, ?2)",
            params![encrypted, max_order + 1],
        )?;
        Ok(())
    }

    pub fn remove_password(&self, password: &str) -> Result<()> {
        // Decrypt all stored passwords and find the matching one to delete
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT password FROM passwords")?;
        let encrypted_passwords: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();

        for encrypted in &encrypted_passwords {
            if let Ok(decrypted) = decrypt_password(encrypted) {
                if decrypted == password {
                    conn.execute("DELETE FROM passwords WHERE password = ?1", params![encrypted])?;
                    return Ok(());
                }
            }
        }
        // Fallback: try direct match (legacy unencrypted data)
        conn.execute("DELETE FROM passwords WHERE password = ?1", params![password])?;
        Ok(())
    }

    // ===== Games =====

    pub fn get_all_games(&self) -> Result<Vec<Game>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id,name,group_id,install_path,exe_path,launch_args,cover_path,save_path,notes,script_path,script_args,total_play_time,last_played_at,status,rating,sort_order,default_mod_dir,mod_naming_pattern,mod_uses_load_order,tracked_process_name \
             FROM games ORDER BY sort_order ASC, created_at DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Game {
                id: r.get(0)?,
                name: r.get(1)?,
                group_id: r.get(2)?,
                install_path: r.get(3)?,
                exe_path: r.get(4)?,
                launch_args: r.get(5)?,
                cover_path: r.get(6)?,
                save_path: r.get(7)?,
                notes: r.get(8)?,
                script_path: r.get(9)?,
                script_args: r.get(10)?,
                total_play_time: r.get(11)?,
                last_played_at: r.get(12)?,
                status: r.get(13)?,
                rating: r.get(14)?,
                sort_order: r.get(15)?,
                default_mod_dir: r.get(16)?,
                mod_naming_pattern: r.get(17)?,
                mod_uses_load_order: r.get(18)?,
                tracked_process_name: r.get(19)?,
            })
        })?;
        let result: Vec<Game> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    pub fn get_game_by_id(&self, id: i64) -> Result<Option<Game>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id,name,group_id,install_path,exe_path,launch_args,cover_path,save_path,notes,script_path,script_args,total_play_time,last_played_at,status,rating,sort_order,default_mod_dir,mod_naming_pattern,mod_uses_load_order,tracked_process_name \
             FROM games WHERE id=?1",
        )?;
        match stmt.query_row(params![id], |r| {
            Ok(Game {
                id: r.get(0)?,
                name: r.get(1)?,
                group_id: r.get(2)?,
                install_path: r.get(3)?,
                exe_path: r.get(4)?,
                launch_args: r.get(5)?,
                cover_path: r.get(6)?,
                save_path: r.get(7)?,
                notes: r.get(8)?,
                script_path: r.get(9)?,
                script_args: r.get(10)?,
                total_play_time: r.get(11)?,
                last_played_at: r.get(12)?,
                status: r.get(13)?,
                rating: r.get(14)?,
                sort_order: r.get(15)?,
                default_mod_dir: r.get(16)?,
                mod_naming_pattern: r.get(17)?,
                mod_uses_load_order: r.get(18)?,
                tracked_process_name: r.get(19)?,
            })
        }) {
            Ok(g) => Ok(Some(g)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_game(
        &self,
        name: &str,
        group_id: Option<i64>,
        install_path: &str,
        exe_path: &str,
        launch_args: &str,
        cover_path: &str,
        save_path: &str,
        notes: &str,
        script_path: &str,
        script_args: &str,
    ) -> Result<i64> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO games (name,group_id,install_path,exe_path,launch_args,cover_path,save_path,notes,script_path,script_args) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![name, group_id, install_path, exe_path, launch_args, cover_path, save_path, notes, script_path, script_args],
        )?;
        Ok(conn.last_insert_rowid())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_game(
        &self,
        id: i64,
        name: &str,
        group_id: Option<i64>,
        install_path: &str,
        exe_path: &str,
        launch_args: &str,
        cover_path: &str,
        save_path: &str,
        notes: &str,
        script_path: &str,
        script_args: &str,
        default_mod_dir: &str,
        mod_naming_pattern: &str,
        mod_uses_load_order: bool,
        tracked_process_name: &str,
    ) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE games SET name=?1,group_id=?2,install_path=?3,exe_path=?4,launch_args=?5,cover_path=?6,save_path=?7,notes=?8,script_path=?9,script_args=?10,default_mod_dir=?11,mod_naming_pattern=?12,mod_uses_load_order=?13,tracked_process_name=?14 WHERE id=?15",
            params![name, group_id, install_path, exe_path, launch_args, cover_path, save_path, notes, script_path, script_args, default_mod_dir, mod_naming_pattern, mod_uses_load_order, tracked_process_name, id],
        )?;
        Ok(())
    }

    /// 手动修正游戏总游玩时长（秒），不改动历史会话记录
    pub fn set_game_play_time(&self, id: i64, seconds: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE games SET total_play_time = ?1 WHERE id = ?2",
            params![seconds.max(0), id],
        )?;
        Ok(())
    }

    pub fn update_game_cover(&self, id: i64, cover_path: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE games SET cover_path=?1 WHERE id=?2",
            params![cover_path, id],
        )?;
        Ok(())
    }

    pub fn update_game_save_path(&self, id: i64, save_path: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE games SET save_path=?1 WHERE id=?2",
            params![save_path, id],
        )?;
        Ok(())
    }

    /// 库重定位：同时更新安装目录与主程序路径
    pub fn update_game_relocate(&self, id: i64, install_path: &str, exe_path: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE games SET install_path=?1, exe_path=?2 WHERE id=?3",
            params![install_path, exe_path, id],
        )?;
        Ok(())
    }

    pub fn delete_game(&self, id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM games WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn batch_delete_games(&self, ids: &[i64]) -> Result<()> {
        let conn = self.conn();
        for id in ids {
            conn.execute("DELETE FROM games WHERE id=?1", params![id])?;
        }
        Ok(())
    }

    pub fn batch_set_game_group(&self, game_ids: &[i64], group_id: Option<i64>) -> Result<()> {
        let conn = self.conn();
        for id in game_ids {
            conn.execute(
                "UPDATE games SET group_id=?1 WHERE id=?2",
                params![group_id, id],
            )?;
        }
        Ok(())
    }

    pub fn set_game_status(&self, game_id: i64, status: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE games SET status=?1 WHERE id=?2",
            params![status, game_id],
        )?;
        Ok(())
    }

    pub fn set_game_rating(&self, game_id: i64, rating: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE games SET rating=?1 WHERE id=?2",
            params![rating, game_id],
        )?;
        Ok(())
    }

    pub fn batch_set_game_status(&self, game_ids: &[i64], status: &str) -> Result<()> {
        let conn = self.conn();
        for id in game_ids {
            conn.execute(
                "UPDATE games SET status=?1 WHERE id=?2",
                params![status, id],
            )?;
        }
        Ok(())
    }

    pub fn batch_set_game_rating(&self, game_ids: &[i64], rating: i64) -> Result<()> {
        let conn = self.conn();
        for id in game_ids {
            conn.execute(
                "UPDATE games SET rating=?1 WHERE id=?2",
                params![rating, id],
            )?;
        }
        Ok(())
    }

    pub fn set_game_group(&self, game_id: i64, group_id: Option<i64>) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE games SET group_id=?1 WHERE id=?2",
            params![group_id, game_id],
        )?;
        Ok(())
    }

    // ===== Groups =====

    pub fn get_all_groups(&self) -> Result<Vec<Group>> {
        let conn = self.conn();
        let mut stmt =
            conn.prepare("SELECT id,name,sort_order FROM groups ORDER BY sort_order,id")?;
        let rows = stmt.query_map([], |r| {
            Ok(Group {
                id: r.get(0)?,
                name: r.get(1)?,
                sort_order: r.get(2)?,
            })
        })?;
        let result: Vec<Group> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    pub fn add_group(&self, name: &str) -> Result<i64> {
        let conn = self.conn();
        let max_order: i64 = conn
            .query_row("SELECT COALESCE(MAX(sort_order),0) FROM groups", [], |r| {
                r.get(0)
            })
            .unwrap_or(0);
        conn.execute(
            "INSERT INTO groups (name,sort_order) VALUES (?1,?2)",
            params![name, max_order + 1],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn rename_group(&self, id: i64, name: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE groups SET name=?1 WHERE id=?2",
            params![name, id],
        )?;
        Ok(())
    }

    pub fn delete_group(&self, id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE games SET group_id=NULL WHERE group_id=?1",
            params![id],
        )?;
        conn.execute("DELETE FROM groups WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn reorder_groups(&self, ordered_ids: &[i64]) -> Result<()> {
        let conn = self.conn();
        for (i, id) in ordered_ids.iter().enumerate() {
            conn.execute(
                "UPDATE groups SET sort_order=?1 WHERE id=?2",
                params![i as i64, id],
            )?;
        }
        Ok(())
    }

    pub fn clear_all(&self) -> Result<()> {
        let conn = self.conn();
        conn.execute_batch(
            "DELETE FROM mod_tags; DELETE FROM mods; DELETE FROM game_tags; DELETE FROM tags; DELETE FROM play_sessions; DELETE FROM games; DELETE FROM groups; DELETE FROM passwords; DELETE FROM settings;",
        )?;
        Ok(())
    }

    // ===== Play Sessions =====

    pub fn start_play_session(&self, game_id: i64, start_time: &str) -> Result<i64> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO play_sessions (game_id, start_time, duration_seconds) VALUES (?1, ?2, 0)",
            params![game_id, start_time],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 增量落盘游玩时长：会话进行中累加时长（同时更新游戏总时长与最后游玩时间）
    pub fn increment_play_time(&self, session_id: i64, game_id: i64, delta_seconds: i64, now: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE play_sessions SET duration_seconds = duration_seconds + ?1 WHERE id = ?2",
            params![delta_seconds, session_id],
        )?;
        conn.execute(
            "UPDATE games SET total_play_time = total_play_time + ?1, last_played_at = ?2 WHERE id = ?3",
            params![delta_seconds, now, game_id],
        )?;
        Ok(())
    }

    /// 关闭游玩会话：仅写入结束时间（时长已在运行期间增量落盘）
    pub fn close_play_session(&self, session_id: i64, end_time: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE play_sessions SET end_time = ?1 WHERE id = ?2",
            params![end_time, session_id],
        )?;
        Ok(())
    }

    /// 查询所有未闭合（end_time 为空）的游玩会话
    pub fn get_open_sessions(&self) -> Result<Vec<PlaySession>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, game_id, start_time, end_time, duration_seconds \
             FROM play_sessions WHERE end_time IS NULL ORDER BY id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(PlaySession {
                id: r.get(0)?,
                game_id: r.get(1)?,
                start_time: r.get(2)?,
                end_time: r.get(3)?,
                duration_seconds: r.get(4)?,
            })
        })?;
        let result: Vec<PlaySession> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    pub fn get_play_sessions(&self, game_id: i64, limit: i64) -> Result<Vec<PlaySession>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, game_id, start_time, end_time, duration_seconds \
             FROM play_sessions WHERE game_id=?1 ORDER BY start_time DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![game_id, limit], |r| {
            Ok(PlaySession {
                id: r.get(0)?,
                game_id: r.get(1)?,
                start_time: r.get(2)?,
                end_time: r.get(3)?,
                duration_seconds: r.get(4)?,
            })
        })?;
        let result: Vec<PlaySession> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    pub fn get_play_calendar(&self, year: i32, month: u32) -> Result<Vec<(String, i64)>> {
        let conn = self.conn();
        let start_date = format!("{:04}-{:02}-01", year, month);
        let end_month = if month == 12 { 1 } else { month + 1 };
        let end_year = if month == 12 { year + 1 } else { year };
        let end_date = format!("{:04}-{:02}-01", end_year, end_month);
        
        let mut stmt = conn.prepare(
            "SELECT DATE(start_time) as play_date, SUM(duration_seconds) \
             FROM play_sessions \
             WHERE start_time >= ?1 AND start_time < ?2 \
             GROUP BY play_date \
             ORDER BY play_date",
        )?;
        let rows = stmt.query_map(params![start_date, end_date], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?;
        let result: Vec<(String, i64)> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    // ===== Tags =====

    pub fn get_all_tags(&self) -> Result<Vec<Tag>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT id, name FROM tags ORDER BY name")?;
        let rows = stmt.query_map([], |r| {
            Ok(Tag {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })?;
        let result: Vec<Tag> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    pub fn add_tag(&self, name: &str) -> Result<i64> {
        let conn = self.conn();
        conn.execute(
            "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
            params![name],
        )?;
        // Get the tag id (whether newly inserted or already existed)
        let id: i64 = conn.query_row(
            "SELECT id FROM tags WHERE name=?1",
            params![name],
            |r| r.get(0),
        )?;
        Ok(id)
    }

    pub fn delete_tag(&self, id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM tags WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn rename_tag(&self, id: i64, name: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute("UPDATE tags SET name=?1 WHERE id=?2", params![name, id])?;
        Ok(())
    }

    pub fn get_tag_usage(&self) -> Result<Vec<TagUsage>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, \
             (SELECT COUNT(*) FROM game_tags gt WHERE gt.tag_id = t.id), \
             (SELECT COUNT(*) FROM mod_tags mt WHERE mt.tag_id = t.id) \
             FROM tags t ORDER BY t.name",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(TagUsage {
                id: r.get(0)?,
                name: r.get(1)?,
                game_count: r.get(2)?,
                mod_count: r.get(3)?,
            })
        })?;
        let result: Vec<TagUsage> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    pub fn get_game_tags(&self, game_id: i64) -> Result<Vec<Tag>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name FROM tags t \
             INNER JOIN game_tags gt ON gt.tag_id = t.id \
             WHERE gt.game_id=?1 ORDER BY t.name",
        )?;
        let rows = stmt.query_map(params![game_id], |r| {
            Ok(Tag {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })?;
        let result: Vec<Tag> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    pub fn add_game_tag(&self, game_id: i64, tag_id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT OR IGNORE INTO game_tags (game_id, tag_id) VALUES (?1, ?2)",
            params![game_id, tag_id],
        )?;
        Ok(())
    }

    pub fn remove_game_tag(&self, game_id: i64, tag_id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM game_tags WHERE game_id=?1 AND tag_id=?2",
            params![game_id, tag_id],
        )?;
        Ok(())
    }

    // ===== Launch Actions（附加启动入口） =====

    pub fn get_launch_actions(&self, game_id: i64) -> Result<Vec<LaunchAction>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, game_id, name, program_path, args, sort_order \
             FROM launch_actions WHERE game_id=?1 ORDER BY sort_order, id",
        )?;
        let rows = stmt.query_map(params![game_id], |r| {
            Ok(LaunchAction {
                id: r.get(0)?,
                game_id: r.get(1)?,
                name: r.get(2)?,
                program_path: r.get(3)?,
                args: r.get(4)?,
                sort_order: r.get(5)?,
            })
        })?;
        let result: Vec<LaunchAction> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    pub fn get_launch_action_by_id(&self, id: i64) -> Result<Option<LaunchAction>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, game_id, name, program_path, args, sort_order \
             FROM launch_actions WHERE id=?1",
        )?;
        let mut rows = stmt.query_map(params![id], |r| {
            Ok(LaunchAction {
                id: r.get(0)?,
                game_id: r.get(1)?,
                name: r.get(2)?,
                program_path: r.get(3)?,
                args: r.get(4)?,
                sort_order: r.get(5)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    }

    /// 整体替换某游戏的附加启动入口（编辑对话框一次提交）
    pub fn replace_launch_actions(
        &self,
        game_id: i64,
        actions: &[LaunchAction],
    ) -> Result<Vec<LaunchAction>> {
        let conn = self.conn();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM launch_actions WHERE game_id=?1",
            params![game_id],
        )?;
        for (i, a) in actions.iter().enumerate() {
            tx.execute(
                "INSERT INTO launch_actions (game_id, name, program_path, args, sort_order) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![game_id, a.name, a.program_path, a.args, i as i64],
            )?;
        }
        tx.commit()?;
        self.get_launch_actions(game_id)
    }

    pub fn get_all_game_tags(&self) -> Result<HashMap<i64, Vec<Tag>>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT gt.game_id, t.id, t.name FROM game_tags gt \
             INNER JOIN tags t ON t.id = gt.tag_id \
             ORDER BY t.name",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                Tag {
                    id: r.get(1)?,
                    name: r.get(2)?,
                },
            ))
        })?;
        let mut map: HashMap<i64, Vec<Tag>> = HashMap::new();
        for row in rows {
            let (game_id, tag) = row?;
            map.entry(game_id).or_default().push(tag);
        }
        Ok(map)
    }

    // 供后续按标签检索游戏使用，当前前端在内存中完成过滤
    #[allow(dead_code)]
    pub fn get_game_ids_by_tag(&self, tag_id: i64) -> Result<Vec<i64>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT game_id FROM game_tags WHERE tag_id=?1")?;
        let rows = stmt.query_map(params![tag_id], |r| r.get::<_, i64>(0))?;
        let result: Vec<i64> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    // ===== Screenshots =====

    pub fn get_game_screenshots(&self, game_id: i64) -> Result<Vec<(i64, String)>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, path FROM game_screenshots WHERE game_id=?1 ORDER BY sort_order, id",
        )?;
        let rows = stmt.query_map(params![game_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        let result: Vec<(i64, String)> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    pub fn add_game_screenshot(&self, game_id: i64, path: &str) -> Result<i64> {
        let conn = self.conn();
        let max_order: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) FROM game_screenshots WHERE game_id=?1",
                params![game_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        conn.execute(
            "INSERT INTO game_screenshots (game_id, path, sort_order) VALUES (?1, ?2, ?3)",
            params![game_id, path, max_order + 1],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn delete_game_screenshot(&self, screenshot_id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM game_screenshots WHERE id=?1", params![screenshot_id])?;
        Ok(())
    }

    pub fn reorder_games(&self, game_ids: &[i64]) -> Result<()> {
        let conn = self.conn();
        for (i, id) in game_ids.iter().enumerate() {
            conn.execute(
                "UPDATE games SET sort_order = ?1 WHERE id = ?2",
                params![i as i64, id],
            )?;
        }
        Ok(())
    }

    // ===== Mods =====

    pub fn get_all_mods(&self) -> Result<Vec<Mod>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id,name,description,mod_path,install_path,game_id,game_dir,version,author,is_enabled,sort_order,category,source_url,cover_path,mod_type,original_name,created_at,updated_at \
             FROM mods ORDER BY sort_order",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Mod {
                id: r.get(0)?,
                name: r.get(1)?,
                description: r.get(2)?,
                mod_path: r.get(3)?,
                install_path: r.get(4)?,
                game_id: r.get(5)?,
                game_dir: r.get(6)?,
                version: r.get(7)?,
                author: r.get(8)?,
                is_enabled: r.get(9)?,
                sort_order: r.get(10)?,
                category: r.get(11)?,
                source_url: r.get(12)?,
                cover_path: r.get(13)?,
                mod_type: r.get(14)?,
                original_name: r.get(15)?,
                created_at: r.get(16)?,
                updated_at: r.get(17)?,
            })
        })?;
        let result: Vec<Mod> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    pub fn get_mods_by_game(&self, game_id: i64) -> Result<Vec<Mod>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id,name,description,mod_path,install_path,game_id,game_dir,version,author,is_enabled,sort_order,category,source_url,cover_path,mod_type,original_name,created_at,updated_at \
             FROM mods WHERE game_id=?1 ORDER BY sort_order",
        )?;
        let rows = stmt.query_map(params![game_id], |r| {
            Ok(Mod {
                id: r.get(0)?,
                name: r.get(1)?,
                description: r.get(2)?,
                mod_path: r.get(3)?,
                install_path: r.get(4)?,
                game_id: r.get(5)?,
                game_dir: r.get(6)?,
                version: r.get(7)?,
                author: r.get(8)?,
                is_enabled: r.get(9)?,
                sort_order: r.get(10)?,
                category: r.get(11)?,
                source_url: r.get(12)?,
                cover_path: r.get(13)?,
                mod_type: r.get(14)?,
                original_name: r.get(15)?,
                created_at: r.get(16)?,
                updated_at: r.get(17)?,
            })
        })?;
        let result: Vec<Mod> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    pub fn get_mods_by_game_dir(&self, game_dir: &str) -> Result<Vec<Mod>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id,name,description,mod_path,install_path,game_id,game_dir,version,author,is_enabled,sort_order,category,source_url,cover_path,mod_type,original_name,created_at,updated_at \
             FROM mods WHERE game_dir=?1 ORDER BY sort_order",
        )?;
        let rows = stmt.query_map(params![game_dir], |r| {
            Ok(Mod {
                id: r.get(0)?,
                name: r.get(1)?,
                description: r.get(2)?,
                mod_path: r.get(3)?,
                install_path: r.get(4)?,
                game_id: r.get(5)?,
                game_dir: r.get(6)?,
                version: r.get(7)?,
                author: r.get(8)?,
                is_enabled: r.get(9)?,
                sort_order: r.get(10)?,
                category: r.get(11)?,
                source_url: r.get(12)?,
                cover_path: r.get(13)?,
                mod_type: r.get(14)?,
                original_name: r.get(15)?,
                created_at: r.get(16)?,
                updated_at: r.get(17)?,
            })
        })?;
        let result: Vec<Mod> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_mod(
        &self,
        name: &str,
        description: &str,
        mod_path: &str,
        install_path: &str,
        game_id: Option<i64>,
        game_dir: &str,
        version: &str,
        author: &str,
        is_enabled: bool,
        sort_order: i32,
        category: &str,
        source_url: &str,
        cover_path: &str,
        mod_type: &str,
        original_name: &str,
    ) -> Result<i64> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO mods (name,description,mod_path,install_path,game_id,game_dir,version,author,is_enabled,sort_order,category,source_url,cover_path,mod_type,original_name) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![name, description, mod_path, install_path, game_id, game_dir, version, author, is_enabled, sort_order, category, source_url, cover_path, mod_type, original_name],
        )?;
        Ok(conn.last_insert_rowid())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_mod(
        &self,
        id: i64,
        name: &str,
        description: &str,
        mod_path: &str,
        install_path: &str,
        game_id: Option<i64>,
        game_dir: &str,
        version: &str,
        author: &str,
        is_enabled: bool,
        sort_order: i32,
        category: &str,
        source_url: &str,
        cover_path: &str,
        mod_type: &str,
        original_name: &str,
    ) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE mods SET name=?1,description=?2,mod_path=?3,install_path=?4,game_id=?5,game_dir=?6,version=?7,author=?8,is_enabled=?9,sort_order=?10,category=?11,source_url=?12,cover_path=?13,mod_type=?14,original_name=?15,updated_at=datetime('now','localtime') WHERE id=?16",
            params![name, description, mod_path, install_path, game_id, game_dir, version, author, is_enabled, sort_order, category, source_url, cover_path, mod_type, original_name, id],
        )?;
        Ok(())
    }

    pub fn delete_mod(&self, id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM mods WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn get_mod_by_id(&self, id: i64) -> Result<Option<Mod>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id,name,description,mod_path,install_path,game_id,game_dir,version,author,is_enabled,sort_order,category,source_url,cover_path,mod_type,original_name,created_at,updated_at \
             FROM mods WHERE id=?1",
        )?;
        let mut rows = stmt.query_map(params![id], |r| {
            Ok(Mod {
                id: r.get(0)?,
                name: r.get(1)?,
                description: r.get(2)?,
                mod_path: r.get(3)?,
                install_path: r.get(4)?,
                game_id: r.get(5)?,
                game_dir: r.get(6)?,
                version: r.get(7)?,
                author: r.get(8)?,
                is_enabled: r.get(9)?,
                sort_order: r.get(10)?,
                category: r.get(11)?,
                source_url: r.get(12)?,
                cover_path: r.get(13)?,
                mod_type: r.get(14)?,
                original_name: r.get(15)?,
                created_at: r.get(16)?,
                updated_at: r.get(17)?,
            })
        })?;
        match rows.next() {
            Some(Ok(m)) => Ok(Some(m)),
            _ => Ok(None),
        }
    }

    pub fn set_mod_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE mods SET is_enabled=?1, updated_at=datetime('now','localtime') WHERE id=?2",
            params![enabled, id],
        )?;
        Ok(())
    }

    pub fn update_mod_path(&self, id: i64, mod_path: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE mods SET mod_path=?1, updated_at=datetime('now','localtime') WHERE id=?2",
            params![mod_path, id],
        )?;
        Ok(())
    }

    pub fn reorder_mods(&self, mod_ids: &[i64]) -> Result<()> {
        let conn = self.conn();
        let tx = conn.unchecked_transaction()?;
        for (i, id) in mod_ids.iter().enumerate() {
            tx.execute(
                "UPDATE mods SET sort_order=?1, updated_at=datetime('now','localtime') WHERE id=?2",
                params![i as i32, id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    // ===== Mod Profiles =====

    pub fn create_mod_profile(&self, game_id: i64, name: &str, mod_ids: &[i64]) -> Result<i64> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO mod_profiles (game_id, name) VALUES (?1, ?2)",
            params![game_id, name],
        )?;
        let profile_id = conn.last_insert_rowid();
        self.set_profile_mods_inner(&conn, profile_id, mod_ids)?;
        Ok(profile_id)
    }

    pub fn rename_mod_profile(&self, id: i64, name: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE mod_profiles SET name=?1, updated_at=datetime('now','localtime') WHERE id=?2",
            params![name, id],
        )?;
        Ok(())
    }

    pub fn delete_mod_profile(&self, id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM mod_profiles WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn get_profiles_by_game(&self, game_id: i64) -> Result<Vec<ModProfile>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, game_id, name, created_at, updated_at FROM mod_profiles WHERE game_id=?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![game_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?;
        let mut profiles = Vec::new();
        for row in rows {
            let (id, gid, name, created_at, updated_at) = row?;
            let mod_ids = self.get_profile_mod_ids_inner(&conn, id)?;
            profiles.push(ModProfile { id, game_id: gid, name, mod_ids, created_at, updated_at });
        }
        Ok(profiles)
    }

    pub fn get_profile_by_id(&self, id: i64) -> Result<Option<ModProfile>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, game_id, name, created_at, updated_at FROM mod_profiles WHERE id=?1",
        )?;
        let mut rows = stmt.query_map(params![id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?;
        match rows.next() {
            Some(Ok((pid, gid, name, created_at, updated_at))) => {
                let mod_ids = self.get_profile_mod_ids_inner(&conn, pid)?;
                Ok(Some(ModProfile { id: pid, game_id: gid, name, mod_ids, created_at, updated_at }))
            }
            _ => Ok(None),
        }
    }

    pub fn set_profile_mods(&self, profile_id: i64, mod_ids: &[i64]) -> Result<()> {
        let conn = self.conn();
        self.set_profile_mods_inner(&conn, profile_id, mod_ids)?;
        conn.execute(
            "UPDATE mod_profiles SET updated_at=datetime('now','localtime') WHERE id=?1",
            params![profile_id],
        )?;
        Ok(())
    }

    fn set_profile_mods_inner(&self, conn: &Connection, profile_id: i64, mod_ids: &[i64]) -> Result<()> {
        let tx = conn.unchecked_transaction()?;
        tx.execute("DELETE FROM mod_profile_mods WHERE profile_id=?1", params![profile_id])?;
        for (i, mod_id) in mod_ids.iter().enumerate() {
            tx.execute(
                "INSERT INTO mod_profile_mods (profile_id, mod_id, sort_order) VALUES (?1, ?2, ?3)",
                params![profile_id, mod_id, i as i32],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    fn get_profile_mod_ids_inner(&self, conn: &Connection, profile_id: i64) -> Result<Vec<i64>> {
        let mut stmt = conn.prepare(
            "SELECT mod_id FROM mod_profile_mods WHERE profile_id=?1 ORDER BY sort_order ASC",
        )?;
        let ids = stmt
            .query_map(params![profile_id], |r| r.get::<_, i64>(0))?
            .collect::<rusqlite::Result<Vec<i64>>>()?;
        Ok(ids)
    }

    pub fn link_mod_to_game(&self, mod_id: i64, game_id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE mods SET game_id=?1, updated_at=datetime('now','localtime') WHERE id=?2",
            params![game_id, mod_id],
        )?;
        Ok(())
    }

    pub fn unlink_mod_from_game(&self, mod_id: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE mods SET game_id=NULL, updated_at=datetime('now','localtime') WHERE id=?1",
            params![mod_id],
        )?;
        Ok(())
    }

    pub fn get_mod_tags(&self, mod_id: i64) -> Result<Vec<Tag>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name FROM tags t \
             INNER JOIN mod_tags mt ON mt.tag_id = t.id \
             WHERE mt.mod_id=?1 ORDER BY t.name",
        )?;
        let rows = stmt.query_map(params![mod_id], |r| {
            Ok(Tag {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })?;
        let result: Vec<Tag> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    pub fn get_all_mod_tags(&self) -> Result<HashMap<i64, Vec<Tag>>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT mt.mod_id, t.id, t.name FROM mod_tags mt \
             INNER JOIN tags t ON t.id = mt.tag_id \
             ORDER BY t.name",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                Tag {
                    id: r.get(1)?,
                    name: r.get(2)?,
                },
            ))
        })?;
        let mut map: HashMap<i64, Vec<Tag>> = HashMap::new();
        for row in rows {
            let (mod_id, tag) = row?;
            map.entry(mod_id).or_default().push(tag);
        }
        Ok(map)
    }

    pub fn set_mod_tags(&self, mod_id: i64, tag_ids: &[i64]) -> Result<()> {
        let conn = self.conn();
        let tx = conn.unchecked_transaction()?;
        tx.execute("DELETE FROM mod_tags WHERE mod_id=?1", params![mod_id])?;
        for tag_id in tag_ids {
            tx.execute(
                "INSERT INTO mod_tags (mod_id, tag_id) VALUES (?1, ?2)",
                params![mod_id, tag_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_db() -> Database {
        let path = PathBuf::from(format!("test_db_{}.sqlite", std::process::id()));
        Database::new(&path).unwrap()
    }

    fn cleanup_test_db(db: &Database) {
        let _ = std::fs::remove_file(db.get_db_path());
    }

    #[test]
    fn test_add_and_get_game() {
        let db = create_test_db();
        let id = db.add_game("Test Game", None, "", "", "", "", "", "", "", "").unwrap();
        let game = db.get_game_by_id(id).unwrap().unwrap();
        assert_eq!(game.name, "Test Game");
        cleanup_test_db(&db);
    }

    #[test]
    fn test_game_status() {
        let db = create_test_db();
        let id = db.add_game("Test Game", None, "", "", "", "", "", "", "", "").unwrap();
        db.set_game_status(id, "playing").unwrap();
        let game = db.get_game_by_id(id).unwrap().unwrap();
        assert_eq!(game.status, "playing");
        cleanup_test_db(&db);
    }

    #[test]
    fn test_game_rating() {
        let db = create_test_db();
        let id = db.add_game("Test Game", None, "", "", "", "", "", "", "", "").unwrap();
        db.set_game_rating(id, 8).unwrap();
        let game = db.get_game_by_id(id).unwrap().unwrap();
        assert_eq!(game.rating, 8);
        cleanup_test_db(&db);
    }

    #[test]
    fn test_batch_operations() {
        let db = create_test_db();
        let id1 = db.add_game("Game 1", None, "", "", "", "", "", "", "", "").unwrap();
        let id2 = db.add_game("Game 2", None, "", "", "", "", "", "", "", "").unwrap();
        db.batch_set_game_status(&[id1, id2], "completed").unwrap();
        let game1 = db.get_game_by_id(id1).unwrap().unwrap();
        let game2 = db.get_game_by_id(id2).unwrap().unwrap();
        assert_eq!(game1.status, "completed");
        assert_eq!(game2.status, "completed");
        cleanup_test_db(&db);
    }

    #[test]
    fn test_password_encryption() {
        let db = create_test_db();
        db.add_password("test_password_123").unwrap();
        let passwords = db.get_passwords().unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0], "test_password_123");
        cleanup_test_db(&db);
    }

    #[test]
    fn test_tags() {
        let db = create_test_db();
        let tag_id = db.add_tag("Action").unwrap();
        let tags = db.get_all_tags().unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "Action");
        
        let game_id = db.add_game("Test Game", None, "", "", "", "", "", "", "", "").unwrap();
        db.add_game_tag(game_id, tag_id).unwrap();
        let game_tags = db.get_game_tags(game_id).unwrap();
        assert_eq!(game_tags.len(), 1);
        cleanup_test_db(&db);
    }

    #[test]
    fn test_screenshots() {
        let db = create_test_db();
        let game_id = db.add_game("Test Game", None, "", "", "", "", "", "", "", "").unwrap();
        let ss_id = db.add_game_screenshot(game_id, "/path/to/screenshot.png").unwrap();
        let screenshots = db.get_game_screenshots(game_id).unwrap();
        assert_eq!(screenshots.len(), 1);
        assert_eq!(screenshots[0].1, "/path/to/screenshot.png");
        db.delete_game_screenshot(ss_id).unwrap();
        let screenshots = db.get_game_screenshots(game_id).unwrap();
        assert_eq!(screenshots.len(), 0);
        cleanup_test_db(&db);
    }
}
