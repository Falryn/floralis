import { invoke } from "@tauri-apps/api/core";
import { addToast } from "../composables/useToast";

/**
 * Unified IPC invocation wrapper.
 * Centralizes error handling for all Tauri command calls.
 * On error, shows a toast notification and re-throws.
 */
export async function invokeWrapper<T>(
  command: string,
  args?: Record<string, unknown>,
  options?: { silent?: boolean; errorMessage?: string }
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    if (!options?.silent) {
      const msg = options?.errorMessage || `操作失败: ${command}`;
      const detail = typeof e === "string" ? e : String(e);
      addToast(`${msg}\n${detail}`, "error");
    }
    throw e;
  }
}
