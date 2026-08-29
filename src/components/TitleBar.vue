<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import { useGameStore } from "../stores/gameStore";

const { t } = useI18n();
const store = useGameStore();
const appWindow = getCurrentWindow();

const isMaximized = ref(false);
const showCloseMenu = ref(false);
const closeMenuRef = ref<HTMLElement | null>(null);

onMounted(async () => {
  try {
    isMaximized.value = await appWindow.isMaximized();
  } catch (e) {
    console.error("isMaximized failed:", e);
  }
});

async function toggleMaximize() {
  try {
    await appWindow.toggleMaximize();
    isMaximized.value = await appWindow.isMaximized();
  } catch (e) {
    console.error("toggleMaximize failed:", e);
  }
}

async function minimizeWindow() {
  try {
    await appWindow.minimize();
  } catch (e) {
    console.error("minimize failed:", e);
  }
}

function toggleCloseMenu() {
  const behavior = store.settings.close_behavior || "ask";
  if (behavior === "exit") {
    confirmExit();
  } else if (behavior === "minimize") {
    minimizeToTray();
  } else {
    showCloseMenu.value = !showCloseMenu.value;
  }
}

async function minimizeToTray() {
  showCloseMenu.value = false;
  try {
    await appWindow.hide();
  } catch (e) {
    console.error("hide failed:", e);
  }
}

async function confirmExit() {
  showCloseMenu.value = false;
  try {
    await invoke("force_close");
  } catch (e) {
    console.error("force_close failed:", e);
  }
}

// 点击菜单外部时收起下拉
function handleClickOutside(e: MouseEvent) {
  if (closeMenuRef.value && !closeMenuRef.value.contains(e.target as Node)) {
    showCloseMenu.value = false;
  }
}
onMounted(() => document.addEventListener("click", handleClickOutside));
onUnmounted(() => document.removeEventListener("click", handleClickOutside));
</script>

<template>
  <div
    class="flex items-center h-9 select-none shrink-0"
    data-tauri-drag-region
  >
    <!-- Left: drag region -->
    <div class="flex-1" data-tauri-drag-region />

    <!-- Right: Window controls -->
    <div class="flex h-full">
      <button
        class="w-11 h-full flex items-center justify-center text-text-sub hover:bg-icon-hover transition-colors"
        @click="minimizeWindow"
      >
        <svg width="10" height="1" viewBox="0 0 10 1"><rect width="10" height="1" fill="currentColor"/></svg>
      </button>
      <button
        class="w-11 h-full flex items-center justify-center text-text-sub hover:bg-icon-hover transition-colors"
        @click="toggleMaximize"
      >
        <svg v-if="!isMaximized" width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1">
          <rect x="0.5" y="0.5" width="9" height="9"/>
        </svg>
        <svg v-else width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1">
          <rect x="2.5" y="0.5" width="7" height="7"/>
          <rect x="0.5" y="2.5" width="7" height="7"/>
        </svg>
      </button>
      <div class="relative" ref="closeMenuRef">
        <button
          class="w-11 h-full flex items-center justify-center text-text-sub hover:bg-red-500 hover:text-white transition-colors"
          @click.stop="toggleCloseMenu"
        >
          <svg width="10" height="10" viewBox="0 0 10 10" stroke="currentColor" stroke-width="1.2">
            <line x1="0" y1="0" x2="10" y2="10"/>
            <line x1="10" y1="0" x2="0" y2="10"/>
          </svg>
        </button>
        <transition name="fade">
          <div
            v-if="showCloseMenu"
            class="absolute top-full right-0 mt-1 z-[100] bg-modal-bg border border-border-light rounded-xl shadow-2xl py-2 min-w-[160px]"
          >
            <button
              class="w-full px-4 py-2.5 text-sm text-left text-text-main hover:bg-icon-hover transition-colors flex items-center gap-2"
              @click="minimizeToTray"
            >
              <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.2">
                <rect x="1" y="1" width="12" height="12" rx="2"/>
                <line x1="1" y1="10" x2="13" y2="10"/>
              </svg>
              {{ t('settings.minimizeToTray') }}
            </button>
            <button
              class="w-full px-4 py-2.5 text-sm text-left text-red-400 hover:bg-red-500/10 transition-colors flex items-center gap-2"
              @click="confirmExit"
            >
              <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.2">
                <path d="M5 1h6a2 2 0 012 2v8a2 2 0 01-2 2H5"/>
                <path d="M1 7h8"/>
                <path d="M6 4l3 3-3 3"/>
              </svg>
              {{ t('common.close') }}
            </button>
          </div>
        </transition>
      </div>
    </div>
  </div>
</template>
