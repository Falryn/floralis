//! 通用工具函数
//!
//! 文件搜索、封面复制、图片迁移等非命令辅助逻辑

use std::fs;
use std::path::{Path, PathBuf};

use crate::db::Database;

// ==================== PE Version Info (Windows FFI) ====================

#[cfg(target_os = "windows")]
mod pe_version {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    #[link(name = "version")]
    extern "system" {
        fn GetFileVersionInfoSizeW(lptstrFilename: *const u16, lpdwHandle: *mut u32) -> u32;
        fn GetFileVersionInfoW(
            lptstrFilename: *const u16,
            dwHandle: u32,
            dwLen: u32,
            lpData: *mut u8,
        ) -> i32;
        fn VerQueryValueW(
            pBlock: *const u8,
            lpSubBlock: *const u16,
            lplpBuffer: *mut *mut u8,
            puLen: *mut u32,
        ) -> i32;
    }

    /// 读取 exe 的 PE 版本信息中的 ProductName 或 FileDescription
    pub fn get_product_name(path: &Path) -> Option<String> {
        let wide: Vec<u16> = OsStr::new(path.as_os_str())
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut handle: u32 = 0;
        let size = unsafe { GetFileVersionInfoSizeW(wide.as_ptr(), &mut handle) };
        if size == 0 {
            return None;
        }

        let mut buf = vec![0u8; size as usize];
        if unsafe { GetFileVersionInfoW(wide.as_ptr(), handle, size, buf.as_mut_ptr()) } == 0 {
            return None;
        }

        // 尝试读取 ProductName，失败则读 FileDescription
        for key in &["\\StringFileInfo\\040904b0\\ProductName",
                     "\\StringFileInfo\\040904b0\\FileDescription",
                     "\\StringFileInfo\\080404b0\\ProductName",
                     "\\StringFileInfo\\080404b0\\FileDescription"] {
            let sub: Vec<u16> = OsStr::new(*key)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let mut ptr: *mut u8 = std::ptr::null_mut();
            let mut len: u32 = 0;
            if unsafe { VerQueryValueW(buf.as_ptr(), sub.as_ptr(), &mut ptr, &mut len) } != 0
                && !ptr.is_null()
                && len > 1
            {
                let wstr = unsafe {
                    std::slice::from_raw_parts(ptr as *const u16, (len - 1) as usize)
                };
                let s = String::from_utf16_lossy(wstr).trim().to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
        None
    }
}

#[cfg(not(target_os = "windows"))]
mod pe_version {
    use std::path::Path;
    pub fn get_product_name(_path: &Path) -> Option<String> {
        None
    }
}

// ==================== Name Detection ====================

/// 名称归一比对外键：小写、`_`/`-` 折叠为空格、压缩连续空格
fn name_key(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c == '_' || c == '-' { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// 引擎/运行库/占位类名称：即便来自 PE ProductName 或 exe 文件名也不能当游戏名
const JUNK_NAMES: &[&str] = &[
    "game", "game_en", "game zh-cn", "data", "bin", "test", "app", "main", "start", "run",
    "launch",
    "nwjs", "node-webkit", "electron", "python", "java", "unityplayer", "mono", "nwjc",
    "chromedriver", "notification_helper", "crashreporter", "setup", "install",
    "application", "executable", "unknown", "noname", "untitled", "新建文件夹", "示例",
    "bootstrappackagedgame", "unrealcefsubprocess", "gameinputsettings",
];

/// 引擎默认导出的产品名称前缀（Godot/Unreal/RPGRT 等会把它们写死在 PE 版本信息里）
const ENGINE_PREFIXES: &[&str] = &[
    "godot", "unreal engine", "unity", "windows phone app", "rgssplayer", "rvaptplayer",
];

/// 分发水印线索：FileDescription 常被盗版组改成推广语
const WATERMARK_HINTS: &[&str] = &[
    "分享", "免费", "首发", "汉化群", "版权所有", "资源站", "游戏站", "www.", "http", ".com", ".net",
];

/// 是否可作为展示名 / 元数据检索词
///
/// 拦住 `nwjs`（RPGMV 引擎名）、`Game`（占位）、`Godot Engine`，以及盗版组写在
/// FileDescription 里的推广水印，避免它们污染库名称与匹配查询词。
pub fn is_meaningful_name(raw: &str) -> bool {
    let t = raw.trim();
    let len = t.chars().count();
    // 中日韩标题两字即成词，拉丁名过短则基本是代号
    let min_len = if !t.is_ascii() { 2 } else { 3 };
    if len < min_len || len > 64 {
        return false;
    }
    let lower = t.to_lowercase();
    // PE 版本信息里常见 NUL 填充与乱码尾巴（如 `game\0\0<\u{12}`），含控制字符即视为脏数据
    if t.chars().any(|c| c.is_control()) {
        return false;
    }
    // Ren'Py 等引擎导出的 `Game_zh-CN` / `Game-EN` 与黑名单里的写法不同分隔符，归一后再比
    let key = name_key(&lower);
    if JUNK_NAMES.iter().any(|j| name_key(j) == key) {
        return false;
    }
    if ENGINE_PREFIXES.iter().any(|p| lower.starts_with(p)) {
        return false;
    }
    if WATERMARK_HINTS.iter().any(|h| lower.contains(h)) {
        return false;
    }
    // 纯符号或纯数字版本号
    if t.chars().all(|c| !c.is_alphanumeric())
        || t.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '_'))
    {
        return false;
    }
    true
}

/// 括号对（全/半角）：同人包命名里常见 `(组名) 标题`、`标题【PC/Android】`
const BRACKET_PAIRS: [(char, char); 4] = [('(', ')'), ('（', '）'), ('[', ']'), ('【', '】')];

/// 括号内出现即认为是水印的词（平台/汉化/分级等，不是标题本体）
const BRACKET_NOISE_TOKENS: &[&str] = &[
    "pc", "android", "ios", "switch", "win", "windows", "mac", "linux", "ver", "v", "r18",
    "18禁", "全年龄", "汉化", "中文", "云风", "dlsite", "精翻", "修复", "补丁", "种子", "冷番",
];

/// 去掉标题开头的 (组名) / [组名] / 【社团】 前缀（可连续多个）
fn strip_leading_bracket(s: &str) -> String {
    let mut out = s.trim().to_string();
    loop {
        let mut changed = false;
        for (open, close) in BRACKET_PAIRS {
            if !out.starts_with(open) {
                continue;
            }
            if let Some(pos) = out.find(close) {
                let rest = out[pos + close.len_utf8()..].trim_start().to_string();
                if !rest.is_empty() {
                    out = rest;
                    changed = true;
                    break;
                }
            }
        }
        if !changed {
            return out;
        }
    }
}

/// 括号内容是否全由水印词/数字构成
fn is_noise_bracket_content(content: &str) -> bool {
    let kept: String = content
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let tokens: Vec<&str> = kept.split_whitespace().collect();
    !tokens.is_empty()
        && tokens.iter().all(|t| {
            BRACKET_NOISE_TOKENS.contains(t)
                || t.chars().all(|c| c.is_ascii_digit())
        })
}

/// 去掉标题结尾的水印括号段（仅当内容全为噪声词），不会吃掉整个标题
fn strip_trailing_bracket_noise(s: &str) -> String {
    let original = s.trim().to_string();
    let mut out = original.clone();
    loop {
        let trimmed = out.trim_end().to_string();
        let Some(close) = trimmed.chars().last() else { return out };
        let Some((open, close_ch)) = BRACKET_PAIRS
            .into_iter()
            .find(|(_, c)| *c == close) else { return out };
        let Some(open_pos) = trimmed.rfind(open) else { return out };
        let content = &trimmed[open_pos + open.len_utf8()..trimmed.len() - close_ch.len_utf8()];
        if !is_noise_bracket_content(content) {
            return out;
        }
        let head = trimmed[..open_pos].trim_end().to_string();
        if head.is_empty() {
            return original;
        }
        out = head;
    }
}

/// 去掉紧贴中文名前的单个拉丁字母前缀（发布组编号习惯：`r管理员的窥视`）
fn strip_latin_group_prefix(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() >= 2 && chars[0].is_ascii_alphabetic() && !chars[1].is_ascii() {
        return chars[1..].iter().collect();
    }
    s.to_string()
}

/// 去掉紧贴标题尾部的版本号（`…ver0.77B` / `…V1.23`）
///
/// 按字符（而非字节）定位标记，避免 CJK 前缀使 `to_lowercase` 的字节偏移与原串错位。
fn strip_version_tail(s: &str) -> String {
    let src = s.trim();
    let chars: Vec<char> = src.chars().collect();
    let markers: [&[char]; 2] = [&['v', 'e', 'r'][..], &['v'][..]];
    for marker in markers {
        for i in (0..chars.len()).rev() {
            if i + marker.len() > chars.len() {
                continue;
            }
            if !marker
                .iter()
                .enumerate()
                .all(|(k, c)| chars[i + k].eq_ignore_ascii_case(c))
            {
                continue;
            }
            let head: String = chars[..i].iter().collect();
            let head = head.trim_end().to_string();
            let tail: String = chars[i + marker.len()..].iter().collect();
            let version_like = head.chars().count() >= 2
                && tail.chars().count() <= 10
                && tail.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                && tail.chars().all(|c| c.is_ascii_alphanumeric() || c == '.');
            if version_like {
                return head;
            }
        }
    }
    src.to_string()
}

/// 展示名与检索词的统一清洗：去组名前缀 / 平台水印括号段 / 单字母编号前缀 / 版本尾巴
pub fn clean_display_name(raw: &str) -> String {
    let s = strip_leading_bracket(raw);
    let s = strip_trailing_bracket_noise(&s);
    let s = strip_latin_group_prefix(&s);
    strip_version_tail(&s)
}

/// RPG Maker MV/MZ：从工程配置 `data/System.json` 读取 gameTitle
///
/// 这两代引擎的 exe 通常叫 Game.exe、PE 产品名固定为 nwjs，真实标题只存在于工程配置里；
/// 读出来能直接救回「目录名是乱码代号」的那批作品。
fn rpgmv_title(exe_dir: &Path) -> Option<String> {
    let mut probe_dirs: Vec<PathBuf> = vec![exe_dir.to_path_buf()];
    if let Some(parent) = exe_dir.parent() {
        probe_dirs.push(parent.to_path_buf());
    }
    for dir in probe_dirs {
        const REL_PATHS: [&[&str]; 2] = [
            &["www", "data", "System.json"],
            &["data", "System.json"],
        ];
        for rel in REL_PATHS.iter() {
            let mut file = dir.clone();
            for part in rel.iter() {
                file = file.join(part);
            }
            let Ok(text) = fs::read_to_string(&file) else {
                continue;
            };
            let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
            let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
                continue;
            };
            if let Some(title) = value.get("gameTitle").and_then(|v| v.as_str()) {
                let title = clean_display_name(title);
                if is_meaningful_name(&title) {
                    return Some(title);
                }
            }
        }
    }
    None
}

/// 目录探测结果
pub struct ExeDiscovery {
    /// 主可执行文件绝对路径（未找到时为空串）
    pub exe_path: String,
    /// 建议展示名 = 信息量最高的名称候选，无候选时回退目录名
    pub detected_name: String,
    /// 存档路径探测用的提示名（沿用 PE 产品名优先的老语义，避免改动既有行为）
    pub save_hint: String,
    /// 元数据匹配用的名称候选（有序，最多 4 个）
    pub name_candidates: Vec<String>,
}

/// 名称候选上限
const MAX_NAME_CANDIDATES: usize = 4;

/// 收集名称候选：过滤垃圾名并按信息量分级
fn push_name(ranked: &mut Vec<(u8, String)>, prio: u8, name: Option<String>) {
    if let Some(n) = name {
        let n = n.trim().to_string();
        if is_meaningful_name(&n) {
            ranked.push((prio, n));
        }
    }
}

/// 是否为发布组/工口资源站常见的乱码代号目录名（纯大写拉丁+数字且不含元音）
///
/// 如 `29SDRQ` `JCSNMNQ` `NXZSFS` `ML202`；这类名字去任何数据库都查不到，
/// 不应排在真正的作品名（exe 名）之前。
fn is_code_like_name(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() || !t.chars().all(|c| c.is_ascii_alphanumeric()) {
        return false;
    }
    let lower = t.to_lowercase();
    // 至少一个字母才谈得上元音；全数字（如年份、卷号）不当代号
    if !lower.chars().any(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    !lower.chars().any(|c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u'))
}

/// 是否为程序化标识符命名（小写拉丁 + 下划线，如 `speed_hypnosis_train`）
///
/// 这类名字是开发者留在 PE 产品名里的工程标识符，作为检索词还行，作为库展示名
/// 不如发布组起的中文目录名，因此降级到紧凑 exe 名同一档。
fn is_tech_identifier(s: &str) -> bool {
    let t = s.trim();
    t.contains('_')
        && t
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// 在目录中查找主可执行文件，并给出展示名与元数据匹配用的名称候选
///
/// 策略（参考 Playnite/LaunchBox）：
/// 1. 递归搜索最多 3 层深度，仅跳过确定性运行库目录
/// 2. 排除已知的非主程序 exe（安装器/启动器/更新器/运行库等）
/// 3. 优先匹配与根目录同名的 exe
/// 4. 综合评分排序：PE版本信息匹配(+1000) > 深度浅(+300/层) > 文件大小
/// 5. 名称候选按信息量排序：引擎工程标题 > 带空格/CJK 的 exe 名 > PE 产品名
///    > 目录名 > 紧凑 exe 名，供多源匹配逐个尝试
pub fn find_main_exe(dir: &Path) -> ExeDiscovery {
    let skip_exe_patterns = [
        "unins", "setup", "install", "config", "crack", "patch", "update",
        "launcher", "updater", "redist", "vcredist", "dxsetup", "dotnet",
        "oalinst", "crashpad", "crashhandler", "uninstall", "register", "activat",
        "steam_api", "steamclient", "openvr", "vrmonitor", "nwjc",
        "d3d", "opengl", "wrapper", "loader", "inject",
    ];

    // 递归收集所有 exe（最多 3 层）
    let mut candidates: Vec<ExeCandidate> = Vec::new();
    collect_exes(dir, 0, 3, &skip_exe_patterns, &mut candidates);

    let dir_name = dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    if candidates.is_empty() {
        let mut ranked: Vec<(u8, String)> = Vec::new();
        push_name(&mut ranked, 3, Some(dir_name.clone()));
        let name_candidates = finalize_names(ranked, &dir_name);
        return ExeDiscovery {
            exe_path: String::new(),
            detected_name: name_candidates[0].clone(),
            save_hint: dir_name.clone(),
            name_candidates,
        };
    }

    let dir_name_lower = dir_name.to_lowercase();

    // 优先匹配与根目录同名的 exe；否则按综合评分取最优
    let by_dir_name = candidates.iter().position(|c| {
        !dir_name_lower.is_empty()
            && c.path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
                .contains(&dir_name_lower)
    });
    let best_idx = by_dir_name.unwrap_or_else(|| {
        for c in &mut candidates {
            let mut score: i64 = 0;
            // PE 版本信息匹配：ProductName/FileDescription 包含目录名
            if let Some(ref product) = c.product_name {
                if product.to_lowercase().contains(&dir_name_lower) && !dir_name_lower.is_empty() {
                    score += 1000;
                }
            }
            // 深度越浅分越高（depth=0 → +300, depth=1 → +200, depth=2 → +100）
            score += (3 - c.depth.min(3)) as i64 * 100;
            // 文件名带空格或 CJK → 多为作品本体而非辅助程序，优先于体积更大的运行库壳
            let stem = c
                .path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if stem.contains(' ') || !stem.is_ascii() {
                score += 50;
            }
            // 同目录下 Unity 项目的 `xxx_Data` 子目录里的 exe 不为本体
            if c.path.to_string_lossy().to_lowercase().contains("_data/")
                || c.path.to_string_lossy().to_lowercase().contains("_data\\")
            {
                score -= 150;
            }
            // 文件大小作为 tiebreaker（以 MB 为单位避免溢出）
            score += (c.size / 1_000_000) as i64;
            c.score = score;
        }
        candidates.sort_by_key(|c| std::cmp::Reverse(c.score));
        0
    });

    let best = &candidates[best_idx];
    let exe_path = best.path.to_string_lossy().to_string();
    let exe_dir = best.path.parent().unwrap_or(dir);
    let exe_stem = best
        .path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // 存档提示名：沿用旧的「PE 产品名优先」语义
    let save_hint = match best.product_name.as_deref() {
        Some(p) => {
            let trimmed = p.trim();
            if trimmed.chars().count() >= 3
                && !trimmed.chars().all(|c| c.is_ascii_digit() || c == '.' || c == ' ')
                && !trimmed.eq_ignore_ascii_case("test")
            {
                trimmed.to_string()
            } else {
                dir_name.clone()
            }
        }
        None => dir_name.clone(),
    };

    let mut ranked: Vec<(u8, String)> = Vec::new();
    // 引擎工程配置里的官方标题最可信
    push_name(&mut ranked, 0, rpgmv_title(exe_dir));
    // exe 文件名带空格或含 CJK → 通常就是作品正式名称
    if exe_stem.split_whitespace().count() > 1 || !exe_stem.is_ascii() {
        push_name(&mut ranked, 1, Some(exe_stem.clone()));
    }
    // PE 产品名（过滤引擎名与推广水印后）；工程标识符式命名降级，不挤掉可读的中文目录名
    let pe_prio = match best.product_name.as_deref() {
        Some(p) if is_tech_identifier(p) => 4,
        _ => 2,
    };
    push_name(&mut ranked, pe_prio, best.product_name.clone());
    // 目录名（用户/发布组命名，常为中文名；先洗掉组前缀与平台水印）
    // 乱码代号目录（29SDRQ/ML202）排到 exe 名之后，否则会挤掉真正可用的检索词
    let dir_prio = if is_code_like_name(&dir_name) { 5 } else { 3 };
    push_name(&mut ranked, dir_prio, Some(clean_display_name(&dir_name)));
    // 兜底：紧凑命名的 exe 名（KaijuPrincess / chikanif 这类）
    push_name(&mut ranked, 4, Some(exe_stem));

    let name_candidates = finalize_names(ranked, &dir_name);
    ExeDiscovery {
        detected_name: name_candidates[0].clone(),
        exe_path,
        save_hint,
        name_candidates,
    }
}

/// 按优先级排序、按大小写不敏感去重，并保证结果非空
fn finalize_names(ranked: Vec<(u8, String)>, fallback: &str) -> Vec<String> {
    let mut ranked = ranked;
    ranked.sort_by_key(|(prio, _)| *prio);
    let mut out: Vec<String> = Vec::new();
    for (_, name) in ranked {
        if out
            .iter()
            .any(|e: &String| e.to_lowercase() == name.to_lowercase())
        {
            continue;
        }
        out.push(name);
        if out.len() >= MAX_NAME_CANDIDATES {
            break;
        }
    }
    if out.is_empty() {
        out.push(if fallback.trim().is_empty() {
            "未知游戏".to_string()
        } else {
            fallback.trim().to_string()
        });
    }
    out
}

struct ExeCandidate {
    path: PathBuf,
    size: u64,
    depth: usize,
    product_name: Option<String>,
    score: i64,
}

/// 递归收集 exe 文件
fn collect_exes(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    skip_patterns: &[&str],
    out: &mut Vec<ExeCandidate>,
) {
    if depth > max_depth {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let folder_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            // 只跳过确定性的运行库/安装器目录（不硬编码业务目录名）
            if matches!(folder_name.as_str(),
                "redist" | "__installer" | "commonredist" | "directx" | "vcredist" | "_commonredist"
            ) || folder_name.starts_with('.') {
                continue;
            }
            collect_exes(&path, depth + 1, max_depth, skip_patterns, out);
        } else if path
            .extension()
            .map(|ext| ext.eq_ignore_ascii_case("exe"))
            .unwrap_or(false)
        {
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
            if skip_patterns.iter().any(|s| name.contains(s)) {
                continue;
            }
            let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let product_name = pe_version::get_product_name(&path);
            out.push(ExeCandidate {
                path,
                size,
                depth,
                product_name,
                score: 0,
            });
        }
    }
}

/// 在目录中查找封面图片
pub fn find_cover_image(dir: &Path) -> String {
    let priority_names = [
        "cover.jpg",
        "cover.png",
        "cover.webp",
        "folder.jpg",
        "folder.png",
        "thumb.jpg",
        "thumb.png",
        "icon.jpg",
        "icon.png",
    ];

    let mut search_dirs = vec![dir.to_path_buf()];
    if let Ok(entries) = fs::read_dir(dir) {
        let subdirs: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.path())
            .collect();
        if subdirs.len() == 1 {
            search_dirs.push(subdirs[0].clone());
        }
    }

    // Check priority names first
    for name in &priority_names {
        for sd in &search_dirs {
            let path = sd.join(name);
            if path.exists() {
                return path.to_string_lossy().to_string();
            }
        }
    }

    // Fallback: find first image > 50KB
    let img_exts = ["jpg", "jpeg", "png", "webp", "bmp"];
    for sd in &search_dirs {
        if let Ok(entries) = fs::read_dir(sd) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if img_exts.iter().any(|e| ext.eq_ignore_ascii_case(e)) {
                        if let Ok(meta) = fs::metadata(&path) {
                            if meta.len() > 50_000 {
                                return path.to_string_lossy().to_string();
                            }
                        }
                    }
                }
            }
        }
    }

    String::new()
}

/// 查找游戏存档目录
///
/// 策略：
/// - 用户目录（APPDATA/LOCALAPPDATA/Documents）：宽泛匹配游戏名 + save 模式
/// - 安装目录：仅匹配 save/sav/savedata/saves（不匹配 data，避免误识别资源目录）
pub fn find_save_directory(game_name: &str, install_path: &str) -> String {
    // 用户目录中使用的模式（宽泛）
    let user_save_patterns = ["save", "sav", "savedata", "saves", "userdata", "data"];
    // 安装目录中使用的模式（严格，排除 data 避免误匹配资源文件夹）
    let install_save_patterns = ["save", "sav", "savedata", "saves"];

    let mut base_dirs = Vec::new();
    if let Ok(v) = std::env::var("APPDATA") {
        base_dirs.push(PathBuf::from(v));
    }
    if let Ok(v) = std::env::var("LOCALAPPDATA") {
        base_dirs.push(PathBuf::from(v));
    }
    if let Ok(v) = std::env::var("USERPROFILE") {
        base_dirs.push(PathBuf::from(&v).join("Documents"));
        base_dirs.push(PathBuf::from(&v).join("Saved Games"));
    }

    // 1. 在用户目录中查找：优先匹配游戏名
    for base in &base_dirs {
        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                let dir_name = entry.file_name().to_string_lossy().to_lowercase();
                if dir_name.contains(&game_name.to_lowercase()) && !game_name.is_empty() {
                    let path = entry.path();
                    if path.is_dir() {
                        return path.to_string_lossy().to_string();
                    }
                }
            }
        }
    }

    // 2. 在用户目录中查找：匹配 save 模式 + 内含存档文件
    for base in &base_dirs {
        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                let dir_name = entry.file_name().to_string_lossy().to_lowercase();
                if user_save_patterns.iter().any(|p| dir_name.contains(p)) {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Ok(sub) = fs::read_dir(&path) {
                            for s in sub.flatten() {
                                let ext = s
                                    .path()
                                    .extension()
                                    .map(|e| e.to_string_lossy().to_lowercase())
                                    .unwrap_or_default();
                                if ["sav", "dat", "bin", "json", "db"]
                                    .iter()
                                    .any(|e| ext == *e)
                                {
                                    return path.to_string_lossy().to_string();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. 在安装目录中查找（仅严格模式，不含 "data"）
    if let Ok(entries) = fs::read_dir(install_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase();
                if install_save_patterns.iter().any(|p| name.contains(p)) {
                    return path.to_string_lossy().to_string();
                }
            }
        }
    }

    String::new()
}

// ==================== Cover / Image Utilities ====================

/// 判断封面路径是否在应用数据目录内
pub fn is_internal_cover(path: &str, app_data_dir: &Path) -> bool {
    if path.is_empty() {
        return false;
    }
    let covers_dir = app_data_dir.join("covers");
    Path::new(path).starts_with(&covers_dir)
}

/// 复制封面到应用数据目录的 covers 子目录
pub fn copy_cover_to_internal(source: &str, game_id: i64, app_data_dir: &Path) -> Result<String, String> {
    use std::io::Write;
    if source.is_empty() || !Path::new(source).exists() {
        return Err("源文件不存在".into());
    }
    let covers_dir = app_data_dir.join("covers");
    fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;
    let ext = Path::new(source)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_lowercase();
    let filename = format!("cover_{}.{}", game_id, ext);
    let dest = covers_dir.join(&filename);
    if Path::new(source).canonicalize().ok() == dest.canonicalize().ok() {
        return Ok(dest.to_string_lossy().to_string());
    }
    let mut src_file = fs::File::open(source).map_err(|e| e.to_string())?;
    let mut dst_file = fs::File::create(&dest).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut src_file, &mut buf).map_err(|e| e.to_string())?;
    dst_file.write_all(&buf).map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().to_string())
}

/// 通用文件复制到应用数据子目录
///
/// 合并了 copy_cover_to_storage 和 copy_mod_cover_to_storage 的逻辑
/// - `subdir`: 目标子目录名（如 "covers"、"mod_covers"）
/// - `prefix`: 文件名前缀（如 "cover"、"mod_cover"）
/// - `id`: 可选 ID，有则用 `{prefix}_{id}.{ext}`，无则用时间戳
pub fn copy_file_to_appdata(
    source: &str,
    subdir: &str,
    prefix: &str,
    id: Option<i64>,
    app_data_dir: &Path,
) -> Result<String, String> {
    use std::io::Write;
    if source.is_empty() || !Path::new(source).exists() {
        return Err("源文件不存在".into());
    }
    let dir = app_data_dir.join(subdir);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let ext = Path::new(source)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_lowercase();
    let filename = if let Some(id) = id {
        format!("{}_{}.{}", prefix, id, ext)
    } else {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        format!("{}_{}.{}", prefix, timestamp, ext)
    };
    let dest = dir.join(&filename);

    let mut src_file = fs::File::open(source).map_err(|e| e.to_string())?;
    let mut dst_file = fs::File::create(&dest).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut src_file, &mut buf).map_err(|e| e.to_string())?;
    dst_file.write_all(&buf).map_err(|e| e.to_string())?;

    Ok(dest.to_string_lossy().to_string())
}

/// 将自定义图片复制到应用数据目录（使用时间戳文件名避免缓存）
pub fn copy_custom_image_to_internal(source: &str, key: &str, app_data_dir: &Path) -> Result<String, String> {
    let img_dir = app_data_dir.join("custom_images");
    fs::create_dir_all(&img_dir).map_err(|e| e.to_string())?;

    // 清理同 key 的旧文件
    if let Ok(entries) = fs::read_dir(&img_dir) {
        for entry in entries.flatten() {
            let stem = entry.path().file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
            if stem == *key || stem.starts_with(&format!("{}_", key)) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    let ext = Path::new(source)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_lowercase();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let dest = img_dir.join(format!("{}_{}.{}", key, ts, ext));
    fs::copy(source, &dest).map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().to_string())
}

/// 启动迁移：将旧版外部路径的自定义图片复制到应用数据目录
pub fn migrate_custom_images(db: &Database, app_data_dir: &Path) {
    let settings = match db.get_settings() {
        Ok(s) => s,
        Err(_) => return,
    };
    let pairs = [
        ("custom_banner", settings.custom_banner.clone()),
        ("custom_sidebar_bg", settings.custom_sidebar_bg.clone()),
        ("custom_empty_illustration", settings.custom_empty_illustration.clone()),
    ];
    for (key, value) in pairs {
        if value.is_empty() {
            continue;
        }
        let p = Path::new(&value);
        if p.starts_with(app_data_dir) {
            continue;
        }
        if !p.exists() {
            continue;
        }
        if let Ok(internal) = copy_custom_image_to_internal(&value, key, app_data_dir) {
            let _ = db.save_setting(key, &internal);
        }
    }
}

// ==================== Version ====================

/// 比较版本号，返回 true 表示 latest > current
pub fn compare_versions(current: &str, latest: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let c = parse(current);
    let l = parse(latest);
    for i in 0..l.len().max(c.len()) {
        let cv = c.get(i).copied().unwrap_or(0);
        let lv = l.get(i).copied().unwrap_or(0);
        if lv > cv {
            return true;
        }
        if lv < cv {
            return false;
        }
    }
    false
}

// ==================== HTTP Proxy ====================

/// 获取系统代理配置
///
/// 优先级：环境变量 > Windows 注册表系统代理
pub fn get_system_proxy() -> Option<ureq::Proxy> {
    // 1. 尝试环境变量
    for var in &["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"] {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() {
                if let Ok(proxy) = ureq::Proxy::new(&val) {
                    return Some(proxy);
                }
            }
        }
    }

    // 2. 读取 Windows 注册表系统代理
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        let hklm = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hklm.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Internet Settings") {
            let enabled: u32 = key.get_value("ProxyEnable").unwrap_or(0);
            if enabled == 1 {
                if let Ok(server) = key.get_value::<String, _>("ProxyServer") {
                    let proxy_url = if server.starts_with("http") {
                        server
                    } else {
                        format!("http://{}", server)
                    };
                    if let Ok(proxy) = ureq::Proxy::new(&proxy_url) {
                        return Some(proxy);
                    }
                }
            }
        }
    }

    None
}

/// 创建带代理支持和超时的 ureq Agent
pub fn build_http_agent() -> ureq::Agent {
    let mut builder = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout_read(std::time::Duration::from_secs(15));
    if let Some(proxy) = get_system_proxy() {
        builder = builder.proxy(proxy);
    }
    builder.build()
}

/// 将 ureq 请求错误转为面向用户的中文提示
pub fn friendly_http_error(source: &str, e: &ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, _) => {
            format!("{} 服务器返回异常状态（HTTP {}），请稍后重试", source, code)
        }
        ureq::Error::Transport(t) => {
            let msg = t.to_string().to_lowercase();
            if msg.contains("timed out") || msg.contains("timeout") || msg.contains("deadline") {
                format!("{} 请求超时，请检查网络连接后重试", source)
            } else if msg.contains("dns") {
                format!("无法解析 {} 的域名，请检查网络连接", source)
            } else if msg.contains("connection refused") || msg.contains("connection reset") || msg.contains("connection aborted") {
                format!("无法连接 {}，服务可能暂时不可用", source)
            } else if msg.contains("proxy") {
                format!("代理连接失败，请检查系统代理设置（{}）", source)
            } else {
                format!("{} 网络请求失败：{}", source, t)
            }
        }
    }
}

// ==================== Directory Utilities ====================

/// 递归复制目录：子目录递归创建，同名文件直接覆盖
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !src.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("源目录不存在: {}", src.display()),
        ));
    }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target)?;
        }
        // 符号链接等其他类型跳过
    }
    Ok(())
}

/// 统计目录内文件数量与总字节数（递归，不含目录本身）
pub fn dir_stats(path: &Path) -> (u64, u64) {
    let mut count = 0u64;
    let mut size = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_dir() {
                stack.push(entry.path());
            } else if ft.is_file() {
                count += 1;
                size += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    (count, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_dir(name: &str) -> PathBuf {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("floralis_test_{}_{}", name, ts));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn copy_dir_recursive_copies_nested_files_and_overwrites() {
        let src = unique_temp_dir("copy_src");
        let dst = unique_temp_dir("copy_dst");

        fs::create_dir_all(src.join("sub/deep")).unwrap();
        fs::write(src.join("a.txt"), b"hello").unwrap();
        fs::write(src.join("sub/b.sav"), b"save-data").unwrap();
        fs::write(src.join("sub/deep/c.dat"), b"deep").unwrap();

        // 预置一个将被覆盖的同名文件
        fs::create_dir_all(dst.join("sub")).unwrap();
        fs::write(dst.join("sub/b.sav"), b"old").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();

        assert_eq!(fs::read(dst.join("a.txt")).unwrap(), b"hello");
        assert_eq!(fs::read(dst.join("sub/b.sav")).unwrap(), b"save-data");
        assert_eq!(fs::read(dst.join("sub/deep/c.dat")).unwrap(), b"deep");

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dst);
    }

    #[test]
    fn copy_dir_recursive_errors_on_missing_src() {
        let dst = unique_temp_dir("copy_missing_dst");
        let missing = dst.join("no_such_dir");
        assert!(copy_dir_recursive(&missing, &dst.join("out")).is_err());
        let _ = fs::remove_dir_all(&dst);
    }

    #[test]
    fn dir_stats_counts_files_and_bytes() {
        let dir = unique_temp_dir("stats");
        fs::create_dir_all(dir.join("nested")).unwrap();
        fs::write(dir.join("x.txt"), b"12345").unwrap();
        fs::write(dir.join("nested/y.txt"), b"678").unwrap();

        let (count, size) = dir_stats(&dir);
        assert_eq!(count, 2);
        assert_eq!(size, 8);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn junk_names_are_rejected() {
        // 引擎名 / 占位名 / 纯版本号 / 盗版组推广水印均不可作为游戏名
        for junk in [
            "Game", "nwjs", "Godot Engine", "UnityPlayer", "Application", "1.0.0", "!!!",
            "新建文件夹", "2024 汉化群 免费分享", "www.example.com", "x",
            // Ren'Py 默认产出的启动器名，分隔符变体也得拦住
            "Game_zh-CN", "Game-EN", "game en", "Data", "Launch",
        ] {
            assert!(!is_meaningful_name(junk), "should reject: {}", junk);
        }
        for ok in [
            "家出少女", "Peeping Dorm Manager", "さいみん!!", "Kaiju Princess", "女性用風俗",
            "神彩の乙女", "Highway Hypnosis",
        ] {
            assert!(is_meaningful_name(ok), "should accept: {}", ok);
        }
    }

    #[test]
    fn tech_identifier_names_are_recognized() {
        // 工程标识符：降级为展示名，但仍可当检索词
        assert!(is_tech_identifier("speed_hypnosis_train"));
        assert!(is_tech_identifier("kamiiro_no_otome"));
        // 人可读命名不降级
        assert!(!is_tech_identifier("KaijuPrincess"));
        assert!(!is_tech_identifier("Peeping Dorm Manager"));
        assert!(!is_tech_identifier("高速列车上的催眠"));
    }

    #[test]
    fn finalize_names_sorts_by_priority_and_dedups() {
        let ranked = vec![
            (3u8, "r管理员的窥视".to_string()),
            (1, "Peeping Dorm Manager".to_string()),
            (3, "R管理员的窥视".to_string()),
            (0, "管理员的窥视".to_string()),
        ];
        let out = finalize_names(ranked, "fallback");
        assert_eq!(out, vec!["管理员的窥视", "Peeping Dorm Manager", "r管理员的窥视"]);
    }

    #[test]
    fn finalize_names_falls_back_when_no_candidate() {
        assert_eq!(finalize_names(Vec::new(), "某目录"), vec!["某目录"]);
        assert_eq!(finalize_names(Vec::new(), "  "), vec!["未知游戏"]);
    }

    #[test]
    fn clean_display_name_strips_pirate_noise() {
        // 发布组单字母编号前缀 / 组名括号 / 平台水印括号段 / 版本尾巴
        assert_eq!(clean_display_name("r管理员的窥视"), "管理员的窥视");
        assert_eq!(clean_display_name("(工口猴子) 女性用風俗"), "女性用風俗");
        assert_eq!(clean_display_name("神彩の乙女【PC／Android】"), "神彩の乙女");
        assert_eq!(clean_display_name("怠惰的怪兽公主不想工作ver0.77B"), "怠惰的怪兽公主不想工作");
        assert_eq!(clean_display_name("High Speed Train Saimin V1.23"), "High Speed Train Saimin");
        // 不应误伤含 v/ver 的正文名与纯代号
        assert_eq!(clean_display_name("MocaLoveRelive"), "MocaLoveRelive");
        assert_eq!(clean_display_name("29SDRQ"), "29SDRQ");
        assert_eq!(clean_display_name("Kaiju Princess 2"), "Kaiju Princess 2");
        // 全括号标题不被清空
        assert_eq!(clean_display_name("【PC】"), "【PC】");
    }

    #[test]
    fn rpgmv_title_reads_game_title_from_www_data() {
        let dir = unique_temp_dir("rpgmv");
        let data = dir.join("www").join("data");
        fs::create_dir_all(&data).unwrap();
        // 带 BOM（RPG 编辑器写出的 System.json 常见）且含真实标题
        fs::write(data.join("System.json"), "\u{feff}{\"gameTitle\":\"家出少女\"}").unwrap();
        assert_eq!(rpgmv_title(&dir).as_deref(), Some("家出少女"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rpgmv_title_ignores_placeholder_title() {
        let dir = unique_temp_dir("rpgmv_junk");
        let data = dir.join("data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("System.json"), "{\"gameTitle\":\"Game\"}").unwrap();
        assert_eq!(rpgmv_title(&dir), None);
        let _ = fs::remove_dir_all(&dir);
    }

    /// 对真实游戏库根目录跑一遍名称探测（需设 SLG_TEST_ROOT，否则跳过）
    /// 例：SLG_TEST_ROOT="D:\Game\SLG" cargo test helpers::tests::probe -- --nocapture
    #[test]
    fn probe_real_library_names() {
        let root = std::env::var("SLG_TEST_ROOT").unwrap_or_default();
        if root.is_empty() {
            return;
        }
        let root = PathBuf::from(&root);
        let mut dirs: Vec<PathBuf> = fs::read_dir(&root)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        for dir in dirs {
            let found = find_main_exe(&dir);
            println!(
                "DIR={}\n   name={}\n   cand={:?}\n   exe={}",
                dir.file_name().unwrap_or_default().to_string_lossy(),
                found.detected_name,
                found.name_candidates,
                found.exe_path
            );
        }
    }
}
