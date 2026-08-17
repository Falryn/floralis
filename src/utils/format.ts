/**
 * 通用格式化工具函数
 * 
 * 统一时间、日期、文本高亮等格式化逻辑，避免各组件重复定义
 */

import { invoke } from "./invoke";

type TFunction = (key: string, params?: Record<string, unknown>) => string;

/**
 * 在系统文件管理器中打开本地目录/文件
 * 
 * 通过后端命令直接调起 explorer.exe，不受 shell:open 的 URL scope 限制
 */
export function openInExplorer(path: string): void {
  if (!path) return;
  // 统一包装内部已归一化并记录错误日志
  invoke("open_in_explorer", { path }).catch(() => {});
}

/**
 * 格式化游玩时长
 * @param seconds 秒数
 * @param t i18n 翻译函数
 * @param prefix i18n key 前缀，如 'game'、'stats'、'calendar'
 * @param short 是否使用短格式（如 "2h30m" 而非 "2小时30分钟"）
 */
export function formatPlayTime(
  seconds: number,
  t: TFunction,
  prefix = "game",
  short = false
): string {
  if (seconds < 60) return t(`${prefix}.seconds`, { n: seconds });
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 0) {
    return short
      ? t(`${prefix}.hoursMinutesShort`, { h: hours, m: minutes })
      : t(`${prefix}.hoursMinutes`, { h: hours, m: minutes });
  }
  return short
    ? t(`${prefix}.minutesShort`, { m: minutes })
    : t(`${prefix}.minutesOnly`, { m: minutes });
}

/**
 * 格式化日期字符串
 * @param dateStr ISO 日期字符串
 * @param t i18n 翻译函数（用于 "从未" 等文案）
 * @param showTime 是否显示时间部分
 */
export function formatDate(
  dateStr: string | null,
  t: TFunction,
  showTime = false
): string {
  if (!dateStr) return t("game.never");
  return showTime ? dateStr.slice(0, 16).replace("T", " ") : dateStr.slice(0, 10);
}

/**
 * HTML 转义
 */
export function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/**
 * 高亮文本中的关键词（返回含 <mark> 标签的 HTML）
 * @param text 原始文本
 * @param keyword 搜索关键词
 */
export function highlightText(text: string, keyword: string): string {
  const kw = keyword.trim();
  if (!kw) return escapeHtml(text);
  const escaped = escapeHtml(text);
  const escapedKw = escapeHtml(kw);
  const regex = new RegExp(
    `(${escapedKw.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")})`,
    "gi"
  );
  return escaped.replace(
    regex,
    '<mark class="bg-yellow-200/70 text-inherit rounded-sm px-0.5">$1</mark>'
  );
}
