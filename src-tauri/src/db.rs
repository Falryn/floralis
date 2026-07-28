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
        // Generate and store new key in keychain
        let mut hasher = Sha256::new();
        hasher.update(b"floralis-pwd-key-v1:");
        let random_bytes: [u8; 32] = std::array::from_fn(|i| {
            ((std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() >> (i * 8)) & 0xff) as u8
        });
        hasher.update(&random_bytes);
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result);
        let _ = entry.set_secret(&key);
        return key;
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
    use std::sync::atomic::{AtomicU64, Ordering};

    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;

    // Generate 12-byte nonce (random via simple counter + random seed)
    let mut nonce_bytes = [0u8; 12];
    // Use a combination of random bytes
    let random_val = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    nonce_bytes[..8].copy_from_slice(&random_val.to_le_bytes()[..8]);
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    nonce_bytes[8..].copy_from_slice(&COUNTER.fetch_add(1, Ordering::Relaxed).to_le_bytes()[..4]);
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

/// 分组数据结构
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
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

    /// 初始化数据库表结构
    /// 
    /// 创建所有必要的表，并执行数据库迁移（添加新列）
    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
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
            ",
        )?;
        // Migration: add script columns if they don't exist
        let _ = conn.execute_batch(
            "ALTER TABLE games ADD COLUMN script_path TEXT NOT NULL DEFAULT '';",
        );
        let _ = conn.execute_batch(
            "ALTER TABLE games ADD COLUMN script_args TEXT NOT NULL DEFAULT '';",
        );
        // Migration: add play time tracking columns
        let _ = conn.execute_batch(
            "ALTER TABLE games ADD COLUMN total_play_time INTEGER NOT NULL DEFAULT 0;",
        );
        let _ = conn.execute_batch(
            "ALTER TABLE games ADD COLUMN last_played_at TEXT;",
        );
        // Migration: add status column
        let _ = conn.execute_batch(
            "ALTER TABLE games ADD COLUMN status TEXT NOT NULL DEFAULT 'not_played';",
        );
        // Migration: add rating column
        let _ = conn.execute_batch(
            "ALTER TABLE games ADD COLUMN rating INTEGER NOT NULL DEFAULT 0;",
        );
        // Migration: add sort_order column
        let _ = conn.execute_batch(
            "ALTER TABLE games ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;",
        );
        Ok(())
    }

    // ===== Statistics =====

    pub fn get_game_stats(&self) -> Result<GameStats> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
                _ => {}
            }
        }
        Ok(s)
    }

    pub fn save_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    // ===== Passwords =====

    pub fn get_passwords(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
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
        let encrypted = encrypt_password(password).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))))?;
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id,name,group_id,install_path,exe_path,launch_args,cover_path,save_path,notes,script_path,script_args,total_play_time,last_played_at,status,rating,sort_order \
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
            })
        })?;
        let result: Vec<Game> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    pub fn get_game_by_id(&self, id: i64) -> Result<Option<Game>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id,name,group_id,install_path,exe_path,launch_args,cover_path,save_path,notes,script_path,script_args,total_play_time,last_played_at,status,rating,sort_order \
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
            })
        }) {
            Ok(g) => Ok(Some(g)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

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
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO games (name,group_id,install_path,exe_path,launch_args,cover_path,save_path,notes,script_path,script_args) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![name, group_id, install_path, exe_path, launch_args, cover_path, save_path, notes, script_path, script_args],
        )?;
        Ok(conn.last_insert_rowid())
    }

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
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE games SET name=?1,group_id=?2,install_path=?3,exe_path=?4,launch_args=?5,cover_path=?6,save_path=?7,notes=?8,script_path=?9,script_args=?10 WHERE id=?11",
            params![name, group_id, install_path, exe_path, launch_args, cover_path, save_path, notes, script_path, script_args, id],
        )?;
        Ok(())
    }

    pub fn update_game_cover(&self, id: i64, cover_path: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE games SET cover_path=?1 WHERE id=?2",
            params![cover_path, id],
        )?;
        Ok(())
    }

    pub fn update_game_save_path(&self, id: i64, save_path: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE games SET save_path=?1 WHERE id=?2",
            params![save_path, id],
        )?;
        Ok(())
    }

    pub fn delete_game(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM games WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn batch_delete_games(&self, ids: &[i64]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        for id in ids {
            conn.execute("DELETE FROM games WHERE id=?1", params![id])?;
        }
        Ok(())
    }

    pub fn batch_set_game_group(&self, game_ids: &[i64], group_id: Option<i64>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        for id in game_ids {
            conn.execute(
                "UPDATE games SET group_id=?1 WHERE id=?2",
                params![group_id, id],
            )?;
        }
        Ok(())
    }

    pub fn set_game_status(&self, game_id: i64, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE games SET status=?1 WHERE id=?2",
            params![status, game_id],
        )?;
        Ok(())
    }

    pub fn set_game_rating(&self, game_id: i64, rating: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE games SET rating=?1 WHERE id=?2",
            params![rating, game_id],
        )?;
        Ok(())
    }

    pub fn batch_set_game_status(&self, game_ids: &[i64], status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        for id in game_ids {
            conn.execute(
                "UPDATE games SET status=?1 WHERE id=?2",
                params![status, id],
            )?;
        }
        Ok(())
    }

    pub fn batch_set_game_rating(&self, game_ids: &[i64], rating: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        for id in game_ids {
            conn.execute(
                "UPDATE games SET rating=?1 WHERE id=?2",
                params![rating, id],
            )?;
        }
        Ok(())
    }

    pub fn set_game_group(&self, game_id: i64, group_id: Option<i64>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE games SET group_id=?1 WHERE id=?2",
            params![group_id, game_id],
        )?;
        Ok(())
    }

    // ===== Groups =====

    pub fn get_all_groups(&self) -> Result<Vec<Group>> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE groups SET name=?1 WHERE id=?2",
            params![name, id],
        )?;
        Ok(())
    }

    pub fn delete_group(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE games SET group_id=NULL WHERE group_id=?1",
            params![id],
        )?;
        conn.execute("DELETE FROM groups WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn reorder_groups(&self, ordered_ids: &[i64]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        for (i, id) in ordered_ids.iter().enumerate() {
            conn.execute(
                "UPDATE groups SET sort_order=?1 WHERE id=?2",
                params![i as i64, id],
            )?;
        }
        Ok(())
    }

    pub fn clear_all(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "DELETE FROM game_tags; DELETE FROM tags; DELETE FROM play_sessions; DELETE FROM games; DELETE FROM groups; DELETE FROM passwords; DELETE FROM settings;",
        )?;
        Ok(())
    }

    // ===== Play Sessions =====

    pub fn start_play_session(&self, game_id: i64, start_time: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO play_sessions (game_id, start_time, duration_seconds) VALUES (?1, ?2, 0)",
            params![game_id, start_time],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn end_play_session(&self, session_id: i64, game_id: i64, end_time: &str, duration_seconds: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE play_sessions SET end_time=?1, duration_seconds=?2 WHERE id=?3",
            params![end_time, duration_seconds, session_id],
        )?;
        conn.execute(
            "UPDATE games SET total_play_time = total_play_time + ?1, last_played_at = ?2 WHERE id = ?3",
            params![duration_seconds, end_time, game_id],
        )?;
        Ok(())
    }

    pub fn get_play_sessions(&self, game_id: i64, limit: i64) -> Result<Vec<PlaySession>> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM tags WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn rename_tag(&self, id: i64, name: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE tags SET name=?1 WHERE id=?2", params![name, id])?;
        Ok(())
    }

    pub fn get_game_tags(&self, game_id: i64) -> Result<Vec<Tag>> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO game_tags (game_id, tag_id) VALUES (?1, ?2)",
            params![game_id, tag_id],
        )?;
        Ok(())
    }

    pub fn remove_game_tag(&self, game_id: i64, tag_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM game_tags WHERE game_id=?1 AND tag_id=?2",
            params![game_id, tag_id],
        )?;
        Ok(())
    }

    pub fn get_all_game_tags(&self) -> Result<HashMap<i64, Vec<Tag>>> {
        let conn = self.conn.lock().unwrap();
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

    pub fn get_game_ids_by_tag(&self, tag_id: i64) -> Result<Vec<i64>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT game_id FROM game_tags WHERE tag_id=?1")?;
        let rows = stmt.query_map(params![tag_id], |r| r.get::<_, i64>(0))?;
        let result: Vec<i64> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    // ===== Screenshots =====

    pub fn get_game_screenshots(&self, game_id: i64) -> Result<Vec<(i64, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, path FROM game_screenshots WHERE game_id=?1 ORDER BY sort_order, id",
        )?;
        let rows = stmt.query_map(params![game_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        let result: Vec<(i64, String)> = rows.filter_map(|r| r.ok()).collect();
        Ok(result)
    }

    pub fn add_game_screenshot(&self, game_id: i64, path: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM game_screenshots WHERE id=?1", params![screenshot_id])?;
        Ok(())
    }

    pub fn reorder_games(&self, game_ids: &[i64]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        for (i, id) in game_ids.iter().enumerate() {
            conn.execute(
                "UPDATE games SET sort_order = ?1 WHERE id = ?2",
                params![i as i64, id],
            )?;
        }
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
