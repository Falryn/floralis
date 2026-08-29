import { onMounted, onUnmounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useGameStore } from "../stores/gameStore";

/** 快捷键行为回调：由调用方注入各自的 UI 状态处理 */
export interface ShortcutHandlers {
  /** Ctrl+F：聚焦搜索框 */
  focusSearch: () => void;
  /** Ctrl+,：打开设置 */
  openSettings: () => void;
  /** Escape：按优先级关闭右键菜单 / 退出选择模式 / 关闭详情 */
  escape: () => void;
  /** Delete：对选中游戏发起删除确认 */
  deleteSelected: () => void;
}

/**
 * 全局键盘快捷键：
 * Ctrl+Shift+I 切换开发者工具、Ctrl+F 聚焦搜索、Ctrl+, 打开设置；
 * 非输入态下 Escape / Delete / Enter / Space 作用于当前选中游戏。
 */
export function useShortcuts(handlers: ShortcutHandlers) {
  const store = useGameStore();
  const appWindow = getCurrentWindow();

  function handleKeydown(e: KeyboardEvent) {
    // Ctrl+Shift+I → toggle dev tools
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "I") {
      e.preventDefault();
      appWindow.toggleDevTools();
      return;
    }
    // Ctrl+F → focus search
    if ((e.ctrlKey || e.metaKey) && e.key === "f") {
      e.preventDefault();
      handlers.focusSearch();
      return;
    }
    // Ctrl+, → open settings
    if ((e.ctrlKey || e.metaKey) && e.key === ",") {
      e.preventDefault();
      handlers.openSettings();
      return;
    }
    // Ignore when input is focused
    const tag = (e.target as HTMLElement).tagName;
    if (["INPUT", "TEXTAREA", "SELECT"].includes(tag)) return;

    if (e.key === "Escape") {
      handlers.escape();
    }
    if (e.key === "Delete" && store.selectedGameId !== null) {
      handlers.deleteSelected();
    }
    if (e.key === "Enter" && store.selectedGameId !== null) {
      store.launchGame(store.selectedGameId);
    }
    if (e.key === " " && store.selectedGameId !== null) {
      e.preventDefault();
      store.launchGame(store.selectedGameId);
    }
  }

  onMounted(() => document.addEventListener("keydown", handleKeydown));
  onUnmounted(() => document.removeEventListener("keydown", handleKeydown));
}
