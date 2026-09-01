//! 标签数据访问：标签 CRUD、使用情况统计，以及游戏/Mod 与标签的关联

use rusqlite::{params, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

use super::Database;

impl Database {
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
    use crate::db::test_db::create_test_db;

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
    }
}
