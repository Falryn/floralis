//! 多源元数据匹配引擎
//!
//! 对输入的游戏名候选并行查询 Bangumi / VNDB / Steam，做标题归一化与相似度打分；
//! 无高置信命中时进行跨源"名称桥接"（Bangumi 中文名 → 日文原名 → VNDB，
//! 或 VNDB 罗马字 → Bangumi），最终返回按分数排序的候选列表。

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::helpers::build_http_agent;
use crate::models::MatchCandidate;

const HIGH_CONF: f64 = 0.85;
const MED_CONF: f64 = 0.55;

// ==================== 标题归一化 ====================

/// 判断是否为版本词元：v1.2 / ver2.0 / 1.05 等
///
/// 带 v/ver 前缀且含数字，或无前缀但同时含数字与小数点才算版本；
/// 纯数字（如 "Steins;Gate 0" 的 0）不视为版本。
fn is_version_token(t: &str) -> bool {
    let body = t
        .strip_prefix("ver")
        .or_else(|| t.strip_prefix('v'))
        .unwrap_or(t);
    if body.is_empty() {
        return false;
    }
    let has_digit = body.chars().any(|c| c.is_ascii_digit());
    let only_ver_chars = body.chars().all(|c| c.is_ascii_digit() || c == '.');
    let prefixed = t.starts_with('v');
    only_ver_chars && has_digit && (prefixed || body.contains('.'))
}

/// 常见版本/汉化/发行版后缀词元（匹配时忽略，避免干扰相似度）
const EDITION_TOKENS: &[&str] = &[
    "dlc", "demo", "trial", "beta", "update", "patch", "ost", "hd",
    "deluxe", "complete", "edition", "remaster", "remastered",
    "collection", "anthology", "bundle", "definitive", "goty",
    "完全版", "汉化版", "汉化", "中文版", "英文版", "日文版",
    "官方中文", "繁中版", "简中版", "数字版", "豪华版", "典藏版", "完整版",
];

/// 常见 DLC / 原声带 / 设定集线索词：这些条目与本体同名但不是本体，不能当匹配结果
const DLC_HINTS: &[&str] = &[
    "soundtrack", "artbook", "art book", "ost", "dlc", "prepack", "bundle", "epilogue",
    "原声", "设定集", "美术集", "音乐集", "插画集",
];

/// 是否为附属商品（Steam/Bangumi 搜索会把本体与 DLC 一起返回）
pub fn is_dlc_like(name: &str) -> bool {
    let lower = name.to_lowercase();
    DLC_HINTS.iter().any(|h| lower.contains(h))
}

/// 在 `hay` 中查找作为独立单词出现的 `word`（两侧不得是 ASCII 字母）
fn has_word(hay: &str, word: &str) -> bool {
    let b = hay.as_bytes();
    let w = word.as_bytes();
    if w.is_empty() || b.len() < w.len() {
        return false;
    }
    (0..=b.len() - w.len()).any(|i| {
        &b[i..i + w.len()] == w
            && (i == 0 || !b[i - 1].is_ascii_alphabetic())
            && (i + w.len() == b.len() || !b[i + w.len()].is_ascii_alphabetic())
    })
}

/// 是否为试玩版：与本体同名但不是本体（未发售作常常只先挂 Demo）
///
/// 必须按词边界判断——直接 `contains("demo")` 会误杀标题里带 Demon 的作品。
pub fn is_demo_like(name: &str) -> bool {
    let lower = name.to_lowercase();
    ["体験版", "试玩版", "体验版"].iter().any(|h| lower.contains(h)) || has_word(&lower, "demo")
}

/// 全角 ASCII 与表意空格折叠到半角，避免同一标题因输入宽度不同而匹配不上
fn fold_width(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{ff01}'..='\u{ff5e}' => char::from_u32(c as u32 - 0xfee0).unwrap_or(c),
            '\u{3000}' => ' ',
            _ => c,
        })
        .collect()
}

/// 标题归一化：全角折叠 → 清洗组名前缀/平台水印/版本尾巴 → 小写 → 去标点 → 去版本词元
///
/// 名称清洗与本地目录探测共用 `helpers::clean_display_name`，保证展示名与参与匹配的词一致。
/// 保留 CJK 与字母数字，其余字符替换为空格后折叠。
pub fn normalize_title(raw: &str) -> String {
    let s = crate::helpers::clean_display_name(&fold_width(raw.trim())).to_lowercase();
    // 版本词需在标点清洗前按原始词元判定（清洗后 "1.2.3" 会碎成 "1 2 3"）
    let prescreened: Vec<String> = s
        .split_whitespace()
        .map(|t| if is_version_token(t) { String::new() } else { t.to_string() })
        .collect();
    let cleaned: String = prescreened
        .join(" ")
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let tokens: Vec<&str> = cleaned
        .split_whitespace()
        .filter(|t| !EDITION_TOKENS.contains(t))
        .collect();
    tokens.join(" ")
}

fn levenshtein(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            cur.push((prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// 二元组 Jaccard（对无空格 CJK 文本比词级比较更稳健）
fn bigram_jaccard(a: &str, b: &str) -> f64 {
    let av: Vec<char> = a.chars().filter(|c| !c.is_whitespace()).collect();
    let bv: Vec<char> = b.chars().filter(|c| !c.is_whitespace()).collect();
    if av.is_empty() || bv.is_empty() {
        return 0.0;
    }
    if av.len() < 2 || bv.len() < 2 {
        return if av == bv { 1.0 } else { 0.0 };
    }
    let pairs = |v: &Vec<char>| -> Vec<(char, char)> { v.windows(2).map(|w| (w[0], w[1])).collect() };
    let ap = pairs(&av);
    let bp = pairs(&bv);
    let inter = ap
        .iter()
        .filter(|p| bp.contains(p))
        .count()
        .min(ap.len().min(bp.len()));
    let union = ap.len() + bp.len() - inter;
    if union == 0 { 0.0 } else { inter as f64 / union as f64 }
}

/// 词元集合 Jaccard：乱序 / 多一个限定词的标题（`Bow Down Eyes Up` vs `Bow Down & Eyes Up`）
/// 用编辑距离会被压低，词集重合能正确反映“同一作品”。
fn token_jaccard(a: &str, b: &str) -> f64 {
    let at: Vec<&str> = a.split_whitespace().collect();
    let bt: Vec<&str> = b.split_whitespace().collect();
    if at.len() < 2 || bt.len() < 2 {
        return 0.0;
    }
    let inter = at.iter().filter(|t| bt.contains(t)).count();
    let union = at.len() + bt.len() - inter;
    if union == 0 { 0.0 } else { inter as f64 / union as f64 }
}

/// 归一化标题间的相似度（0..1）
///
/// 分级：完全相等 → 1.0；包含关系 → 0.75 起步按长度比上浮；
/// 否则取编辑距离比率、二元组 Jaccard 与词元 Jaccard 的最大值。
pub fn similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    if a == b {
        return 1.0;
    }
    if a.contains(b) || b.contains(a) {
        let (lo, hi) = if a.chars().count() < b.chars().count() {
            (a, b)
        } else {
            (b, a)
        };
        let ratio = lo.chars().count() as f64 / hi.chars().count().max(1) as f64;
        return 0.75 + 0.15 * ratio;
    }
    let ac: Vec<char> = a.chars().filter(|c| !c.is_whitespace()).collect();
    let bc: Vec<char> = b.chars().filter(|c| !c.is_whitespace()).collect();
    let max_len = ac.len().max(bc.len());
    let lev_ratio = if max_len == 0 {
        0.0
    } else {
        1.0 - levenshtein(&ac, &bc) as f64 / max_len as f64
    };
    lev_ratio
        .max(bigram_jaccard(a, b))
        .max(token_jaccard(a, b))
        .max(char_jaccard(a, b))
        .max(0.0)
}

/// 单字集合 Jaccard：仅在两侧都含中日韩表意文字时启用
///
/// 简繁/日汉混排（`萨雅小姐的帮助` vs `佐雅小姐的帮助`、`女性用風俗` vs `女性用风俗`）
/// 会让编辑距离和二元组同时被字形差异击穿，而单字重合度仍能稳定表达「同一作品」。
/// 上限钳制在 [`CJK_SIMILARITY_CAP`] 以下，避免它单独撑起高置信自动采纳。
fn char_jaccard(a: &str, b: &str) -> f64 {
    let has_cjk = |s: &str| s.chars().any(|c| matches!(c, '\u{3400}'..='\u{9fff}'));
    if !has_cjk(a) || !has_cjk(b) {
        return 0.0;
    }
    let set = |s: &str| -> Vec<char> {
        let mut v: Vec<char> = Vec::new();
        for c in s.chars().filter(|c| !c.is_whitespace()) {
            if !v.contains(&c) {
                v.push(c);
            }
        }
        v
    };
    let av = set(a);
    let bv = set(b);
    let inter = av.iter().filter(|c| bv.contains(c)).count();
    let union = av.len() + bv.len() - inter;
    if union == 0 {
        return 0.0;
    }
    (inter as f64 / union as f64).min(CJK_SIMILARITY_CAP)
}

/// 单字重合度允许贡献的最高分（刻意低于 [`HIGH_CONF`]）
const CJK_SIMILARITY_CAP: f64 = 0.82;

fn confidence_of(score: f64) -> String {
    if score >= HIGH_CONF {
        "high".into()
    } else if score >= MED_CONF {
        "medium".into()
    } else {
        "low".into()
    }
}

/// 用多个候选名（如中文名+原名）对归一化查询取最高相似度
fn best_score(query_norm: &str, names: &[String]) -> f64 {
    names
        .iter()
        .filter(|n| !n.trim().is_empty())
        .map(|n| similarity(query_norm, &normalize_title(n)))
        .fold(0.0f64, f64::max)
}

/// 额外桥接词也能命中时视为交叉验证，取两者较大相似度
fn best_score2(a_norm: &str, b_norm: Option<&str>, names: &[String]) -> f64 {
    let base = best_score(a_norm, names);
    match b_norm {
        Some(extra) => base.max(best_score(extra, names)),
        None => base,
    }
}

/// 展示名挑选的同分窗口：差距小于此值视为“同一作品的不同写法”
const ALIAS_TIE_WINDOW: f64 = 0.2;

/// 从条目的多个别名（中文/原文/罗马字）中挑展示名
///
/// 直接取数据源主标题会踩到两类问题：VNDB 主标题可能是罗马字（库里中文/日文名才是别名），
/// 而用户检索用的就是自己熟悉的那个写法。策略：与任一查询词最相似者胜；
/// 得分差距在 [`ALIAS_TIE_WINDOW`] 内时优先含非 ASCII（中日韩）文字的写法，
/// 同语种则优先更完整（更长）的那个——`1room` 这类简写别名不如 `1room -家出少女-` 好认。
/// 结果先 trim 去尾空格。
fn pick_display_name(queries: &[&str], aliases: &[String]) -> Option<String> {
    let mut best: Option<(f64, bool, String)> = None;
    for alias in aliases {
        let name = alias.trim();
        if name.is_empty() {
            continue;
        }
        let norm = normalize_title(name);
        let score = queries
            .iter()
            .map(|q| similarity(q, &norm))
            .fold(0.0f64, f64::max);
        let has_cjk = !name.is_ascii();
        let take = match &best {
            None => true,
            Some((bs, bc, bn)) => {
                if (score - *bs).abs() < ALIAS_TIE_WINDOW {
                    (has_cjk && !*bc) || (has_cjk == *bc && name.chars().count() > bn.chars().count())
                } else {
                    score > *bs
                }
            }
        };
        if take {
            best = Some((score, has_cjk, name.to_string()));
        }
    }
    best.map(|(_, _, n)| n)
}

/// 候选信息完整度（同分排序用）：封面与简介齐备者靠前，提高自动采纳后的补全率
fn completeness(c: &MatchCandidate) -> u8 {
    let has_cover = c.cover_url.is_some() || (c.source == "steam" && c.app_id.is_some());
    u8::from(has_cover) * 2 + u8::from(c.summary.is_some())
}

/// 同名异源候选之间借用缺失的封面 / 简介
///
/// 典型场景：Bangumi 条目有中文名但无图，VNDB 同名作品有图有简介。
/// 不合并则自动采纳后只剩一个光秃秃的名字。
fn merge_duplicate_media(cands: &mut [MatchCandidate]) {
    for i in 0..cands.len() {
        if cands[i].cover_url.is_some() && cands[i].summary.is_some() {
            continue;
        }
        let key = normalize_title(&cands[i].name);
        let orig = normalize_title(&cands[i].original_name);
        let donor = cands
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .find(|(_, c)| {
                let ck = normalize_title(&c.name);
                ck == key || ck == orig || normalize_title(&c.original_name) == key
            })
            .map(|(_, c)| (c.cover_url.clone(), c.summary.clone()));
        if let Some((cover, summary)) = donor {
            if cands[i].cover_url.is_none() {
                cands[i].cover_url = cover;
            }
            if cands[i].summary.is_none() {
                cands[i].summary = summary;
            }
        }
    }
}

/// 简介裁剪：外部数据源的 description 可能极长，入库前按字符数截断
fn clamp_summary(text: Option<String>) -> Option<String> {
    const LIMIT: usize = 500;
    let t = text?.trim().to_string();
    if t.is_empty() {
        return None;
    }
    if t.chars().count() <= LIMIT {
        return Some(t);
    }
    let cut: String = t.chars().take(LIMIT).collect();
    Some(format!("{}…", cut.trim_end()))
}

// ==================== 数据源限流 ====================

/// Bangumi 未认证接口限频约 30 次/分钟；VNDB 明确限一秒一次，突发请求直接回 401/429。
/// 全局最小间隔令牌桶（进程内）避免批量匹配时打爆接口。
static BANGUMI_LAST: Mutex<Option<Instant>> = Mutex::new(None);
static VNDB_LAST: Mutex<Option<Instant>> = Mutex::new(None);
const BANGUMI_INTERVAL: Duration = Duration::from_millis(1200);
const VNDB_INTERVAL: Duration = Duration::from_millis(1200);
/// VNDB 被限频后的退避基数与总尝试次数
const VNDB_BACKOFF: Duration = Duration::from_millis(1500);
const VNDB_ATTEMPTS: u32 = 3;

fn throttle_slot(slot: &Mutex<Option<Instant>>, interval: Duration) {
    loop {
        let sleep_for = {
            let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
            match *guard {
                Some(last) => {
                    let elapsed = last.elapsed();
                    if elapsed >= interval {
                        *guard = Some(Instant::now());
                        None
                    } else {
                        Some(interval - elapsed)
                    }
                }
                None => {
                    *guard = Some(Instant::now());
                    None
                }
            }
        };
        match sleep_for {
            Some(d) => std::thread::sleep(d),
            None => return,
        }
    }
}

fn bangumi_throttle() {
    throttle_slot(&BANGUMI_LAST, BANGUMI_INTERVAL);
}

fn vndb_throttle() {
    throttle_slot(&VNDB_LAST, VNDB_INTERVAL);
}

// ==================== 各源内部搜索 ====================

/// VNDB 标题别名（`titles` 数组）：各语种官方登录名
#[derive(serde::Deserialize)]
struct VndbTitle {
    #[serde(default)]
    title: String,
    #[serde(default)]
    official: bool,
}

/// VNDB 匹配用搜索（取全部标题别名：中文登录名只存在于 titles 数组，
/// 不取则会出现「搜得到但打分归零」的漏匹配）
#[derive(serde::Deserialize)]
struct VndbMatchItem {
    #[serde(default)]
    title: String,
    #[serde(default)]
    alttitle: Option<String>,
    #[serde(default)]
    titles: Vec<VndbTitle>,
    #[serde(default)]
    image: Option<VndbImageRef>,
    #[serde(default)]
    description: Option<String>,
}

impl VndbMatchItem {
    /// 参与打分与展示的全部别名（去重）
    ///
    /// 只取官方标题：`official = false` 的条目多为转载者自加的简写/噪声罗马字
    /// （实测 `-1room-`、越南语罗马字这类会盖掉真正好认的完整名字）。
    fn aliases(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let push = |name: &str, bag: &mut Vec<String>| {
            // 数据库里的别名常带《》或【】包装，清洗后再比较
            let n = name.trim().trim_matches(|c| c == '《' || c == '》' || c == '【' || c == '】');
            let n = n.trim();
            if n.is_empty() || bag.iter().any(|e: &String| e == n) {
                return;
            }
            bag.push(n.to_string());
        };
        push(&self.title, &mut out);
        if let Some(a) = self.alttitle.as_deref() {
            push(a, &mut out);
        }
        for t in &self.titles {
            if t.official {
                push(&t.title, &mut out);
            }
        }
        out
    }
}

/// VNDB 站内引用标记（简介里的 `[char:…]`、`[release:v… "x"]` 等）
const VNDB_REF_TAGS: &[&str] = &["char", "vn", "release", "producer", "staff", "tag", "edit"];

/// 去掉 VNDB description 里的站内引用标记，保纯文本
fn vndb_description(raw: &Option<String>) -> Option<String> {
    let text = raw.as_ref().map(|t| t.trim().to_string())?;
    if text.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '[' {
            out.push(c);
            continue;
        }
        let mut span = String::new();
        let mut closed = false;
        for inner in chars.by_ref() {
            if inner == ']' {
                closed = true;
                break;
            }
            span.push(inner);
        }
        let head = span
            .split([':', ' '])
            .next()
            .unwrap_or_default();
        if !closed || !VNDB_REF_TAGS.contains(&head) {
            out.push('[');
            out.push_str(&span);
            if closed {
                out.push(']');
            }
        }
    }
    let cleaned = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

#[derive(serde::Deserialize)]
struct VndbImageRef {
    url: Option<String>,
}

#[derive(serde::Deserialize)]
struct VndbMatchResponse {
    #[serde(default)]
    results: Vec<VndbMatchItem>,
}

/// VNDB 检索
///
/// `search` 过滤器实为前缀/词元匹配（实测行为稳定，同一词多次调用结果集一致），
/// 对民间汉化中文名基本搜不到东西——这属于库里没数据，不是匹配算法缺陷。
/// 注意：`titles.title` 不能作为过滤字段（`/kana/vn` 会回 400 “Invalid '=' filter: Unknown field”），
/// 只能拉回 `titles` 数组在本地参与打分。
///
/// 被限频（401/429）时静默当成「库里没有」会让整批导入漏命中，按线性退避重试。
fn vndb_search(query: &str) -> Vec<VndbMatchItem> {
    let body = serde_json::json!({
        "filters": ["search", "=", query],
        "fields": "id,title,alttitle,titles.title,titles.official,image.url,description",
        "results": 6
    });
    let agent = build_http_agent();
    for attempt in 0..VNDB_ATTEMPTS {
        if attempt == 0 {
            vndb_throttle();
        } else {
            // 被限频时按线性退避重试：静默当成「库里没有」会让整批导入漏命中
            std::thread::sleep(VNDB_BACKOFF * attempt);
        }
        let resp = match agent
            .post("https://api.vndb.org/kana/vn")
            .set("Content-Type", "application/json")
            .set("User-Agent", "Floralis/0.1")
            .send_string(&body.to_string())
        {
            Ok(r) => r,
            Err(ureq::Error::Status(code, _)) if code == 401 || code == 429 => continue,
            Err(_) => return Vec::new(),
        };
        return resp
            .into_json::<VndbMatchResponse>()
            .map(|r| r.results)
            .unwrap_or_default();
    }
    Vec::new()
}

/// Bangumi 图片 URL 可能以 // 开头，补全协议
fn fix_image_url(url: Option<String>) -> Option<String> {
    url.map(|u| {
        if u.starts_with("//") {
            format!("https:{}", u)
        } else {
            u
        }
    })
}

fn bangumi_best_image(r: &crate::bangumi::BangumiResult) -> Option<String> {
    let imgs = r.images.as_ref()?;
    fix_image_url(
        imgs.large
            .clone()
            .or_else(|| imgs.common.clone())
            .or_else(|| imgs.grid.clone()),
    )
}

/// Bangumi 结果 → 打分候选（展示优先中文名；`extra_norm` 为桥接词，用于交叉验证）
fn bangumi_candidates(
    results: &[crate::bangumi::BangumiResult],
    q_norm: &str,
    extra_norm: Option<&str>,
) -> Vec<(f64, MatchCandidate)> {
    results
        .iter()
        .filter(|r| !is_dlc_like(&r.name) && !is_dlc_like(r.name_cn.as_deref().unwrap_or("")))
        .map(|r| {
            let names: Vec<String> = [r.name_cn.clone().unwrap_or_default(), r.name.clone()]
                .into_iter()
                .filter(|n| !n.trim().is_empty())
                .collect();
            let score = best_score2(q_norm, extra_norm, &names);
            let mut qs: Vec<&str> = vec![q_norm];
            if let Some(e) = extra_norm {
                qs.push(e);
            }
            let cand = MatchCandidate {
                source: "bangumi".into(),
                score: 0.0,
                confidence: String::new(),
                name: pick_display_name(&qs, &names).unwrap_or_else(|| r.name.trim().to_string()),
                original_name: r.name.clone(),
                cover_url: bangumi_best_image(r),
                app_id: None,
                summary: clamp_summary(r.summary.clone()),
            };
            (score, cand)
        })
        .collect()
}

/// VNDB 结果 → 打分候选（title + alttitle 参与打分）
fn vndb_candidates(results: &[VndbMatchItem], q_norm: &str, extra_norm: Option<&str>) -> Vec<(f64, MatchCandidate)> {
    results
        .iter()
        .filter(|r| !is_dlc_like(&r.title))
        .map(|r| {
            let names = r.aliases();
            let score = best_score2(q_norm, extra_norm, &names);
            let mut qs: Vec<&str> = vec![q_norm];
            if let Some(e) = extra_norm {
                qs.push(e);
            }
            let cand = MatchCandidate {
                source: "vndb".into(),
                score: 0.0,
                confidence: String::new(),
                name: pick_display_name(&qs, &names).unwrap_or_else(|| r.title.trim().to_string()),
                original_name: r.title.clone(),
                cover_url: r.image.as_ref().and_then(|i| i.url.clone()),
                app_id: None,
                summary: clamp_summary(vndb_description(&r.description)),
            };
            (score, cand)
        })
        .collect()
}

/// 参与 appdetails 补全的 Steam 候选上限（每个候选一次额外请求）
const STEAM_ENRICH_LIMIT: usize = 2;

/// Steam 双 locale 结果 → 打分候选，并对靠前命中补全中文名与简介
async fn steam_candidates(
    hits: &[crate::steam::SteamBilingualHit],
    q_norm: &str,
) -> Vec<(f64, MatchCandidate)> {
    let mut scored: Vec<(f64, crate::steam::SteamBilingualHit)> = hits
        .iter()
        .filter(|h| {
            ![h.name_cn.as_deref(), h.name_en.as_deref()]
                .into_iter()
                .flatten()
                .any(|n| is_dlc_like(n) || is_demo_like(n))
        })
        .map(|h| {
            let names: Vec<String> = [h.name_cn.clone().unwrap_or_default(), h.name_en.clone().unwrap_or_default()]
                .into_iter()
                .filter(|n| !n.trim().is_empty())
                .collect();
            (best_score(q_norm, &names), h.clone())
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut out = Vec::new();
    for (rank, (score, hit)) in scored.into_iter().enumerate() {
        let aliases: Vec<String> = [hit.name_cn.clone().unwrap_or_default(), hit.name_en.clone().unwrap_or_default()]
            .into_iter()
            .filter(|n| !n.trim().is_empty())
            .collect();
        let fallback_name = pick_display_name(&[q_norm], &aliases)
            .or_else(|| hit.name_cn.clone())
            .or_else(|| hit.name_en.clone())
            .unwrap_or_default();
        let (name, summary) = if rank < STEAM_ENRICH_LIMIT && score >= MED_CONF {
            let id = hit.id;
            match tauri::async_runtime::spawn_blocking(move || crate::steam::fetch_steam_detail(id))
                .await
                .unwrap_or(None)
            {
                Some(d) => (d.name, clamp_summary(Some(d.short_description))),
                None => (fallback_name.clone(), None),
            }
        } else {
            (fallback_name.clone(), None)
        };
        out.push((
            score,
            MatchCandidate {
                source: "steam".into(),
                score: 0.0,
                confidence: String::new(),
                name,
                original_name: hit.name_en.clone().unwrap_or_else(|| fallback_name.clone()),
                // Steam 封面按 app_id 走 CDN（download_steam_cover），无需直链
                cover_url: None,
                app_id: Some(hit.id),
                summary,
            },
        ));
    }
    out
}

// ==================== 主命令 ====================

/// 一次匹配最多尝试的检索词数（包含派生变体；每个都要打一轮三源请求，过多会拖慢批量导入）
const MAX_QUERIES: usize = 4;
/// 返回候选上限
const MAX_CANDIDATES: usize = 8;

/// 建议候选的得分下限：低于此值的多为跨语种噪声（如把 "GAME" 当成任意带 game 的标题），
/// 展示出来只会让用户在候选卡片里翻找，不如直接归入「未命中、请手动搜索」。
const MIN_SUGGESTION: f64 = 0.3;

/// 副标题分隔符：数据库里的登录名通常只有主标题，全串搜索反而查不到
const SUBTITLE_SEPS: [char; 8] = ['～', '〜', '—', '－', ':', '：', '!', '！'];

fn push_query(list: &mut Vec<String>, value: String) {
    let v = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if v.chars().count() < 2 || list.len() >= MAX_QUERIES {
        return;
    }
    if list.iter().any(|e| e.to_lowercase() == v.to_lowercase()) {
        return;
    }
    list.push(v);
}

/// 由名称候选派生实际检索词
///
/// 1. `kamiiro_no_otome` 这类下划线工程命名折叠成空格，否则任何库都搜不中；
/// 2. 长标题（`女性用風俗～裏オプ営業…～`）额外取主标题段再试一轮——
///    数据库登录名往往只到副标题分隔符为止。
fn expand_queries(raw: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for q in raw {
        let base = q.replace('_', " ");
        push_query(&mut out, base.clone());
        if base.chars().count() > 10 {
            if let Some((idx, _)) = base.char_indices().find(|(_, c)| SUBTITLE_SEPS.contains(c)) {
                let head = base[..idx].to_string();
                if head.chars().count() >= 3 {
                    push_query(&mut out, head);
                }
            }
        }
    }
    if out.is_empty() {
        out.extend(raw.iter().map(|r| r.trim().to_string()));
    }
    out
}

/// 多源匹配：对多个名称候选逐个查询三源，返回按得分排序的候选（最多 8 个）
///
/// 名称候选来自本地目录探测（引擎工程标题 / exe 产品名 / 目录名），按信息量排序；
/// 首个候选已拿到高置信命中时立即收手，避免把请求浪费在已经匹配上的项目上。
#[tauri::command]
pub async fn match_game_metadata(queries: Vec<String>) -> Result<Vec<MatchCandidate>, String> {
    let mut uniq: Vec<String> = Vec::new();
    for raw in queries {
        let q = raw.trim().to_string();
        if q.is_empty() {
            continue;
        }
        if uniq.iter().any(|e| e.to_lowercase() == q.to_lowercase()) {
            continue;
        }
        uniq.push(q);
        if uniq.len() >= MAX_QUERIES {
            break;
        }
    }
    if uniq.is_empty() {
        return Err("搜索关键词为空".into());
    }
    match_impl(uniq).await
}

/// 单个查询词的三源检索与打分
async fn collect_for_query(query: &str) -> Vec<(f64, MatchCandidate)> {
    let q_norm = normalize_title(query);
    let qb = query.to_string();
    let qv = query.to_string();
    let qs = query.to_string();
    let hb = tauri::async_runtime::spawn_blocking(move || {
        bangumi_throttle();
        crate::bangumi::search_bangumi(qb).unwrap_or_default()
    });
    let hv = tauri::async_runtime::spawn_blocking(move || vndb_search(&qv));
    let hs = tauri::async_runtime::spawn_blocking(move || crate::steam::search_steam_bilingual(&qs));

    let (bangumi_results, vndb_results, steam_hits) = (
        hb.await.unwrap_or_default(),
        hv.await.unwrap_or_default(),
        hs.await.unwrap_or_default(),
    );

    let mut out = bangumi_candidates(&bangumi_results, &q_norm, None);
    out.extend(vndb_candidates(&vndb_results, &q_norm, None));
    out.extend(steam_candidates(&steam_hits, &q_norm).await);
    out
}

async fn match_impl(queries: Vec<String>) -> Result<Vec<MatchCandidate>, String> {
    let queries = expand_queries(&queries);
    let primary = queries[0].clone();
    let q_norm = normalize_title(&primary);

    // 第一轮：按候选名逐个查询，命中高置信即停
    let mut candidates: Vec<(f64, MatchCandidate)> = Vec::new();
    let mut best_round1 = 0.0f64;
    for q in &queries {
        let round = collect_for_query(q).await;
        best_round1 = best_round1.max(round.iter().map(|(s, _)| *s).fold(0.0f64, f64::max));
        candidates.extend(round);
        if best_round1 >= HIGH_CONF {
            break;
        }
    }

    // 第二轮：跨源桥接（仅当第一轮无高置信命中）
    if best_round1 < HIGH_CONF {
        // Bangumi 最佳命中 → 取日文原名回查 VNDB
        let bangumi_bridge = candidates
            .iter()
            .filter(|(s, c)| c.source == "bangumi" && *s >= MED_CONF)
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, c)| c.original_name.clone());
        if let Some(orig) = bangumi_bridge {
            let orig_norm = normalize_title(&orig);
            let bridged = tauri::async_runtime::spawn_blocking(move || vndb_search(&orig))
                .await
                .unwrap_or_default();
            candidates.extend(vndb_candidates(&bridged, &q_norm, Some(&orig_norm)));
        }

        // VNDB 最佳命中 → 取标题回查 Bangumi（换取中文名与交叉验证）
        let vndb_bridge = candidates
            .iter()
            .filter(|(s, c)| c.source == "vndb" && *s >= MED_CONF)
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, c)| c.name.clone());
        if let Some(title) = vndb_bridge {
            let title_norm = normalize_title(&title);
            let bridged = tauri::async_runtime::spawn_blocking(move || {
                bangumi_throttle();
                crate::bangumi::search_bangumi(title).unwrap_or_default()
            })
            .await
            .unwrap_or_default();
            candidates.extend(bangumi_candidates(&bridged, &q_norm, Some(&title_norm)));
        }
    }

    // 同名词在多轮查询中会重复出现，按（数据源, 展示名, appid）保高分去重
    let mut deduped: Vec<(f64, MatchCandidate)> = Vec::new();
    for (score, cand) in candidates.into_iter() {
        if let Some(existing) = deduped.iter_mut().find(|(_, e)| {
            e.source == cand.source
                && e.name == cand.name
                && e.app_id == cand.app_id
        }) {
            if score > existing.0 {
                existing.0 = score;
            }
            continue;
        }
        deduped.push((score, cand));
    }

    deduped.retain(|(score, _)| *score >= MIN_SUGGESTION);
    deduped.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| completeness(&b.1).cmp(&completeness(&a.1)))
    });
    let mut out: Vec<MatchCandidate> = deduped
        .into_iter()
        .take(MAX_CANDIDATES)
        .map(|(score, mut c)| {
            // 四舍五入到千分位，避免前端浮点噪声
            c.score = (score * 1000.0).round() / 1000.0;
            c.confidence = confidence_of(score);
            c
        })
        .collect();
    // 补全率优先：同名异源的候选互借封面与简介
    merge_duplicate_media(&mut out);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_group_prefix() {
        assert_eq!(normalize_title("[Water Color] 桜色レコンキスタ"), "桜色レコンキスタ");
        assert_eq!(normalize_title("【社团】夏日狂想曲"), "夏日狂想曲");
    }

    /// 发布组/盗版包常见命名：前缀水印、平台括号、版本尾巴都应被洗掉
    #[test]
    fn normalize_cleans_pirate_pack_naming() {
        assert_eq!(normalize_title("r管理员的窥视"), "管理员的窥视");
        assert_eq!(normalize_title("神彩の乙女【PC／Android】"), "神彩の乙女");
        assert_eq!(normalize_title("High Speed Train Saimin Ver2.35"), "high speed train saimin");
        assert_eq!(normalize_title("さいみん!!"), "さいみん");
        assert_eq!(normalize_title("(工口猴子) 女性用風俗"), "女性用風俗");
        assert_eq!(normalize_title("MocaLoveRelive"), "mocaloverelive");
    }

    #[test]
    fn normalize_strips_version_and_edition_tokens() {
        assert_eq!(normalize_title("My Game v1.2.3"), "my game");
        assert_eq!(normalize_title("Title VER2.0"), "title");
        assert_eq!(normalize_title("游戏名 完全版"), "游戏名");
        assert_eq!(normalize_title("Game Deluxe Edition"), "game");
        assert_eq!(normalize_title("某游戏 官方中文 汉化版"), "某游戏");
    }

    #[test]
    fn normalize_keeps_meaningful_digits() {
        // 编号类数字不是版本号，应保留
        assert_eq!(normalize_title("Steins;Gate 0"), "steins gate 0");
    }

    #[test]
    fn normalize_strips_punctuation() {
        assert_eq!(normalize_title("Fate/stay night -REALTA NUA-"), "fate stay night realta nua");
    }

    #[test]
    fn normalize_folds_fullwidth() {
        // 全角拉丁/数字与半角写法应归一到同一路径
        assert_eq!(normalize_title("Ｄｅｌｉｖｅｒｙ　Ｈｏｔ"), "delivery hot");
        assert_eq!(normalize_title("Game．Ｖ１．２"), "game");
    }

    #[test]
    fn dlc_like_names_are_detected() {
        assert!(is_dlc_like("Peeping Dorm Manager Soundtrack"));
        assert!(is_dlc_like("傲慢的怪兽公主与名侦探使魔 设定集"));
        assert!(is_dlc_like("某游戏 ArtBook"));
        assert!(!is_dlc_like("Peeping Dorm Manager"));
        assert!(!is_dlc_like("怠惰的怪兽公主不想工作"));
    }

    #[test]
    fn demo_like_names_are_detected_on_word_boundary() {
        assert!(is_demo_like("Kaiju Princess Demo"));
        assert!(is_demo_like("PeepingDormManager-DEMO"));
        assert!(is_demo_like("某作品体験版"));
        assert!(is_demo_like("某作品 试玩版"));
        // 词边界：标题里带 Demon / 罗马字含 demo 的不能误杀
        assert!(!is_demo_like("Demon's Corner"));
        assert!(!is_demo_like("Demomistress"));
        assert!(!is_demo_like("Peeping Dorm Manager"));
    }

    #[test]
    fn similarity_exact_and_containment() {
        assert!((similarity("abc", "abc") - 1.0).abs() < 1e-9);
        let s = similarity("summer pockets", "summer pockets reflection blue");
        assert!(s > 0.75 && s < 0.95, "containment score out of range: {}", s);
        assert_eq!(similarity("", "abc"), 0.0);
    }

    #[test]
    fn similarity_fuzzy_and_cjk() {
        let s = similarity("clannad", "clannado");
        assert!(s > 0.8, "one-char typo should score high: {}", s);
        let cjk = similarity("樱之诗", "樱之诗在樱花之森上飞舞");
        assert!(cjk > 0.75, "CJK containment should score high: {}", cjk);
        let unrel = similarity("nekopara", "muv luv");
        assert!(unrel < 0.4, "unrelated titles should score low: {}", unrel);
    }

    #[test]
    fn similarity_ignores_word_order_and_separators() {
        // 词序不同 / 多一个连接符 → 词元覆盖度应给出高分
        let reordered = similarity("peeping manager dorm", "peeping dorm manager");
        assert!(reordered >= 0.95, "reordered tokens should stay high: {}", reordered);
        let separated = similarity("bow down eyes up", "bow down and eyes up");
        assert!(separated > 0.75, "extra connector should stay high: {}", separated);
        let partial = similarity("my wife", "wife is a cat");
        assert!(partial < 0.5, "weak overlap should stay low: {}", partial);
    }

    #[test]
    fn confidence_tiers() {
        assert_eq!(confidence_of(0.9), "high");
        assert_eq!(confidence_of(0.6), "medium");
        assert_eq!(confidence_of(0.2), "low");
    }

    #[test]
    fn clamp_summary_limits_length() {
        assert_eq!(clamp_summary(None), None);
        assert_eq!(clamp_summary(Some("  ".into())), None);
        let long: String = "字".repeat(600);
        let cut = clamp_summary(Some(long)).unwrap();
        assert_eq!(cut.chars().count(), 501, "应截到 500 字 + 省略号");
    }

    /// 展示名不能盲取数据源主标题：罗马字主标题下应回到用户熟悉的中文名
    #[test]
    fn pick_display_prefers_matching_and_cjk_alias() {
        let aliases = vec![
            "Wo Jiushi Yao Dang Chikan".to_string(),
            "我就是要当痴汉".to_string(),
        ];
        assert_eq!(
            pick_display_name(&["我就是要当痴汉"], &aliases).as_deref(),
            Some("我就是要当痴汉")
        );
        // 得分差距大时，与查询最匹配的别名胜过不相干别名
        assert_eq!(
            pick_display_name(&["some alias"], &[
                "Some Alias".to_string(),
                "某作品".to_string()
            ])
            .as_deref(),
            Some("Some Alias")
        );
        // 空别名不参与选择
        assert_eq!(
            pick_display_name(&["sketchy"], &["".to_string(), "SKETCHY".to_string()])
                .as_deref(),
            Some("SKETCHY")
        );
        // 尾部空白清洗
        assert_eq!(
            pick_display_name(&["sketchy massage"], &[
                "SKETCHY MASSAGE ".to_string(),
                "".to_string()
            ])
            .as_deref(),
            Some("SKETCHY MASSAGE")
        );
        // 同分窗口内：简写别名（-1room-）不该盖掉带副标题的完整写法
        let short_first = vec![
            "-1room-".to_string(),
            "1room -家出少女-".to_string(),
        ];
        let long_first = vec![
            "1room -家出少女-".to_string(),
            "-1room-".to_string(),
        ];
        assert_eq!(
            pick_display_name(&["1room"], &short_first).as_deref(),
            Some("1room -家出少女-")
        );
        assert_eq!(
            pick_display_name(&["1room"], &long_first).as_deref(),
            Some("1room -家出少女-")
        );
    }

    /// VNDB 别名需含官方中文登录名、排除非官方噪声别名（带《》包装的要清洗），
    /// 且简介中的站内引用标记要被清除
    #[test]
    fn vndb_aliases_and_description_markup_are_normalized() {
        let item = VndbMatchItem {
            title: "Sennou Appli de Takabishana Ojou-sama".into(),
            alttitle: Some("洗脳アプリで高飛車なお嬢様を好き放題".into()),
            titles: vec![
                VndbTitle {
                    title: "《用洗脑APP肆意玩弄狂妄大小姐》".into(),
                    official: true,
                },
                VndbTitle {
                    title: "random junk alias".into(),
                    official: false,
                },
            ],
            image: None,
            description: Some("前半[release:v12345 某版本]中间[char:a1b2C3d4]后半".into()),
        };
        let aliases = item.aliases();
        // 《》包装被清洗后仍以官方名入选
        assert!(aliases
            .iter()
            .any(|a| a == "用洗脑APP肆意玩弄狂妄大小姐"));
        // 非官方别名（转载者自加的简写/罗马字）不参与打分与展示
        assert!(!aliases.iter().any(|a| a == "random junk alias"));
        assert_eq!(
            vndb_description(&item.description).as_deref(),
            Some("前半中间后半")
        );
        // 非引用标记的方括号内容保留
        assert_eq!(
            vndb_description(&Some("[PC] 标题".into())).as_deref(),
            Some("[PC] 标题")
        );
    }

    /// 简繁/日汉字形差异下，编辑距离与二元组同时失效，靠单字重合兜住
    #[test]
    fn char_jaccard_bridges_simplified_traditional_variants() {
        // 同一作品的简体/繁体写法：字形不同的字占了一半
        let trad = similarity(
            &normalize_title("用洗脑APP肆意玩弄狂妄大小姐"),
            &normalize_title("用洗腦APP肆意的玩弄狂氣的大小姐"),
        );
        assert!(
            trad >= MED_CONF,
            "simplified vs traditional should reach medium tier: {}",
            trad
        );
        let alias = similarity(
            &normalize_title("萨雅小姐的帮助"),
            &normalize_title("佐雅小姐的帮助"),
        );
        assert!(alias >= MED_CONF, "one-char variant: {}", alias);
        // 纯拉丁标题不受影响（避免 nekopara vs muv luv 这类被单字重合抬高）
        assert!(similarity("nekopara", "muv luv") < 0.4);
        // 单字重合度自身被钳制，不会越过完全相等/包含关系之外的渠道给出高置信
        assert!(
            char_jaccard("用洗脑APP肆意玩弄狂妄大小姐", "用洗腦APP肆意的玩弄狂氣的大小姐") <= CJK_SIMILARITY_CAP
        );
        assert_eq!(char_jaccard("nekopara", "muv luv"), 0.0);
    }

    /// 检索词派生：下划线工程名折叠、长标题取主标题段
    #[test]
    fn expand_queries_derives_searchable_variants() {
        let got = expand_queries(&[
            "女性用風俗～裏オプ営業でイカせ放題、やり放題～".to_string(),
            "kamiiro_no_otome".to_string(),
        ]);
        assert_eq!(got[0], "女性用風俗～裏オプ営業でイカせ放題、やり放題～");
        assert_eq!(got[1], "女性用風俗");
        assert_eq!(got[2], "kamiiro no otome");
        // 不重复，且单字噪声词不入选
        let got2 = expand_queries(&["A B".to_string(), "A B".to_string(), "C".to_string()]);
        assert_eq!(got2, vec!["A B".to_string()]);
        // 不超上限
        let many: Vec<String> = (0..8).map(|i| format!("title {}", i)).collect();
        assert_eq!(expand_queries(&many).len(), MAX_QUERIES);
    }

    #[test]
    fn completeness_prefers_cover_and_summary() {
        let bare = MatchCandidate {
            source: "bangumi".into(),
            score: 0.0,
            confidence: String::new(),
            name: "X".into(),
            original_name: "X".into(),
            cover_url: None,
            app_id: None,
            summary: None,
        };
        let rich = MatchCandidate {
            cover_url: Some("https://img".into()),
            summary: Some("简介".into()),
            ..bare.clone()
        };
        let steam_like = MatchCandidate {
            source: "steam".into(),
            app_id: Some(1),
            ..bare.clone()
        };
        assert!(completeness(&rich) > completeness(&bare));
        // Steam 候选靠 app_id 走 CDN 拿图，等价于有封面
        assert!(completeness(&steam_like) > completeness(&bare));
    }

    /// 同名异源候选互借封面/简介，避免自动采纳后只剩一个光名字
    #[test]
    fn merge_duplicate_media_borrows_from_sibling_source() {
        let mut cands = vec![
            MatchCandidate {
                source: "bangumi".into(),
                score: 1.0,
                confidence: "high".into(),
                name: "神彩の乙女".into(),
                original_name: "kamiiro no otome".into(),
                cover_url: None,
                app_id: None,
                summary: None,
            },
            MatchCandidate {
                source: "vndb".into(),
                score: 0.9,
                confidence: "high".into(),
                name: "kamiiro no otome".into(),
                original_name: "kamiiro no otome".into(),
                cover_url: Some("https://cdn/v.jpg".into()),
                app_id: None,
                summary: Some("VNDB 简介".into()),
            },
        ];
        merge_duplicate_media(&mut cands);
        assert_eq!(cands[0].cover_url.as_deref(), Some("https://cdn/v.jpg"));
        assert_eq!(cands[0].summary.as_deref(), Some("VNDB 简介"));
    }

    /// 真实网络匹配验证（需设置 MATCH_TEST_QUERY 环境变量，否则跳过）
    /// 例：MATCH_TEST_QUERY="夏日狂想曲" cargo test matcher -- --nocapture
    /// 多候选：MATCH_TEST_QUERY="管理员的窥视,Peeping Dorm Manager"
    #[test]
    fn match_real_query() {
        let raw = std::env::var("MATCH_TEST_QUERY").unwrap_or_default();
        if raw.is_empty() {
            return;
        }
        let queries: Vec<String> = raw.split(',').map(|s| s.trim().to_string()).collect();
        let primary = queries[0].clone();
        let candidates = tauri::async_runtime::block_on(match_impl(queries)).expect("匹配失败");
        println!("query: {} → {} candidates", primary, candidates.len());
        for c in candidates.iter().take(5) {
            println!(
                "  [{}/{}] score={} name={} orig={} cover={} summary={}",
                c.source,
                c.confidence,
                c.score,
                c.name,
                c.original_name,
                c.cover_url.is_some(),
                c.summary.as_ref().map(|s| s.chars().count()).unwrap_or(0)
            );
        }
        assert!(!candidates.is_empty(), "未返回任何候选");
    }

    /// 真实游戏库端到端命中率验证（需设 SLG_TEST_ROOT，否则跳过）
    /// 例：SLG_TEST_ROOT="D:\Game\SLG" cargo test match_real_library_e2e -- --nocapture
    /// 目录探测 → 多源匹配全链路走生产代码，输出「高置信/需确认/未命中」分布。
    #[test]
    #[ignore = "需要真实游戏库与网络，手动运行"]
    fn match_real_library_e2e() {
        let root = std::env::var("SLG_TEST_ROOT").unwrap_or_default();
        if root.is_empty() {
            return;
        }
        let mut dirs: Vec<_> = std::fs::read_dir(&root)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();

        let (mut high, mut medium, mut none) = (0, 0, 0);
        let (mut with_cover, mut with_summary) = (0, 0);
        let mut report = String::new();
        for dir in &dirs {
            let found = crate::helpers::find_main_exe(dir);
            let queries: Vec<String> = if found.name_candidates.is_empty() {
                vec![found.detected_name.clone()]
            } else {
                found.name_candidates.clone()
            };
            let cands = tauri::async_runtime::block_on(match_impl(queries.clone()))
                .unwrap_or_default();
            let top = cands.first();
            let tier = match top.map(|c| c.confidence.as_str()) {
                Some("high") => "HIGH",
                Some(_) => "MED",
                None => "NONE",
            };
            match tier {
                "HIGH" => high += 1,
                "MED" => medium += 1,
                _ => none += 1,
            }
            let orig_suffix = match top {
                Some(t) if !t.original_name.trim().is_empty() && t.original_name != t.name => {
                    format!(" / {}", t.original_name)
                }
                _ => String::new(),
            };
            let has_cover = top
                .map(|c| c.cover_url.is_some() || (c.source == "steam" && c.app_id.is_some()))
                .unwrap_or(false);
            let summary_len = top
                .and_then(|c| c.summary.clone())
                .map(|s| s.chars().count())
                .unwrap_or(0);
            let src_mix: Vec<String> = cands
                .iter()
                .fold(std::collections::BTreeMap::<String, usize>::new(), |mut m, c| {
                    *m.entry(c.source.clone()).or_default() += 1;
                    m
                })
                .into_iter()
                .map(|(s, n)| format!("{}:{}", s, n))
                .collect();
            report.push_str(&format!(
                "[{}] {}\n   查询={:?}\n   命中={} ({}{}) score={} 候选{}个 [{}] 封面={} 简介={}字\n",
                tier,
                dir.file_name().unwrap_or_default().to_string_lossy(),
                queries,
                top.map(|c| c.name.clone()).unwrap_or_default(),
                top.map(|c| c.source.clone()).unwrap_or_default(),
                orig_suffix,
                top.map(|c| c.score).unwrap_or_default(),
                cands.len(),
                src_mix.join(" "),
                has_cover,
                summary_len,
            ));
            if top.is_some() {
                with_cover += i32::from(has_cover);
                with_summary += i32::from(summary_len > 0);
            }
        }
        report.push_str(&format!(
            "\n=== 总计 {} 款：高置信直采 {} / 待确认 {} / 未命中 {}；命中项中带封面 {}/{}、带简介 {}/{}\n",
            dirs.len(),
            high,
            medium,
            none,
            with_cover,
            high + medium,
            with_summary,
            high + medium
        ));
        // 控制台输出会被 PowerShell 按本地代码页重编码，中文报告落一份原始 UTF-8 文件
        print!("{}", report);
        let out = std::env::var("E2E_OUT").unwrap_or_default();
        if !out.is_empty() {
            std::fs::write(&out, report).ok();
        }
    }
}
