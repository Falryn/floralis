/**
 * 封面按需加载
 *
 * 通过 IntersectionObserver 只在卡片接近视口时才生成/加载缩略图，
 * 避免大库启动时对所有封面并行发起 generate_thumbnail + 图片解码。
 * 保留原有语义：封面路径变化时重新加载；缩略图失败回落原图。
 */

import { onUnmounted, ref, type Directive } from "vue";
import { invoke } from "../utils/invoke";
import { loadImage } from "../stores/gameStore";

export function useLazyCovers() {
  const coverUrls = ref<Map<number, string>>(new Map());
  const landscapeIds = ref<Set<number>>(new Set());
  const loadedCoverPath = new Map<number, string>();
  const pendingIds = new Set<number>();
  // 当前处于视口（含 rootMargin）内的卡片元素
  const intersecting = new WeakSet<Element>();

  function tryLoad(el: HTMLElement) {
    const id = Number(el.dataset.coverId);
    const path = el.dataset.coverPath ?? "";
    if (!id || !path) return;
    if (loadedCoverPath.get(id) === path) return;
    if (pendingIds.has(id)) return;
    pendingIds.add(id);
    loadedCoverPath.set(id, path);
    loadCover(id, path).finally(() => pendingIds.delete(id));
  }

  function detectOrientation(id: number, url: string) {
    const img = new Image();
    img.onload = () => {
      if (img.naturalWidth > img.naturalHeight) {
        const next = new Set(landscapeIds.value);
        next.add(id);
        landscapeIds.value = next;
      }
    };
    img.src = url;
  }

  async function loadCover(id: number, path: string) {
    let finalUrl = "";
    try {
      const thumbPath = await invoke<string>("generate_thumbnail", {
        sourcePath: path,
        gameId: id,
      });
      const url = loadImage(thumbPath);
      if (url) finalUrl = url;
    } catch (_) {
      // Thumbnail generation failed, fall back to original
    }
    if (!finalUrl) {
      const url = loadImage(path);
      if (url) finalUrl = url;
    }
    if (finalUrl) {
      const next = new Map(coverUrls.value);
      next.set(id, finalUrl);
      coverUrls.value = next;
      landscapeIds.value.delete(id);
      detectOrientation(id, finalUrl);
    }
  }

  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        const el = entry.target as HTMLElement;
        if (entry.isIntersecting) {
          intersecting.add(el);
          tryLoad(el);
        } else {
          intersecting.delete(el);
        }
      }
    },
    { rootMargin: "300px" }
  );

  /** 卡片根元素上使用的指令：进入视口附近时按需加载封面 */
  const vObserveCover: Directive<HTMLElement> = {
    mounted(el) {
      observer.observe(el);
    },
    // 封面路径可能在卡片仍可见时被更新（如换封面），重渲染后重试加载
    updated(el) {
      if (intersecting.has(el)) tryLoad(el);
    },
    unmounted(el) {
      observer.unobserve(el);
      intersecting.delete(el);
    },
  };

  function clearCover(id: number) {
    const next = new Map(coverUrls.value);
    next.delete(id);
    coverUrls.value = next;
    loadedCoverPath.delete(id);
  }

  onUnmounted(() => observer.disconnect());

  return { coverUrls, landscapeIds, vObserveCover, clearCover };
}
