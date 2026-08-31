//! 设置与密码数据访问：应用设置读写、压缩包密码管理
//!
//! 密码使用 AES-256-GCM 加密存储，密钥通过系统密钥链管理

use rusqlite::{params, Result};
use serde::{Deserialize, Serialize};

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
    pub auto_backup: String,
    pub save_backup_dir: String,
    pub watch_dir: String,
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
            auto_backup: "true".to_string(),
            save_backup_dir: String::new(),
            watch_dir: String::new(),
        }
    }
}

use super::Database;

impl Database {
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
                "auto_backup" => s.auto_backup = v,
                "save_backup_dir" => s.save_backup_dir = v,
                "watch_dir" => s.watch_dir = v,
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

    /// 读取单个设置项，不存在时返回空字符串（用于内部状态如 last_auto_backup）
    pub fn get_setting(&self, key: &str) -> Result<String> {
        let conn = self.conn();
        let value = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |r| r.get::<_, String>(0),
            )
            .unwrap_or_default();
        Ok(value)
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
}

#[cfg(test)]
mod tests {
    use crate::db::test_db::{cleanup_test_db, create_test_db};

    #[test]
    fn test_password_encryption() {
        let db = create_test_db();
        db.add_password("test_password_123").unwrap();
        let passwords = db.get_passwords().unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0], "test_password_123");
        cleanup_test_db(&db);
    }
}
