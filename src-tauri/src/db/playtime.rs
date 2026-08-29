//! 游玩时长数据访问：会话管理、增量落盘、日历统计与手动修正

use rusqlite::{params, Result};
use serde::{Deserialize, Serialize};

/// 游戏会话记录
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlaySession {
    pub id: i64,
    pub game_id: i64,
    pub start_time: String,
    pub end_time: Option<String>,
    pub duration_seconds: i64,
}

use super::Database;

impl Database {
    /// 手动修正游戏总游玩时长（秒），不改动历史会话记录
    pub fn set_game_play_time(&self, id: i64, seconds: i64) -> Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE games SET total_play_time = ?1 WHERE id = ?2",
            params![seconds.max(0), id],
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
}
