//! 游戏数据访问：游戏 CRUD、分组、附加启动入口、截图、排序与统计

use rusqlite::{params, Result};
use serde::{Deserialize, Serialize};

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

/// 分组数据结构
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
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

use super::Database;

impl Database {
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
        rows.next().transpose()
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
}

#[cfg(test)]
mod tests {
    use crate::db::test_db::{cleanup_test_db, create_test_db};

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
