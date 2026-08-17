/**
 * 统一 Tauri invoke 调用收口
 *
 * 所有前端 → Rust 后端的命令调用应经此包装：
 * 1. 错误归一化：Tauri invoke 失败时抛出的是裸字符串或未知类型，
 *    统一转换为带命令名的 InvokeError，便于定位与展示；
 * 2. 可关联的错误信息：每条错误都携带 command 字段，可选关联
 *    后台任务键（taskKey），失败时自动将对应 Task 标记为 error，
 *    让用户在任务中心看到失败原因。
 */

import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { failTask } from "../composables/useTaskCenter";

/** 归一化后的 invoke 错误 */
export class InvokeError extends Error {
  /** 触发失败的后端命令名 */
  readonly command: string;
  /** 原始错误对象（裸字符串 / Error / 其他） */
  readonly cause: unknown;

  constructor(command: string, message: string, cause: unknown) {
    super(message);
    this.name = "InvokeError";
    this.command = command;
    this.cause = cause;
  }
}

/**
 * 将任意 invoke 失败值归一化为可读消息
 */
export function normalizeInvokeError(command: string, err: unknown): InvokeError {
  let message: string;
  if (typeof err === "string") {
    message = err;
  } else if (err instanceof Error) {
    message = err.message || String(err);
  } else if (err == null) {
    message = "Unknown error";
  } else {
    try {
      message = JSON.stringify(err);
    } catch {
      message = String(err);
    }
  }
  return new InvokeError(command, `[${command}] ${message}`, err);
}

/**
 * 统一 invoke 包装
 *
 * @param cmd 后端命令名
 * @param args 命令参数
 * @param options.taskKey 可选：关联的后台任务键（如 "extract"、"scan-covers"），
 *   失败时自动将任务中心对应任务标记为 error
 */
export async function invoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
  options?: { taskKey?: string }
): Promise<T> {
  try {
    return await tauriInvoke<T>(cmd, args);
  } catch (err) {
    const normalized = normalizeInvokeError(cmd, err);
    console.error("[invoke] failed:", normalized.message, normalized.cause);
    if (options?.taskKey) {
      failTask(options.taskKey, normalized.message);
    }
    throw normalized;
  }
}
