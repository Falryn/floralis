//! 数据库模块 - 管理所有持久化数据
//!
//! 使用 SQLite 存储游戏、Mod、标签、游玩时长、设置与密码等数据。
//! 按领域拆分为子模块：games / mods / tags / playtime / settings，
//! 本文件仅保留连接管理、表结构初始化与迁移逻辑。

mod games;
mod mods;
mod playtime;
mod settings;
mod tags;

pub use games::*;
pub use mods::*;
pub use playtime::*;
pub use settings::*;
pub use tags::*;

use rusqlite::{Connection, Result};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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
const SCHEMA_VERSION: i64 = 5;

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

    /// 测试专用：创建内存数据库（不落盘，各测试天然隔离）
    #[cfg(test)]
    pub(crate) fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        let db = Database {
            conn: Mutex::new(conn),
            db_path: PathBuf::from(":memory:"),
        };
        db.init_tables()?;
        Ok(db)
    }

    pub fn get_db_path(&self) -> PathBuf {
        self.db_path.clone()
    }

    /// 将 WAL 日志刷回主数据库文件（备份前调用，保证文件拷贝完整）
    pub fn checkpoint_wal(&self) -> Result<()> {
        let conn = self.conn();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
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
            // v5：收藏功能
            (5, &[
                ("games", "is_favorite", "INTEGER NOT NULL DEFAULT 0"),
            ]),
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

    pub fn clear_all(&self) -> Result<()> {
        let conn = self.conn();
        conn.execute_batch(
            "DELETE FROM mod_tags; DELETE FROM mods; DELETE FROM game_tags; DELETE FROM tags; DELETE FROM play_sessions; DELETE FROM games; DELETE FROM groups; DELETE FROM passwords; DELETE FROM settings;",
        )?;
        Ok(())
    }
}

/// 测试专用：内存数据库，不落盘、无需清理
#[cfg(test)]
pub(crate) mod test_db {
    use super::Database;

    pub(crate) fn create_test_db() -> Database {
        Database::new_in_memory().unwrap()
    }
}

#[cfg(test)]
mod migration_tests {
    use super::{Database, SCHEMA_VERSION};
    use rusqlite::Connection;
    use std::path::{Path, PathBuf};

    fn legacy_db_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "floralis_migration_{}_{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    /// 构造 v0 旧库：games/mods 仅含迁移前的基础列，并已写入数据
    fn create_legacy_db(path: &Path) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "
            CREATE TABLE games (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                group_id INTEGER,
                install_path TEXT NOT NULL DEFAULT '',
                exe_path TEXT NOT NULL DEFAULT '',
                launch_args TEXT NOT NULL DEFAULT '',
                cover_path TEXT NOT NULL DEFAULT '',
                save_path TEXT NOT NULL DEFAULT '',
                notes TEXT NOT NULL DEFAULT '',
                created_at TEXT DEFAULT (datetime('now','localtime'))
            );
            CREATE TABLE mods (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                mod_path TEXT NOT NULL DEFAULT '',
                install_path TEXT NOT NULL DEFAULT '',
                game_id INTEGER,
                game_dir TEXT NOT NULL DEFAULT '',
                version TEXT NOT NULL DEFAULT '',
                author TEXT NOT NULL DEFAULT '',
                is_enabled INTEGER NOT NULL DEFAULT 1,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now','localtime')),
                updated_at TEXT DEFAULT (datetime('now','localtime'))
            );
            INSERT INTO games (name, install_path) VALUES ('Legacy Game', 'C:\\Games\\legacy');
            INSERT INTO mods (name, mod_path) VALUES ('Legacy Mod', 'C:\\mods\\legacy.pak');
            PRAGMA user_version = 0;
            ",
        )
        .unwrap();
    }

    fn remove_db_files(path: &Path) {
        let _ = std::fs::remove_file(path);
        for suffix in ["-wal", "-shm"] {
            let mut side = path.as_os_str().to_os_string();
            side.push(suffix);
            let _ = std::fs::remove_file(side);
        }
    }

    #[test]
    fn test_migration_from_legacy_v0_to_current() {
        let path = legacy_db_path();
        create_legacy_db(&path);

        let db = Database::new(&path).unwrap();

        // 旧数据保留，v1/v2/v5 迁移列以默认值补齐
        let game = db.get_game_by_id(1).unwrap().unwrap();
        assert_eq!(game.name, "Legacy Game");
        assert_eq!(game.install_path, "C:\\Games\\legacy");
        assert_eq!(game.status, "not_played");
        assert_eq!(game.rating, 0);
        assert_eq!(game.total_play_time, 0);
        assert!(!game.is_favorite);
        assert_eq!(game.tracked_process_name, "");

        let m = db.get_mod_by_id(1).unwrap().unwrap();
        assert_eq!(m.name, "Legacy Mod");
        assert_eq!(m.mod_path, "C:\\mods\\legacy.pak");
        assert_eq!(m.mod_type, "file");
        assert_eq!(m.category, "");

        // v3/v4 新建的表可直接使用
        db.add_game_screenshot(1, "/shot.png").unwrap();
        let profile_id = db.create_mod_profile(1, "default", &[1]).unwrap();
        assert!(db.get_profile_by_id(profile_id).unwrap().is_some());
        drop(db);

        // 版本号已推进到当前
        let conn = Connection::open(&path).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        drop(conn);

        // 重开幂等：不重复迁移、数据仍在
        let db2 = Database::new(&path).unwrap();
        assert!(db2.get_game_by_id(1).unwrap().is_some());
        assert!(db2.get_mod_by_id(1).unwrap().is_some());
        drop(db2);

        remove_db_files(&path);
    }
}
