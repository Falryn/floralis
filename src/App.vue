<script setup lang="ts">
import { onMounted, onUnmounted, ref, watchEffect, watch, computed } from "vue";
import { useGameStore, loadImage } from "./stores/gameStore";
import { useModStore } from "./stores/modStore";
import { getCurrentWindow, type Window } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as openUrl } from "@tauri-apps/plugin-shell";
import { useI18n } from "vue-i18n";
import type { UpdateInfo, Mod } from "./types";
import Sidebar from "./components/Sidebar.vue";
import GameGrid from "./components/GameGrid.vue";
import GameDetail from "./components/GameDetail.vue";
import ImportDialog from "./components/ImportDialog.vue";
import type { ExtractedGameData } from "./components/ImportDialog.vue";
import SettingsDialog from "./components/SettingsDialog.vue";
import EditGameDialog from "./components/EditGameDialog.vue";
import type { CreateGameData } from "./components/EditGameDialog.vue";
import ConfirmDialog from "./components/ConfirmDialog.vue";
import GameContextMenu from "./components/GameContextMenu.vue";
import CustomSelect from "./components/CustomSelect.vue";
import StatsPanel from "./components/StatsPanel.vue";
import Toast from "./components/Toast.vue";
import ModManager from "./components/ModManager.vue";
import ModDetail from "./components/ModDetail.vue";
import ModEditDialog from "./components/ModEditDialog.vue";
import ImportModDialog from "./components/ImportModDialog.vue";
import ScanModDialog from "./components/ScanModDialog.vue";
import ModProfilesDialog from "./components/ModProfilesDialog.vue";
import ModContextMenu from "./components/ModContextMenu.vue";
import ViewToolbar from "./components/ViewToolbar.vue";
import TaskCenter from "./components/TaskCenter.vue";
import IntegrityDialog from "./components/IntegrityDialog.vue";
import RandomPickDialog from "./components/RandomPickDialog.vue";
import { initTaskCenter } from "./composables/useTaskCenter";
import { addToast } from "./composables/useToast";
import { openInExplorer } from "./utils/format";

const { t } = useI18n();
const store = useGameStore();
const modStore = useModStore();
const appWindow = getCurrentWindow();
const isMaximized = ref(false);
const showImport = ref(false);
const showSettings = ref(false);
const showIntegrity = ref(false);
const showStats = ref(false);
const showRandomPick = ref(false);
const showEditGame = ref(false);
// 编辑对话框目标游戏 id（从详情页/卡片按钮打开时不依赖右键菜单上下文）
const editGameId = ref<number | undefined>(undefined);
const bannerUrl = ref("");

// Create game flow (import → edit dialog)
const showCreateGame = ref(false);
const createGameData = ref<CreateGameData>({});

function onExtracted(data: ExtractedGameData) {
  createGameData.value = data;
  showCreateGame.value = true;
}

function onGameCreated() {
  showCreateGame.value = false;
}

// Mod view state
const currentView = ref<'games' | 'mods'>('games');
const showModEdit = ref(false);
const showImportMod = ref(false);
const showScanMod = ref(false);
const showModProfiles = ref(false);
const editingMod = ref<Mod | null>(null);
const modEditInitialTab = ref<'basic' | 'assoc'>('basic');

// Mod context menu state
const modCtxMenu = ref<{ modId: number; x: number; y: number } | null>(null);

// Context menu state
const ctxMenu = ref<{ gameId: number; x: number; y: number } | null>(null);
// Track original selected game before opening edit dialog from context menu
const editDialogOriginalGameId = ref<number | null>(null);

// Delete confirmation state
const showDeleteConfirm = ref(false);
const deleteTargetId = ref<number | null>(null);

// Batch delete confirmation
const showBatchDeleteConfirm = ref(false);

// Batch move group picker
const showBatchMoveMenu = ref(false);

// Batch scan loading
const batchScanning = ref(false);

// Batch status/rating menus
const showBatchStatusMenu = ref(false);
const showBatchRatingMenu = ref(false);

// Status options
const statusOptions = [
  { labelKey: 'game.notPlayed', value: 'not_played' },
  { labelKey: 'game.playing', value: 'playing' },
  { labelKey: 'game.completed', value: 'completed' },
  { labelKey: 'game.shelved', value: 'shelved' },
];

// Close menu state
const showCloseMenu = ref(false);
const closeMenuRef = ref<HTMLElement | null>(null);

// View mode
const viewMode = ref<"grid" | "list">("grid");
const gameToolbarRef = ref<InstanceType<typeof ViewToolbar> | null>(null);

// Drag-drop import
const isDragging = ref(false);
const droppedPaths = ref<string[]>([]);
let dragCounter = 0;

const IMAGE_EXTS = ["jpg", "jpeg", "png", "webp", "gif", "bmp"];
function isImageFile(path: string): boolean {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return IMAGE_EXTS.includes(ext);
}

/** 拖入单张图片且命中游戏卡片/详情页 → 设为封面 */
async function handleDropCover(gameId: number, imagePath: string) {
  try {
    await store.setGameCover(gameId, imagePath);
    addToast(t('game.coverUpdated'), "success");
  } catch (e) {
    console.error("封面设置失败:", e);
    addToast(t('game.coverUpdateFail'), "error");
  }
}

// Update notification
const updateAvailable = ref<UpdateInfo | null>(null);

watchEffect(async () => {
  if (store.settings.custom_banner) {
    bannerUrl.value = await loadImage(store.settings.custom_banner);
  } else {
    bannerUrl.value = "";
  }
});

// 横幅模糊度与亮度
const bannerBlur = computed(() => parseInt(store.settings.banner_blur) || 0);
const bannerBrightness = computed(() => parseInt(store.settings.banner_brightness) || 100);
const bannerFilterStyle = computed(() => {
  const filters: string[] = [];
  if (bannerBlur.value > 0) filters.push(`blur(${bannerBlur.value}px)`);
  if (bannerBrightness.value !== 100) filters.push(`brightness(${bannerBrightness.value / 100})`);
  if (filters.length === 0) return {};
  const style: Record<string, string> = { filter: filters.join(' ') };
  if (bannerBlur.value > 0) style.transform = 'scale(1.06)';
  return style;
});

// Apply theme class to root element
const THEME_CLASSES = ["theme-light", "theme-light-sakura", "theme-light-mint", "theme-dark", "theme-dark-ocean", "theme-dark-crimson"];
watch(
  () => store.settings.theme,
  (theme) => {
    const el = document.documentElement;
    THEME_CLASSES.forEach((c) => el.classList.remove(c));
    if (theme && theme !== "light") {
      el.classList.add("theme-" + theme);
    }
  },
  { immediate: true }
);

onMounted(async () => {
  try {
    isMaximized.value = await appWindow.isMaximized();
  } catch (e) {
    console.error("isMaximized failed:", e);
  }
  await Promise.all([
    store.loadGames(),
    store.loadGroups(),
    store.loadSettings(),
    store.loadPasswords(),
    store.loadTags(),
    store.loadAllGameTags(),
  ]);
  modStore.loadMods();
  modStore.loadAllModTags();

  // Migrate external covers to internal storage on startup
  try {
    await invoke("migrate_covers_to_internal");
    // Refresh games after migration (cover paths may have changed)
    await store.loadGames();
  } catch (e) {
    console.warn("Cover migration failed:", e);
  }

  // Silent update check on startup
  try {
    const info = await store.checkForUpdate();
    if (info.available) updateAvailable.value = info;
  } catch (_) {
    // ignore
  }
});

// Keyboard shortcuts
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
    gameToolbarRef.value?.focusSearch();
    return;
  }
  // Ctrl+, → open settings
  if ((e.ctrlKey || e.metaKey) && e.key === ",") {
    e.preventDefault();
    showSettings.value = true;
    return;
  }
  // Ignore when input is focused
  const tag = (e.target as HTMLElement).tagName;
  if (["INPUT", "TEXTAREA", "SELECT"].includes(tag)) return;

  if (e.key === "Escape") {
    if (ctxMenu.value) {
      ctxMenu.value = null;
    } else if (store.isSelectMode) {
      exitSelectMode();
    } else if (store.selectedGameId !== null) {
      store.selectedGameId = null;
    }
  }
  if (e.key === "Delete" && store.selectedGameId !== null) {
    deleteTargetId.value = store.selectedGameId;
    showDeleteConfirm.value = true;
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

// 游玩时长监控事件：后端增量落盘/会话结束时刷新游戏列表（总时长、最后游玩时间）
onMounted(async () => {
  const unlistenTime = await listen("play-time-updated", () => store.loadGames());
  const unlistenEnded = await listen("play-session-ended", () => store.loadGames());
  onUnmounted(() => {
    unlistenTime();
    unlistenEnded();
  });
});

// File drag-drop from OS file manager (using Tauri webview API)
onMounted(async () => {
  try {
    const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    const webview = getCurrentWebviewWindow();
    const unlisten = await (webview as any).onFileDropEvent((event: any) => {
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
            handleDropCover(gameId, paths[0]);
          } else {
            addToast(t('app.dropCoverHint'), "info");
          }
          return;
        }
        droppedPaths.value = paths;
        showImport.value = true;
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
    onUnmounted(() => unlisten());
  } catch (e) {
    console.warn("File drop API not available:", e);
  }
});

// Disable browser default context menu
document.addEventListener("contextmenu", (e) => e.preventDefault());

function onGridContextMenu(id: number, x: number, y: number) {
  ctxMenu.value = { gameId: id, x, y };
}

function onCtxLaunch(id: number, actionId?: number) {
  store.launchGame(id, actionId);
  ctxMenu.value = null;
}

function onCtxEdit(id: number) {
  // 从右键菜单打开编辑对话框时，不设置选中状态，避免触发详情页
  editGameId.value = id;
  showEditGame.value = true;
  ctxMenu.value = null;
}

function onGameDetailEdit() {
  // 从详情页打开编辑对话框：保持详情页打开，仅弹出编辑对话框
  if (store.selectedGameId === null) return;
  editGameId.value = store.selectedGameId;
  showEditGame.value = true;
}

function onCtxDelete(id: number) {
  deleteTargetId.value = id;
  showDeleteConfirm.value = true;
  ctxMenu.value = null;
}

async function onCtxMoveToGroup(gameId: number, groupId: number | null) {
  await store.setGameGroup(gameId, groupId);
  ctxMenu.value = null;
}

async function confirmDeleteGame() {
  if (deleteTargetId.value !== null) {
    await store.deleteGame(deleteTargetId.value);
  }
  showDeleteConfirm.value = false;
  deleteTargetId.value = null;
}

function enterSelectMode() {
  store.isSelectMode = true;
}

function exitSelectMode() {
  store.clearSelection();
}

async function confirmBatchDelete() {
  await store.batchDeleteGames();
  showBatchDeleteConfirm.value = false;
}

async function doBatchMove(groupId: number | null) {
  await store.batchMoveGames(groupId);
  showBatchMoveMenu.value = false;
}

async function doBatchScanCovers() {
  batchScanning.value = true;
  try {
    const ids = store.filteredGames.map((g) => g.id);
    const count = await store.batchScanCovers(ids);
    await store.loadGames();
    addToast(t('batch.scanComplete', { count }), "success");
  } catch (e) {
    console.error("批量扫描封面失败:", e);
  } finally {
    batchScanning.value = false;
  }
}

async function doBatchSetStatus(status: string) {
  await store.batchSetStatus(status);
  showBatchStatusMenu.value = false;
}

async function doBatchSetRating(rating: number) {
  await store.batchSetRating(rating);
  showBatchRatingMenu.value = false;
}

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

async function closeWindow() {
  try {
    await invoke("force_close");
  } catch (e) {
    console.error("force_close failed:", e);
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

// Close dropdown when clicking outside
function handleClickOutside(e: MouseEvent) {
  if (closeMenuRef.value && !closeMenuRef.value.contains(e.target as Node)) {
    showCloseMenu.value = false;
  }
}
onMounted(() => document.addEventListener("click", handleClickOutside));
onUnmounted(() => document.removeEventListener("click", handleClickOutside));

function startResize(direction: string) {
  invoke("start_window_resize", { direction }).catch((e) => {
    console.error("startResize failed:", e);
  });
}

// Mod view handlers
function onSwitchView(view: 'games' | 'mods') {
  currentView.value = view;
  if (view === 'games') {
    modStore.selectedModId = null;
  }
}

function onModManagerImportMod() {
  showImportMod.value = true;
}

function onModManagerScanDir() {
  showScanMod.value = true;
}

function onModManagerProfiles() {
  showModProfiles.value = true;
}

function onModManagerEditMod(id: number) {
  const mod = modStore.mods.find(m => m.id === id);
  if (mod) {
    editingMod.value = { ...mod };
    modEditInitialTab.value = 'basic';
    showModEdit.value = true;
  }
}

function onModContextMenu(modId: number, x: number, y: number) {
  modCtxMenu.value = { modId, x, y };
}

function onModCtxEdit(id: number) {
  const mod = modStore.mods.find(m => m.id === id);
  if (mod) {
    editingMod.value = { ...mod };
    modEditInitialTab.value = 'basic';
    showModEdit.value = true;
  }
  modCtxMenu.value = null;
}

function onModCtxDelete(id: number) {
  modStore.selectedModId = id;
  showModDeleteConfirm.value = true;
  modCtxMenu.value = null;
}

function onModCtxLinkGame(id: number) {
  const mod = modStore.mods.find(m => m.id === id);
  if (mod) {
    editingMod.value = { ...mod };
    modEditInitialTab.value = 'assoc';
    showModEdit.value = true;
  }
  modCtxMenu.value = null;
}

function onModDetailEdit() {
  if (modStore.selectedMod) {
    editingMod.value = { ...modStore.selectedMod };
    modEditInitialTab.value = 'basic';
    showModEdit.value = true;
  }
}

function onModDetailOpenDir() {
  if (modStore.selectedMod?.mod_path) {
    openInExplorer(modStore.selectedMod.mod_path);
  }
}

// Mod delete confirmation state
const showModDeleteConfirm = ref(false);

async function onModDetailDelete() {
  if (modStore.selectedMod) {
    showModDeleteConfirm.value = true;
  }
}

async function confirmModDelete() {
  if (modStore.selectedMod) {
    await modStore.deleteMod(modStore.selectedMod.id);
  }
  showModDeleteConfirm.value = false;
}

function onModEditClose() {
  showModEdit.value = false;
  editingMod.value = null;
  modEditInitialTab.value = 'basic';
}

function onModEditSaved() {
  showModEdit.value = false;
  editingMod.value = null;
  modStore.loadMods();
  modStore.loadAllModTags();
}

function onImportModClose() {
  showImportMod.value = false;
}

function onImportModImported() {
  showImportMod.value = false;
  modStore.loadMods();
  modStore.loadAllModTags();
}

function onScanModClose() {
  showScanMod.value = false;
}

function onScanModImported() {
  showScanMod.value = false;
  modStore.loadMods();
  modStore.loadAllModTags();
}

function onGameDetailManageMods() {
  const gameId = store.selectedGameId;
  if (gameId) {
    modStore.modFilterGameId = gameId;
    currentView.value = 'mods';
  }
}

function handleMainClick(e: MouseEvent) {
  const target = e.target as HTMLElement;
  // 点击在详情页面板内 → 不处理
  if (target.closest(".game-detail-panel")) return;
  if (target.closest(".mod-detail-panel")) return;
  // 点击在游戏卡片内 → 不处理
  if (target.closest("[draggable='true']")) return;
  // 点击在批量移动菜单内 → 不处理
  if (target.closest(".batch-move-menu")) return;
  // 点击在批量状态菜单内 → 不处理
  if (target.closest(".batch-status-menu")) return;
  // 点击在批量评分菜单内 → 不处理
  if (target.closest(".batch-rating-menu")) return;
  // 关闭批量菜单
  showBatchMoveMenu.value = false;
  showBatchStatusMenu.value = false;
  showBatchRatingMenu.value = false;
  // 其他空白区域 → 关闭详情页
  store.selectedGameId = null;
  modStore.selectedModId = null;
}
</script>

<template>
  <div class="relative flex h-screen overflow-hidden bg-gradient-main">
    <!-- Resize edges -->
    <div class="absolute top-0 left-4 right-4 h-1 cursor-n-resize z-50" @mousedown="startResize('north')" />
    <div class="absolute bottom-0 left-4 right-4 h-1 cursor-s-resize z-50" @mousedown="startResize('south')" />
    <div class="absolute top-4 bottom-4 left-0 w-1 cursor-w-resize z-50" @mousedown="startResize('west')" />
    <div class="absolute top-4 bottom-4 right-0 w-1 cursor-e-resize z-50" @mousedown="startResize('east')" />
    <div class="absolute top-0 left-0 w-5 h-5 cursor-nw-resize z-50" @mousedown="startResize('north-west')" />
    <div class="absolute top-0 right-0 w-5 h-5 cursor-ne-resize z-50" @mousedown="startResize('north-east')" />
    <div class="absolute bottom-0 left-0 w-5 h-5 cursor-sw-resize z-50" @mousedown="startResize('south-west')" />
    <div class="absolute bottom-0 right-0 w-5 h-5 cursor-se-resize z-50" @mousedown="startResize('south-east')" />

    <!-- Sidebar drag region (same height as title bar) -->
    <div class="absolute top-0 left-0 w-72 h-9 z-40" data-tauri-drag-region />
    <!-- Note: sidebar header + view switcher are above the title bar drag region -->

    <!-- Sidebar (full height, left side) -->
    <Sidebar :currentView="currentView" @settings="showSettings = true" @switchView="onSwitchView" />

    <!-- Right side: title bar + content -->
    <div class="flex flex-col flex-1 min-w-0">
      <!-- Custom Title Bar -->
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

      <!-- Main Content -->
      <main class="flex-1 relative overflow-hidden flex flex-col" @click.capture="handleMainClick">
        <!-- Update notification -->
        <div
          v-if="updateAvailable"
          class="mx-8 mt-2 px-4 py-2.5 rounded-xl bg-primary-500/10 border border-primary-400/30 flex items-center justify-between text-sm shrink-0"
        >
          <span class="text-primary-600">
            ✨ {{ t('app.newVersion', { version: updateAvailable.latest_version, current: updateAvailable.current_version }) }}
            <button
              v-if="updateAvailable.release_url"
              class="ml-2 underline text-primary-500 hover:text-primary-700"
              @click="openUrl(updateAvailable.release_url)"
            >
              {{ t('app.viewRelease') }}
            </button>
          </span>
          <button class="text-primary-400 hover:text-primary-600 text-xs" @click="updateAvailable = null">✕</button>
        </div>

        <!-- Banner（游戏/Mod 视图共享） -->
        <div v-if="bannerUrl" class="mx-8 mt-3 shrink-0">
          <div class="w-full h-28 rounded-2xl overflow-hidden shadow-md">
            <img
              :src="bannerUrl"
              class="w-full h-full object-cover"
              alt="banner"
              :style="bannerFilterStyle"
            />
          </div>
        </div>

        <!-- Games View -->
        <div v-show="currentView === 'games'" class="flex-1 min-h-0 flex flex-col @container">
          <div class="flex-1 overflow-auto p-8">
          <!-- Top bar -->
          <ViewToolbar
            ref="gameToolbarRef"
            :title="store.selectedGroupId ? store.groups.find(g => g.id === store.selectedGroupId)?.name ?? t('group.ungrouped') : t('group.allGames')"
            :subtitle="t('app.totalGames', { count: store.filteredGames.length })"
            v-model:searchModelValue="store.searchInput"
            :searchPlaceholder="t('app.search')"
            :sortModelValue="store.sortType"
            @update:sortModelValue="(v: string) => (store.sortType = v as typeof store.sortType)"
            :sortOptions="[
              { label: t('sort.recentlyAdded'), value: 'created_desc' },
              { label: t('sort.earliestAdded'), value: 'created_asc' },
              { label: t('sort.recentlyPlayed'), value: 'last_played' },
              { label: t('sort.highestRated'), value: 'rating_desc' },
              { label: t('sort.nameAZ'), value: 'name_asc' },
              { label: t('sort.nameZA'), value: 'name_desc' },
            ]"
            :showSelectMode="true"
            :isSelectMode="store.isSelectMode"
            v-model:viewMode="viewMode"
            @enterSelectMode="enterSelectMode"
          >
            <template #subtitle-extra>
              <span v-if="store.isSelectMode" class="text-primary-500 font-medium ml-2">
                {{ t('app.selectedCount', { count: store.selectedGameIds.size }) }}
              </span>
            </template>
            <template #filters>
              <CustomSelect
                v-model="store.selectedTagId"
                :options="[
                  { label: t('tag.allTags'), value: null },
                  ...store.tags.map(tg => ({ label: tg.name, value: tg.id as number })),
                ]"
                class="w-28"
                searchable
              />
            </template>
            <template #batch-actions>
                <button
                  class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-main hover:bg-primary-50 transition-colors"
                  @click="store.selectAll()"
                >
                  {{ t('app.selectAll') }}
                </button>
                <div class="relative batch-move-menu">
                  <button
                    class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-main hover:bg-primary-50 transition-colors"
                    @click="showBatchMoveMenu = !showBatchMoveMenu"
                  >
                    {{ t('batch.moveToGroup') }} ▾
                  </button>
                  <div
                    v-if="showBatchMoveMenu"
                    class="absolute top-full mt-1 right-0 z-[80] bg-modal-bg border border-border-light rounded-xl shadow-2xl py-2 min-w-[140px]"
                  >
                    <button
                      class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 transition-colors"
                      @click="doBatchMove(null)"
                    >
                      {{ t('batch.ungrouped') }}
                    </button>
                    <button
                      v-for="g in store.groups"
                      :key="g.id"
                      class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 transition-colors"
                      @click="doBatchMove(g.id)"
                    >
                      {{ g.name }}
                    </button>
                  </div>
                </div>
                <button
                  class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-main hover:bg-primary-50 transition-colors"
                  :disabled="batchScanning"
                  @click="doBatchScanCovers"
                >
                  {{ batchScanning ? t('batch.scanning') : t('batch.scanCovers') }}
                </button>
                <div class="relative batch-status-menu">
                  <button
                    class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-main hover:bg-primary-50 transition-colors"
                    @click="showBatchStatusMenu = !showBatchStatusMenu"
                  >
                    {{ t('batch.setStatus') }} ▾
                  </button>
                  <div
                    v-if="showBatchStatusMenu"
                    class="absolute top-full mt-1 right-0 z-[80] bg-modal-bg border border-border-light rounded-xl shadow-2xl py-2 min-w-[140px]"
                  >
                    <button
                      v-for="opt in statusOptions"
                      :key="opt.value"
                      class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 transition-colors"
                      @click="doBatchSetStatus(opt.value)"
                    >
                      {{ t(opt.labelKey) }}
                    </button>
                  </div>
                </div>
                <div class="relative batch-rating-menu">
                  <button
                    class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-main hover:bg-primary-50 transition-colors"
                    @click="showBatchRatingMenu = !showBatchRatingMenu"
                  >
                    {{ t('batch.setRating') }} ▾
                  </button>
                  <div
                    v-if="showBatchRatingMenu"
                    class="absolute top-full mt-1 right-0 z-[80] bg-modal-bg border border-border-light rounded-xl shadow-2xl py-2 min-w-[100px]"
                  >
                    <button
                      v-for="r in [1, 2, 3, 4, 5]"
                      :key="r"
                      class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 transition-colors"
                      @click="doBatchSetRating(r)"
                    >
                      {{ r }} {{ t('batch.star') }}
                    </button>
                    <button
                      class="w-full px-4 py-2 text-sm text-left text-text-sub hover:bg-primary-50 transition-colors"
                      @click="doBatchSetRating(0)"
                    >
                      {{ t('batch.clearRating') }}
                    </button>
                  </div>
                </div>
                <button
                  class="px-3 py-2 text-sm rounded-xl bg-red-500 text-white hover:bg-red-600 transition-colors"
                  @click="showBatchDeleteConfirm = true"
                >
                  {{ t('batch.delete') }}
                </button>
                <button
                  class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-sub hover:bg-code-bg transition-colors"
                  @click="exitSelectMode"
                >
                  {{ t('app.cancel') }}
                </button>
            </template>
          </ViewToolbar>

          <GameGrid
            :games="store.filteredGames"
            :view-mode="viewMode"
            @select="(id) => (store.selectedGameId = id)"
            @edit="(id) => { editGameId = id; showEditGame = true; }"
            @contextmenu="onGridContextMenu"
          />
          </div>

          <!-- Bottom Action Bar -->
          <div class="flex items-center gap-3 px-8 py-6 border-t border-border-light shrink-0">
            <button
              class="px-4 py-2.5 rounded-xl bg-gradient-to-r from-primary-500 to-primary-600 text-white text-sm font-medium hover:from-primary-600 hover:to-primary-700 transition-all shadow-lg shadow-primary-500/20"
              @click="showImport = true"
            >
              ✨ {{ t('import.title') }}
            </button>
            <button
              class="px-4 py-2.5 rounded-xl border border-border-medium text-sm text-text-sub hover:bg-input-bg transition-colors"
              @click="showStats = true"
            >
              📊 {{ t('stats.title') }}
            </button>
            <button
              class="px-4 py-2.5 rounded-xl border border-border-medium text-sm text-text-sub hover:bg-input-bg transition-colors"
              @click="showRandomPick = true"
            >
              🎲 {{ t('random.title') }}
            </button>
          </div>
        </div>

        <!-- Mods View -->
        <div v-show="currentView === 'mods'" class="flex-1 min-h-0 flex flex-col @container">
          <ModManager @importMod="onModManagerImportMod" @scanDir="onModManagerScanDir" @profiles="onModManagerProfiles" @contextMenu="onModContextMenu" @editMod="onModManagerEditMod" />
        </div>

        <!-- Detail panel: 绝对定位浮动，不挤压列表 -->
        <transition name="slide">
          <GameDetail
            v-if="currentView === 'games' && store.selectedGame"
            class="absolute top-0 right-0 h-full z-10"
            :game="store.selectedGame"
            @close="store.selectedGameId = null"
            @edit="onGameDetailEdit"
            @launch="(actionId?: number) => store.launchGame(store.selectedGameId!, actionId)"
            @manageMods="onGameDetailManageMods"
          />
        </transition>

        <!-- Mod Detail panel: 绝对定位浮动，不挤压列表 -->
        <transition name="slide">
          <ModDetail
            v-if="currentView === 'mods' && modStore.selectedMod"
            class="absolute top-0 right-0 h-full z-10"
            @editMod="onModDetailEdit"
            @openDir="onModDetailOpenDir"
            @deleteMod="onModDetailDelete"
            @linkGame="onModDetailEdit"
            @addTag="onModDetailEdit"
          />
        </transition>
      </main>
    </div>

    <transition name="modal">
      <ImportDialog v-if="showImport" :initial-paths="droppedPaths" @close="showImport = false; droppedPaths = []" @extracted="onExtracted" @batch-imported="showImport = false" />
    </transition>
    <transition name="modal">
      <SettingsDialog v-if="showSettings" @close="showSettings = false" @openIntegrity="showIntegrity = true" />
    </transition>
    <transition name="modal">
      <IntegrityDialog v-if="showIntegrity" @close="showIntegrity = false" />
    </transition>
    <transition name="modal">
      <StatsPanel v-if="showStats" @close="showStats = false" />
    </transition>
    <transition name="modal">
      <RandomPickDialog v-if="showRandomPick" @close="showRandomPick = false" />
    </transition>
    <transition name="modal">
      <EditGameDialog
        v-if="showEditGame"
        :game-id="editGameId"
        @close="() => { showEditGame = false; editGameId = undefined; }"
      />
    </transition>
    <transition name="modal">
      <EditGameDialog
        v-if="showCreateGame"
        :create-data="createGameData"
        @close="showCreateGame = false"
        @created="onGameCreated"
      />
    </transition>

    <!-- Context Menu -->
    <GameContextMenu
      v-if="ctxMenu"
      :game-id="ctxMenu.gameId"
      :x="ctxMenu.x"
      :y="ctxMenu.y"
      @close="ctxMenu = null"
      @launch="onCtxLaunch"
      @edit="onCtxEdit"
      @delete="onCtxDelete"
      @move-to-group="onCtxMoveToGroup"
    />

    <!-- Delete Game Confirmation -->
    <transition name="modal">
      <ConfirmDialog
        v-if="showDeleteConfirm"
        :title="t('game.delete')"
        :message="t('game.confirmDelete')"
        :confirm-text="t('common.delete')"
        :danger="true"
        @confirm="confirmDeleteGame"
        @cancel="showDeleteConfirm = false"
      />
    </transition>

    <!-- Batch Delete Confirmation -->
    <transition name="modal">
      <ConfirmDialog
        v-if="showBatchDeleteConfirm"
        :title="t('batch.delete')"
        :message="t('batch.confirmDelete')"
        :confirm-text="t('common.delete')"
        :danger="true"
        @confirm="confirmBatchDelete"
        @cancel="showBatchDeleteConfirm = false"
      />
    </transition>

    <!-- Mod Context Menu -->
    <ModContextMenu
      v-if="modCtxMenu"
      :mod-id="modCtxMenu.modId"
      :x="modCtxMenu.x"
      :y="modCtxMenu.y"
      @close="modCtxMenu = null"
      @edit="onModCtxEdit"
      @delete="onModCtxDelete"
      @link-game="onModCtxLinkGame"
    />

    <!-- Mod Delete Confirmation -->
    <transition name="modal">
      <ConfirmDialog
        v-if="showModDeleteConfirm"
        :title="t('mod.delete')"
        :message="t('mod.deleteConfirm')"
        :confirm-text="t('common.delete')"
        :danger="true"
        @confirm="confirmModDelete"
        @cancel="showModDeleteConfirm = false"
      />
    </transition>

    <!-- Background Task Center -->
    <TaskCenter />

    <!-- Toast Notifications -->
    <Toast />

    <!-- Mod Edit Dialog -->
    <transition name="modal">
      <ModEditDialog
        v-if="showModEdit"
        :mod="editingMod"
        :initial-tab="modEditInitialTab"
        @close="onModEditClose"
        @saved="onModEditSaved"
      />
    </transition>

    <!-- Import Mod Dialog -->
    <transition name="modal">
      <ImportModDialog
        v-if="showImportMod"
        @close="onImportModClose"
        @imported="onImportModImported"
      />
    </transition>

    <!-- Scan Mod Dialog -->
    <transition name="modal">
      <ScanModDialog
        v-if="showScanMod"
        @close="onScanModClose"
        @imported="onScanModImported"
      />
    </transition>

    <!-- Mod Profiles Dialog -->
    <transition name="modal">
      <ModProfilesDialog
        v-if="showModProfiles"
        @close="showModProfiles = false"
      />
    </transition>

    <!-- Drag-drop overlay -->
    <transition name="fade">
      <div
        v-if="isDragging"
        class="fixed inset-0 z-[200] flex items-center justify-center bg-primary-500/20 backdrop-blur-sm pointer-events-none"
      >
        <div class="px-8 py-6 rounded-3xl bg-modal-bg border-2 border-dashed border-primary-400 shadow-2xl text-center">
          <p class="text-4xl mb-2">📂</p>
          <p class="text-lg font-medium text-text-main">{{ t('app.dropToImport') }}</p>
        </div>
      </div>
    </transition>
  </div>
</template>
