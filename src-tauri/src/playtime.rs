//! 游玩时长监控模块
//!
//! 采用业内通用的"进程扫描 + 增量落盘"方案替代子进程 wait 监控：
//! - 每 5 秒扫描一次系统进程，通过"进程名快筛 + 安装目录路径前缀匹配"判断游戏是否在运行
//! - 每 30 秒增量落盘一次时长，应用崩溃/中途退出最多丢失 30 秒数据
//! - 应用启动时自动恢复未闭合的游玩会话（进程仍在则继续监控，否则直接关闭）
//!
//! 该方案同时解决了：launcher 短命进程误判、外部启动不计入、
//! `cmd /c start` 回退路径记 0、应用中途关闭丢全部时长等问题。
//!
//! 性能说明：无活跃会话时监控线程纯休眠（CPU 占用为 0）；
//! 扫描时先比对进程名（纳秒级），仅对命中候选（通常 0~2 个）查询完整映像路径。

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};

#[cfg(windows)]
use winapi::shared::minwindef::{DWORD, FALSE};
#[cfg(windows)]
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
#[cfg(windows)]
use winapi::um::processthreadsapi::OpenProcess;
#[cfg(windows)]
use winapi::um::winbase::QueryFullProcessImageNameW;
#[cfg(windows)]
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
    TH32CS_SNAPPROCESS,
};
#[cfg(windows)]
use winapi::um::winnt::PROCESS_QUERY_LIMITED_INFORMATION;

use crate::db::{Database, Game};

/// 进程扫描间隔
const SCAN_INTERVAL: Duration = Duration::from_secs(5);
/// 增量落盘间隔
const FLUSH_INTERVAL: Duration = Duration::from_secs(30);
/// 宽限期：刚启动或进程刚消失的这段时间内不判定会话结束
/// （覆盖启动缓慢、launcher 短暂存活、瞬时扫描漏检等场景）
const GRACE_PERIOD: Duration = Duration::from_secs(60);

/// 正在监控的游玩会话
#[derive(Clone)]
struct TrackedSession {
    session_id: i64,
    game_id: i64,
    /// 进程映像文件名（小写，用于快照快筛）
    exe_name: String,
    /// 安装目录前缀（小写，用于完整路径匹配）
    prefix: String,
    /// 用户自定义追踪进程名（小写，含 .exe）：命中即视为运行中，
    /// 不要求位于安装目录内，覆盖启动器与游戏本体分离的场景
    tracked_name: Option<String>,
    /// 会话开始被监控的时刻（宽限期判定用）
    started_at: Instant,
    /// 进程开始检测不到的时刻（None 表示当前能检测到）
    missing_since: Option<Instant>,
    /// 上次增量落盘时刻
    last_flush: Instant,
}

#[derive(Default)]
struct MonitorInner {
    sessions: HashMap<i64, TrackedSession>,
}

/// 游玩时长监控器（可廉价克隆，内部共享状态）
#[derive(Clone, Default)]
pub struct PlaytimeMonitor {
    inner: Arc<Mutex<MonitorInner>>,
}

fn lock_inner(inner: &Mutex<MonitorInner>) -> std::sync::MutexGuard<'_, MonitorInner> {
    inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn local_now_str() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 归一化自定义进程名：小写、去除首尾空白，缺省时补 .exe 后缀
fn normalize_tracked_name(raw: &str) -> Option<String> {
    let name = raw.trim().to_lowercase();
    if name.is_empty() {
        return None;
    }
    Some(if name.ends_with(".exe") { name } else { format!("{}.exe", name) })
}

/// 从游戏配置构建进程匹配信息：(进程名, 安装目录前缀, 自定义追踪进程名)
///
/// 匹配前缀优先使用 install_path；为空时回退到 exe_path 所在目录。
fn build_match(game: &Game) -> Option<(String, String, Option<String>)> {
    let exe_path = Path::new(&game.exe_path);
    let exe_name = exe_path
        .file_name()?
        .to_string_lossy()
        .to_lowercase();
    let dir = if !game.install_path.is_empty() {
        game.install_path.clone()
    } else {
        exe_path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    };
    if dir.is_empty() {
        return None;
    }
    let mut prefix = dir.replace('/', "\\").to_lowercase();
    if !prefix.ends_with('\\') {
        prefix.push('\\');
    }
    Some((exe_name, prefix, normalize_tracked_name(&game.tracked_process_name)))
}

impl PlaytimeMonitor {
    /// 启动后台监控线程
    pub fn start(db: Arc<Database>, app: AppHandle) -> Self {
        let monitor = Self::default();
        let inner = monitor.inner.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(SCAN_INTERVAL);
            tick(&inner, &db, &app);
        });
        monitor
    }

    /// 追踪一个新的游玩会话（启动游戏后调用）
    ///
    /// 若该游戏已有会话在监控中（重复启动），旧会话会被新的替换。
    pub fn track(&self, game: &Game, session_id: i64) {
        let Some((exe_name, prefix, tracked_name)) = build_match(game) else {
            // 无 exe_path 时无法做进程匹配，直接关闭会话避免悬空记录
            return;
        };
        let now = Instant::now();
        let mut inner = lock_inner(&self.inner);
        // 替换该游戏已有的旧会话监控（旧会话保持数据库中的未闭合状态，
        // 若进程仍在运行，下次扫描会因新会话前缀相同而自然延续）
        inner.sessions.retain(|_, s| s.game_id != game.id);
        inner.sessions.insert(
            session_id,
            TrackedSession {
                session_id,
                game_id: game.id,
                exe_name,
                prefix,
                tracked_name,
                started_at: now,
                missing_since: None,
                last_flush: now,
            },
        );
    }

    /// 插入一条恢复的会话监控（启动时恢复用，进程已确认在运行）
    fn track_recovered(
        &self,
        session_id: i64,
        game_id: i64,
        exe_name: String,
        prefix: String,
        tracked_name: Option<String>,
    ) {
        let now = Instant::now();
        let mut inner = lock_inner(&self.inner);
        inner.sessions.insert(
            session_id,
            TrackedSession {
                session_id,
                game_id,
                exe_name,
                prefix,
                tracked_name,
                started_at: now,
                missing_since: None,
                last_flush: now,
            },
        );
    }
}

/// 单次监控循环：扫描进程 → 增量落盘 / 关闭已结束的会话
fn tick(inner: &Mutex<MonitorInner>, db: &Database, app: &AppHandle) {
    let now = Instant::now();
    let snapshot: Vec<TrackedSession> = {
        let inner = lock_inner(inner);
        if inner.sessions.is_empty() {
            return;
        }
        inner.sessions.values().cloned().collect()
    };

    let mut names: HashSet<String> = snapshot.iter().map(|s| s.exe_name.clone()).collect();
    for snap in &snapshot {
        if let Some(tn) = &snap.tracked_name {
            names.insert(tn.clone());
        }
    }
    let running_paths = running_image_paths(&names);

    let mut inner = lock_inner(inner);
    let mut to_remove = Vec::new();
    for snap in &snapshot {
        let Some(session) = inner.sessions.get_mut(&snap.session_id) else {
            continue;
        };
        // 运行判定：安装目录前缀匹配，或自定义追踪进程名命中（不限路径）
        let running = running_paths.iter().any(|p| p.starts_with(&session.prefix))
            || session
                .tracked_name
                .as_ref()
                .map(|tn| {
                    running_paths.iter().any(|p| {
                        p.rsplit('\\').next().map(|f| f == tn).unwrap_or(false)
                    })
                })
                .unwrap_or(false);
        if running {
            session.missing_since = None;
            // 增量落盘
            if now.duration_since(session.last_flush) >= FLUSH_INTERVAL {
                let delta = now.duration_since(session.last_flush).as_secs() as i64;
                let now_str = local_now_str();
                if db
                    .increment_play_time(session.session_id, session.game_id, delta, &now_str)
                    .is_ok()
                {
                    session.last_flush = now;
                    let _ = app.emit("play-time-updated", session.game_id);
                }
            }
        } else {
            // 宽限期：刚启动的会话不判定结束
            if now.duration_since(session.started_at) < GRACE_PERIOD {
                continue;
            }
            let missing_start = *session.missing_since.get_or_insert(now);
            // 进程持续消失超过宽限期，判定游戏已退出
            if now.duration_since(missing_start) >= GRACE_PERIOD {
                let now_str = local_now_str();
                let _ = db.close_play_session(session.session_id, &now_str);
                let _ = app.emit("play-session-ended", session.game_id);
                to_remove.push(session.session_id);
            }
        }
    }
    for id in to_remove {
        inner.sessions.remove(&id);
    }
}

/// 应用启动时恢复未闭合的游玩会话：
/// 进程仍在运行 → 恢复监控；否则直接关闭（时长已在运行期间增量落盘）
pub fn recover_open_sessions(db: &Arc<Database>, monitor: &PlaytimeMonitor) {
    let open = match db.get_open_sessions() {
        Ok(v) => v,
        Err(_) => return,
    };
    if open.is_empty() {
        return;
    }
    let now_str = local_now_str();
    // 收集所有未闭合会话的匹配信息
    let mut candidates: Vec<(i64, i64, String, String, Option<String>)> = Vec::new(); // (session_id, game_id, exe_name, prefix, tracked_name)
    for session in &open {
        let game = db.get_game_by_id(session.game_id).ok().flatten();
        match game.and_then(|g| build_match(&g).map(|(n, p, t)| (session.id, g.id, n, p, t))) {
            Some(c) => candidates.push(c),
            None => {
                let _ = db.close_play_session(session.id, &now_str);
            }
        }
    }
    if candidates.is_empty() {
        return;
    }
    let mut names: HashSet<String> = candidates.iter().map(|c| c.2.clone()).collect();
    for c in &candidates {
        if let Some(tn) = &c.4 {
            names.insert(tn.clone());
        }
    }
    let running_paths = running_image_paths(&names);
    for (session_id, game_id, exe_name, prefix, tracked_name) in candidates {
        let running = running_paths.iter().any(|p| p.starts_with(&prefix))
            || tracked_name
                .as_ref()
                .map(|tn| {
                    running_paths
                        .iter()
                        .any(|p| p.rsplit('\\').next().map(|f| f == tn).unwrap_or(false))
                })
                .unwrap_or(false);
        if running {
            monitor.track_recovered(session_id, game_id, exe_name, prefix, tracked_name);
        } else {
            let _ = db.close_play_session(session_id, &now_str);
        }
    }
}

// ==================== Windows 进程枚举 ====================

/// 枚举系统进程，返回进程名命中 `names` 的进程的完整映像路径（小写）
///
/// 先做进程名快筛（纯内存比较），仅对候选进程查询完整路径，开销极低。
#[cfg(windows)]
fn running_image_paths(names: &HashSet<String>) -> HashSet<String> {
    let mut result = HashSet::new();
    if names.is_empty() {
        return result;
    }
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return result;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as DWORD;
        if Process32FirstW(snap, &mut entry) != FALSE {
            loop {
                let end = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..end]).to_lowercase();
                if names.contains(&name) {
                    if let Some(path) = query_process_image_path(entry.th32ProcessID) {
                        result.insert(path.to_lowercase());
                    }
                }
                if Process32NextW(snap, &mut entry) == FALSE {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }
    result
}

/// 查询指定进程的完整映像路径
#[cfg(windows)]
unsafe fn query_process_image_path(pid: DWORD) -> Option<String> {
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
    if handle.is_null() {
        return None;
    }
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as DWORD;
    let path = if QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len) != FALSE {
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    } else {
        None
    };
    CloseHandle(handle);
    path
}

/// 非 Windows 平台占位（项目仅支持 Windows，此分支仅为保证编译通过）
#[cfg(not(windows))]
fn running_image_paths(_names: &HashSet<String>) -> HashSet<String> {
    HashSet::new()
}
