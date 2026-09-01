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

// ==================== File Discovery ====================

/// 在目录中查找主可执行文件，返回 (exe_path, detected_name)
///
/// 策略（参考 Playnite/LaunchBox）：
/// 1. 递归搜索最多 3 层深度，仅跳过确定性运行库目录
/// 2. 排除已知的非主程序 exe（安装器/启动器/更新器/运行库等）
/// 3. 优先匹配与根目录同名的 exe
/// 4. 综合评分排序：PE版本信息匹配(+1000) > 深度浅(+300/层) > 文件大小
/// 5. detected_name 始终使用根目录名
pub fn find_main_exe(dir: &Path) -> (String, String) {
    let skip_exe_patterns = [
        "unins", "setup", "install", "config", "crack", "patch", "update",
        "launcher", "updater", "redist", "vcredist", "dxsetup", "dotnet",
        "oalinst", "crashpad", "uninstall", "register", "activat",
        "steam_api", "steamclient", "openvr", "vrmonitor",
        "d3d", "opengl", "wrapper", "loader", "inject",
    ];

    // 递归收集所有 exe（最多 3 层）
    let mut candidates: Vec<ExeCandidate> = Vec::new();
    collect_exes(dir, 0, 3, &skip_exe_patterns, &mut candidates);

    // detected_name 始终使用根目录名
    let detected_name = dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    if candidates.is_empty() {
        return (String::new(), detected_name);
    }

    let dir_name_lower = detected_name.to_lowercase();

    // 优先匹配与根目录同名的 exe
    if !dir_name_lower.is_empty() {
        if let Some(c) = candidates.iter().find(|c| {
            c.path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
                .contains(&dir_name_lower)
        }) {
            return (c.path.to_string_lossy().to_string(), detected_name);
        }
    }

    // 综合评分排序
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
        // 文件大小作为 tiebreaker（以 MB 为单位避免溢出）
        score += (c.size / 1_000_000) as i64;
        c.score = score;
    }

    candidates.sort_by_key(|c| std::cmp::Reverse(c.score));
    let best = &candidates[0];
    let exe_path = best.path.to_string_lossy().to_string();

    // 优先使用 PE ProductName 作为 detected_name（更有可能是游戏的正式名称）
    let final_name = if let Some(ref product) = best.product_name {
        let trimmed = product.trim();
        // 过滤掉太短、纯数字、或明显是通用名称的情况
        if trimmed.len() >= 3
            && !trimmed.chars().all(|c| c.is_ascii_digit() || c == '.' || c == ' ')
            && !trimmed.eq_ignore_ascii_case("test")
        {
            trimmed.to_string()
        } else {
            detected_name
        }
    } else {
        detected_name
    };

    (exe_path, final_name)
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
}
