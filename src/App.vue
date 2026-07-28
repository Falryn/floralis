<script setup lang="ts">
import { onMounted, onUnmounted, ref, watchEffect, watch } from "vue";
import { useGameStore, loadImage } from "./stores/gameStore";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { open as openUrl } from "@tauri-apps/plugin-shell";
import { useI18n } from "vue-i18n";
import type { UpdateInfo } from "./types";
import Sidebar from "./components/Sidebar.vue";
import GameGrid from "./components/GameGrid.vue";
import GameDetail from "./components/GameDetail.vue";
import ImportDialog from "./components/ImportDialog.vue";
import SettingsDialog from "./components/SettingsDialog.vue";
import EditGameDialog from "./components/EditGameDialog.vue";
import ConfirmDialog from "./components/ConfirmDialog.vue";
import GameContextMenu from "./components/GameContextMenu.vue";
import CustomSelect from "./components/CustomSelect.vue";
import StatsPanel from "./components/StatsPanel.vue";
import Toast from "./components/Toast.vue";
import { addToast } from "./composables/useToast";

const { t } = useI18n();
const store = useGameStore();
const appWindow = getCurrentWindow();
const isMaximized = ref(false);
const showImport = ref(false);
const showSettings = ref(false);
const showStats = ref(false);
const showEditGame = ref(false);
const bannerUrl = ref("");

// Context menu state
const ctxMenu = ref<{ gameId: number; x: number; y: number } | null>(null);

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
const searchInputRef = ref<HTMLInputElement | null>(null);

// Drag-drop import
const isDragging = ref(false);
const droppedPath = ref("");
let dragCounter = 0;

// Update notification
const updateAvailable = ref<UpdateInfo | null>(null);

watchEffect(async () => {
  if (store.settings.custom_banner) {
    bannerUrl.value = await loadImage(store.settings.custom_banner);
  } else {
    bannerUrl.value = "";
  }
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
  // Ctrl+F → focus search
  if ((e.ctrlKey || e.metaKey) && e.key === "f") {
    e.preventDefault();
    searchInputRef.value?.focus();
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

// File drag-drop from OS file manager (using Tauri webview API)
onMounted(async () => {
  try {
    const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    const webview = getCurrentWebviewWindow();
    const unlisten = await (webview as any).onFileDropEvent((event: any) => {
      if (event.payload.type === "drop") {
        isDragging.value = false;
        dragCounter = 0;
        const paths = event.payload.paths;
        if (paths.length > 0) {
          droppedPath.value = paths[0];
          showImport.value = true;
        }
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

function onCtxLaunch(id: number) {
  store.launchGame(id);
  ctxMenu.value = null;
}

function onCtxEdit(id: number) {
  store.selectedGameId = id;
  showEditGame.value = true;
  ctxMenu.value = null;
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

function handleMainClick(e: MouseEvent) {
  const target = e.target as HTMLElement;
  // 点击在详情页面板内 → 不处理
  if (target.closest(".game-detail-panel")) return;
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

    <!-- Sidebar (full height, left side) -->
    <Sidebar @import="showImport = true" @settings="showSettings = true" @stats="showStats = true" />

    <!-- Right side: title bar + content -->
    <div class="flex flex-col flex-1 min-w-0">
      <!-- Custom Title Bar -->
      <div
        class="flex items-center justify-end h-9 select-none shrink-0"
        data-tauri-drag-region
      >
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
      <main class="flex-1 relative overflow-hidden" @click.capture="handleMainClick">
        <!-- Update notification -->
        <div
          v-if="updateAvailable"
          class="mx-8 mt-2 px-4 py-2.5 rounded-xl bg-primary-500/10 border border-primary-400/30 flex items-center justify-between text-sm"
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

        <div class="h-full overflow-auto p-8 @container">
          <!-- Banner -->
          <div
            v-if="bannerUrl"
            class="w-full h-44 rounded-3xl overflow-hidden mb-10 shadow-md"
          >
            <img :src="bannerUrl" class="w-full h-full object-cover" alt="banner" />
          </div>

          <!-- Top bar -->
          <div class="flex items-end justify-between mb-10 gap-4">
            <div>
              <h2 class="text-2xl font-bold text-text-main">
                {{ store.selectedGroupId ? store.groups.find(g => g.id === store.selectedGroupId)?.name ?? t('group.ungrouped') : t('group.allGames') }}
              </h2>
              <p class="text-sm text-text-sub mt-1">
                {{ t('app.totalGames', { count: store.filteredGames.length }) }}
                <span v-if="store.isSelectMode" class="text-primary-500 font-medium ml-2">
                  {{ t('app.selectedCount', { count: store.selectedGameIds.size }) }}
                </span>
              </p>
            </div>
            <div class="flex items-center gap-2 shrink-0">
              <!-- Batch action bar (visible in select mode) -->
              <template v-if="store.isSelectMode">
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
              <!-- Normal mode controls -->
              <template v-else>
                <button
                  class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-sub hover:bg-primary-50 hover:text-text-main transition-colors"
                  @click="enterSelectMode"
                >
                  {{ t('app.multiSelect') }}
                </button>
                <!-- View toggle -->
                <button
                  class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-sub hover:bg-primary-50 hover:text-text-main transition-colors"
                  @click="viewMode = viewMode === 'grid' ? 'list' : 'grid'"
                  :title="viewMode === 'grid' ? t('app.switchToList') : t('app.switchToGrid')"
                >
                  {{ viewMode === 'grid' ? '☰' : '⊞' }}
                </button>
                <!-- Search -->
                <div class="relative">
                  <input
                    ref="searchInputRef"
                    v-model="store.searchInput"
                    type="text"
                    :placeholder="t('app.search')"
                    class="w-48 pl-8 pr-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-main placeholder-text-sub/50 outline-none focus:border-primary-400 transition-colors"
                  />
                  <svg class="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-text-sub/50" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="11" cy="11" r="8"/>
                    <line x1="21" y1="21" x2="16.65" y2="16.65"/>
                  </svg>
                </div>
                <!-- Sort -->
                <CustomSelect
                  v-model="store.sortType"
                  :options="[
                    { label: t('sort.recentlyAdded'), value: 'created_desc' },
                    { label: t('sort.earliestAdded'), value: 'created_asc' },
                    { label: t('sort.recentlyPlayed'), value: 'last_played' },
                    { label: t('sort.highestRated'), value: 'rating_desc' },
                    { label: t('sort.nameAZ'), value: 'name_asc' },
                    { label: t('sort.nameZA'), value: 'name_desc' },
                  ]"
                  class="w-32"
                />
                <!-- Tag filter -->
                <CustomSelect
                  v-model="store.selectedTagId"
                  :options="[
                    { label: t('tag.allTags'), value: null },
                    ...store.tags.map(t => ({ label: t.name, value: t.id as number })),
                  ]"
                  class="w-28"
                />
              </template>
            </div>
          </div>

          <GameGrid
            :games="store.filteredGames"
            :view-mode="viewMode"
            @select="(id) => (store.selectedGameId = id)"
            @edit="
              (id) => {
                store.selectedGameId = id;
                showEditGame = true;
              }
            "
            @contextmenu="onGridContextMenu"
          />
        </div>

        <!-- Detail panel: 绝对定位浮动，不挤压列表 -->
        <transition name="slide">
          <GameDetail
            v-if="store.selectedGame"
            class="absolute top-0 right-0 h-full z-10"
            :game="store.selectedGame"
            @close="store.selectedGameId = null"
            @edit="showEditGame = true"
            @launch="store.launchGame(store.selectedGameId!)"
          />
        </transition>
      </main>
    </div>

    <transition name="modal">
      <ImportDialog v-if="showImport" :initial-path="droppedPath" @close="showImport = false; droppedPath = ''" />
    </transition>
    <transition name="modal">
      <SettingsDialog v-if="showSettings" @close="showSettings = false" />
    </transition>
    <transition name="modal">
      <StatsPanel v-if="showStats" @close="showStats = false" />
    </transition>
    <transition name="modal">
      <EditGameDialog
        v-if="showEditGame && store.selectedGame"
        :game="store.selectedGame"
        @close="showEditGame = false"
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

    <!-- Toast Notifications -->
    <Toast />

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
