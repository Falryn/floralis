import { onMounted, onUnmounted, ref } from "vue";

const IMAGE_EXTS = ["jpg", "jpeg", "png", "webp", "gif", "bmp"];

function isImageFile(path: string): boolean {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return IMAGE_EXTS.includes(ext);
}

export interface FileDropHandlers {
  /** 拖入非图片文件（压缩包等）→ 打开导入对话框 */
  onFiles: (paths: string[]) => void;
  /** 拖入单张图片且命中游戏卡片/详情页 → 设为封面 */
  onCoverDrop: (gameId: number, imagePath: string) => void;
  /** 拖入单张图片但未命中任何游戏 → 提示 */
  onCoverMiss: () => void;
}

/**
 * 监听操作系统文件管理器拖入事件（Tauri webview API）：
 * - 单张图片命中 [data-game-id] 元素 → 设为封面
 * - 其他文件 → 交给导入流程
 * 返回 isDragging 供拖拽遮罩层使用。
 */
export function useFileDrop(handlers: FileDropHandlers) {
  const isDragging = ref(false);
  let dragCounter = 0;
  let unlisten: (() => void) | null = null;

  onMounted(async () => {
    try {
      const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      const webview = getCurrentWebviewWindow();
      unlisten = await (webview as any).onFileDropEvent((event: any) => {
        if (event.payload.type === "drop") {
          isDragging.value = false;
          dragCounter = 0;
          const paths: string[] = event.payload.paths;
          if (paths.length === 0) return;
          // 单张图片：命中游戏卡片/详情页 → 设为封面
          if (paths.length === 1 && isImageFile(paths[0])) {
            const pos = event.payload.position;
            const el = pos ? document.elementFromPoint(pos.x, pos.y) : null;
            const target = el?.closest("[data-game-id]") as HTMLElement | null;
            const gameId = target ? Number(target.getAttribute("data-game-id")) : 0;
            if (gameId) {
              handlers.onCoverDrop(gameId, paths[0]);
            } else {
              handlers.onCoverMiss();
            }
            return;
          }
          handlers.onFiles(paths);
        } else if (event.payload.type === "enter") {
          dragCounter++;
          isDragging.value = true;
        } else if (event.payload.type === "leave") {
          dragCounter--;
          if (dragCounter <= 0) {
            isDragging.value = false;
            dragCounter = 0;
          }
        } else if (event.payload.type === "cancel") {
          isDragging.value = false;
          dragCounter = 0;
        }
      });
    } catch (e) {
      console.warn("File drop API not available:", e);
    }
  });

  onUnmounted(() => {
    unlisten?.();
  });

  return { isDragging };
}
