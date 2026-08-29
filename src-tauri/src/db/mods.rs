//! Mod 数据访问：Mod CRUD、启用状态、排序、配置组合（Profile）与游戏关联

use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

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

use super::Database;

impl Database {
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
}
