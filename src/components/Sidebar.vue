<script setup lang="ts">
import { ref, computed, watchEffect } from "vue";
import { useGameStore, loadImage } from "../stores/gameStore";
import { useModStore } from "../stores/modStore";
import { useI18n } from "vue-i18n";
import type { Group } from "../types";
import ConfirmDialog from "./ConfirmDialog.vue";

const { t } = useI18n();

const props = defineProps<{
  currentView?: 'games' | 'mods';
}>();

const emit = defineEmits<{
  settings: [];
  about: [];
  switchView: [view: 'games' | 'mods'];
}>();

const store = useGameStore();
const modStore = useModStore();
const newGroupName = ref("");
const editingGroupId = ref<number | null>(null);
const editingName = ref("");

function selectGroup(id: number | null) {
  store.selectedGroupId = id;
}

async function createGroup() {
  const name = newGroupName.value.trim();
  if (!name) return;
  await store.addGroup(name);
  newGroupName.value = "";
}

function startRename(group: { id: number; name: string }) {
  editingGroupId.value = group.id;
  editingName.value = group.name;
}

async function confirmRename(id: number) {
  const name = editingName.value.trim();
  if (!name) return;
  await store.renameGroup(id, name);
  editingGroupId.value = null;
}

function cancelRename() {
  editingGroupId.value = null;
}

function requestDeleteGroup(group: { id: number; name: string }) {
  deletingGroupId.value = group.id;
  deletingGroupName.value = group.name;
}

async function confirmDeleteGroup() {
  if (deletingGroupId.value !== null) {
    await store.deleteGroup(deletingGroupId.value);
  }
  deletingGroupId.value = null;
}

const sidebarBgUrl = ref("");
const deletingGroupId = ref<number | null>(null);
const deletingGroupName = ref("");
const dragOverGroupId = ref<number | null>(null);

function onGroupDragStart(group: Group, e: DragEvent) {
  e.dataTransfer?.setData("text/group-id", String(group.id));
  e.dataTransfer!.effectAllowed = "move";
}

function onGroupDragOver(group: Group, e: DragEvent) {
  e.preventDefault();
  e.dataTransfer!.dropEffect = "move";
  dragOverGroupId.value = group.id;
}

function onGroupDragLeave() {
  dragOverGroupId.value = null;
}

async function onGroupDrop(targetGroup: Group, e: DragEvent) {
  e.preventDefault();
  dragOverGroupId.value = null;
  const draggedId = Number(e.dataTransfer?.getData("text/group-id"));
  if (!draggedId || draggedId === targetGroup.id) return;

  // Build new order: move dragged group to target position
  const ids = store.groups.map((g) => g.id);
  const fromIdx = ids.indexOf(draggedId);
  const toIdx = ids.indexOf(targetGroup.id);
  if (fromIdx === -1 || toIdx === -1) return;

  ids.splice(fromIdx, 1);
  ids.splice(toIdx, 0, draggedId);
  await store.reorderGroups(ids);
}

watchEffect(async () => {
  const path = store.settings.custom_sidebar_bg;
  if (path) {
    sidebarBgUrl.value = await loadImage(path);
  } else {
    sidebarBgUrl.value = "";
  }
});

const sidebarBgStyle = () => {
  if (sidebarBgUrl.value) {
    return {
      "--sidebar-bg-image": `url(${sidebarBgUrl.value})`,
      "--sidebar-blur": `${parseInt(store.settings.sidebar_blur) || 0}px`,
      "--sidebar-brightness": String((parseInt(store.settings.sidebar_brightness) || 100) / 100),
    };
  }
  return {};
};

function tagGameCount(tagId: number): number {
  let count = 0;
  for (const [, tlist] of store.gameTags) {
    if (tlist.some((t) => t.id === tagId)) count++;
  }
  return count;
}

function tagModCount(tagId: number): number {
  let count = 0;
  for (const [, tlist] of modStore.modTags) {
    if (tlist.some((t) => t.id === tagId)) count++;
  }
  return count;
}

const statusOptions = [
  { id: "not_played", icon: "🆕", nameKey: "game.notPlayed" },
  { id: "playing", icon: "▶️", nameKey: "game.playing" },
  { id: "completed", icon: "✅", nameKey: "game.completed" },
  { id: "shelved", icon: "📌", nameKey: "game.shelved" },
];

function statusCount(statusId: string): number {
  return store.games.filter((g) => g.status === statusId).length;
}

// Games that have mods (for sidebar game groups)
const gamesWithMods = computed(() => {
  return store.games
    .map(g => ({ game: g, count: modStore.mods.filter(m => m.game_id === g.id).length }))
    .filter(item => item.count > 0);
});

const independentModCount = computed(() => modStore.mods.filter(m => m.game_id === null).length);

// 最近在玩：有游玩记录的游戏按最后游玩时间倒序，取前 4 个作快捷入口
const recentGames = computed(() =>
  store.games
    .filter((g) => g.last_played_at)
    .sort((a, b) => (b.last_played_at ?? "").localeCompare(a.last_played_at ?? ""))
    .slice(0, 4)
);
</script>

<template>
  <aside
    class="sidebar-bg w-72 h-full flex flex-col shrink-0 relative overflow-hidden"
    style="background: var(--color-sidebar-bg)"
    :style="sidebarBgStyle()"
  >
    <!-- Header -->
    <div class="px-7 pt-7 pb-4 border-b border-border-light">
      <h1 class="text-xl font-bold flex items-center gap-3">
        <img src="/app-icon.png" alt="icon" class="w-8 h-8 rounded-lg" />
        <span class="bg-gradient-to-r from-primary-500 to-sakura-400 bg-clip-text text-transparent">
          花譜
        </span>
      </h1>
    </div>

    <!-- View Switcher -->
    <div class="px-5 pt-4 pb-2">
      <div class="flex items-center rounded-xl bg-input-bg/50 p-1">
        <button
          class="flex-1 flex items-center justify-center gap-2 px-3 py-2.5 rounded-lg text-sm font-semibold transition-all"
          :class="props.currentView === 'games' ? 'bg-primary-500 text-white shadow-sm' : 'text-text-sub hover:text-text-main hover:bg-overlay-white'"
          @click="emit('switchView', 'games')"
        >
          <span class="text-base">📚</span>
          {{ t('app.gamesLabel') }}
        </button>
        <button
          class="flex-1 flex items-center justify-center gap-2 px-3 py-2.5 rounded-lg text-sm font-semibold transition-all"
          :class="props.currentView === 'mods' ? 'bg-primary-500 text-white shadow-sm' : 'text-text-sub hover:text-text-main hover:bg-overlay-white'"
          @click="emit('switchView', 'mods')"
        >
          <span class="text-base">🧩</span>
          {{ t('app.modsLabel') }}
          <span v-if="modStore.mods.length > 0" class="text-[10px] opacity-70">{{ modStore.mods.length }}</span>
        </button>
      </div>
    </div>

    <!-- Navigation -->
    <nav class="flex-1 overflow-auto px-4 py-6 space-y-2">
      <!-- ========== Games View: Directory structure ========== -->
      <template v-if="props.currentView === 'games'">
        <!-- All Games -->
        <div
          class="flex items-center px-4 py-2.5 rounded-xl cursor-pointer transition-all duration-200"
          :class="
            store.selectedGroupId === null
              ? 'bg-sidebar-active text-primary-700 font-medium shadow-sm'
              : 'hover:bg-sidebar-hover text-text-main/70'
          "
          @click="selectGroup(null)"
        >
          <span class="mr-3 text-base">📚</span>
          <span class="flex-1 text-sm">{{ t('group.allGames') }}</span>
          <span class="text-xs bg-primary-100 text-primary-600 px-2 py-0.5 rounded-lg">{{
            store.games.length
          }}</span>
        </div>

        <!-- Recently Played -->
        <div v-if="recentGames.length > 0" class="pt-4 mt-2 border-t border-border-light">
          <p class="px-4 text-[11px] text-text-sub font-medium uppercase tracking-wider mb-2">{{ t('group.recent') }}</p>
          <div
            v-for="g in recentGames"
            :key="g.id"
            class="flex items-center px-4 py-2 rounded-xl cursor-pointer transition-all duration-200"
            :class="
              store.selectedGameId === g.id
                ? 'bg-sidebar-active text-primary-700 font-medium'
                : 'hover:bg-sidebar-hover text-text-main/70'
            "
            :title="g.name"
            @click="store.selectedGameId = g.id"
          >
            <span class="mr-2 text-sm">🕒</span>
            <span class="flex-1 truncate text-[13px]">{{ g.name }}</span>
          </div>
        </div>

        <!-- Groups -->
        <div>
          <div
            v-for="group in store.groups"
            :key="group.id"
            draggable="true"
            class="group flex items-center px-4 py-2.5 rounded-xl cursor-pointer transition-all duration-200"
            :class="[
              store.selectedGroupId === group.id
                ? 'bg-sidebar-active text-primary-700 font-medium shadow-sm'
                : 'hover:bg-sidebar-hover text-text-main/70',
              dragOverGroupId === group.id ? 'ring-2 ring-primary-400 ring-inset' : ''
            ]"
            @click="selectGroup(group.id)"
            @dragstart="onGroupDragStart(group, $event)"
            @dragover="onGroupDragOver(group, $event)"
            @dragleave="onGroupDragLeave"
            @drop="onGroupDrop(group, $event)"
          >
            <span class="mr-3 text-base">📁</span>
            <template v-if="editingGroupId === group.id">
              <input
                v-model="editingName"
                class="flex-1 bg-transparent border-b border-primary-300/50 outline-none text-sm min-w-0 text-text-main"
                @keyup.enter="confirmRename(group.id)"
                @keyup.escape="cancelRename"
                @blur="confirmRename(group.id)"
                @click.stop
              />
            </template>
            <template v-else>
              <span class="flex-1 truncate text-sm">{{ group.name }}</span>
              <span class="text-xs bg-primary-100 text-primary-600 px-1.5 py-0.5 rounded-md mr-1">{{
                store.games.filter((g) => g.group_id === group.id).length
              }}</span>
              <div class="hidden group-hover:flex items-center gap-0.5">
                <button
                  class="p-1 rounded-md hover:bg-sidebar-btn-hover text-xs leading-none"
                  @click.stop="startRename(group)"
                  :title="t('group.rename')"
                >
                  ✏️
                </button>
                <button
                  class="p-1 rounded-md hover:bg-red-500/20 text-xs leading-none"
                  @click.stop="requestDeleteGroup(group)"
                  :title="t('group.delete')"
                >
                  🗑️
                </button>
              </div>
            </template>
          </div>
        </div>

        <!-- Add new group (inline) -->
        <div class="flex items-center gap-1 px-2 mt-1">
          <input
            v-model="newGroupName"
            :placeholder="t('group.newGroup')"
            class="flex-1 px-3 py-2 text-xs rounded-lg bg-sidebar-input border border-border-light text-text-main placeholder-text-sub/50 outline-none focus:border-primary-400 transition-colors min-w-0"
            @keyup.enter="createGroup"
          />
          <button
            class="px-2.5 py-2 text-xs rounded-lg bg-sidebar-btn hover:bg-sidebar-btn-hover text-text-main transition-colors shrink-0"
            @click="createGroup"
          >
            +
          </button>
        </div>

        <!-- Status filter -->
        <div class="pt-4 mt-2 border-t border-border-light">
          <p class="px-4 text-[11px] text-text-sub font-medium uppercase tracking-wider mb-2">{{ t('group.status') }}</p>
          <div
            class="flex items-center px-4 py-2 rounded-xl cursor-pointer transition-all duration-200"
            :class="
              store.selectedStatus === null
                ? 'bg-sidebar-active text-primary-700 font-medium'
                : 'hover:bg-sidebar-hover text-text-main/70'
            "
            @click="store.selectedStatus = null"
          >
            <span class="mr-2 text-sm">📊</span>
            <span class="flex-1 text-[13px]">{{ t('group.allStatus') }}</span>
            <span class="text-[11px] text-text-sub/60">{{ store.games.length }}</span>
          </div>
          <div
            v-for="s in statusOptions"
            :key="s.id"
            class="flex items-center px-4 py-2 rounded-xl cursor-pointer transition-all duration-200"
            :class="
              store.selectedStatus === s.id
                ? 'bg-sidebar-active text-primary-700 font-medium'
                : 'hover:bg-sidebar-hover text-text-main/70'
            "
            @click="store.selectedStatus = store.selectedStatus === s.id ? null : s.id"
          >
            <span class="mr-2 text-sm">{{ s.icon }}</span>
            <span class="flex-1 truncate text-[13px]">{{ t(s.nameKey) }}</span>
            <span class="text-[11px] text-text-sub/60">{{ statusCount(s.id) }}</span>
          </div>
        </div>

        <!-- Tags filter -->
        <div v-if="store.tags.length > 0" class="pt-4 mt-2 border-t border-border-light">
          <p class="px-4 text-[11px] text-text-sub font-medium uppercase tracking-wider mb-2">{{ t('group.tags') }}</p>
          <div
            class="flex items-center px-4 py-2 rounded-xl cursor-pointer transition-all duration-200"
            :class="
              store.selectedTagId === null
                ? 'bg-sidebar-active text-primary-700 font-medium'
                : 'hover:bg-sidebar-hover text-text-main/70'
            "
            @click="store.selectedTagId = null"
          >
            <span class="mr-2 text-sm">🏷️</span>
            <span class="flex-1 text-[13px]">{{ t('group.allTags') }}</span>
            <span class="text-[11px] text-text-sub/60">{{ store.games.length }}</span>
          </div>
          <div
            v-for="tag in store.tags"
            :key="tag.id"
            class="flex items-center px-4 py-2 rounded-xl cursor-pointer transition-all duration-200"
            :class="
              store.selectedTagId === tag.id
                ? 'bg-sidebar-active text-primary-700 font-medium'
                : 'hover:bg-sidebar-hover text-text-main/70'
            "
            @click="store.selectedTagId = store.selectedTagId === tag.id ? null : tag.id"
          >
            <span class="mr-2 text-sm">#</span>
            <span class="flex-1 truncate text-[13px]">{{ tag.name }}</span>
            <span class="text-[11px] text-text-sub/60">{{ tagGameCount(tag.id) }}</span>
          </div>
        </div>
      </template>

      <!-- ========== Mods View: Clean mod summary ========== -->
      <template v-else>
        <!-- All Mods -->
        <div
          class="flex items-center px-4 py-2.5 rounded-xl cursor-pointer transition-all duration-200"
          :class="
            modStore.modFilterEnabled === null
              ? 'bg-sidebar-active text-primary-700 font-medium shadow-sm'
              : 'hover:bg-sidebar-hover text-text-main/70'
          "
          @click="modStore.modFilterEnabled = null"
        >
          <span class="mr-3 text-base">🧩</span>
          <span class="flex-1 text-sm">{{ t('mod.allMods') }}</span>
          <span class="text-xs bg-primary-100 text-primary-600 px-2 py-0.5 rounded-lg">{{ modStore.mods.length }}</span>
        </div>

        <!-- Enabled / Disabled -->
        <div class="space-y-1.5 mt-1">
          <div
            class="flex items-center justify-between text-xs px-4 py-2 rounded-xl cursor-pointer transition-all duration-200"
            :class="modStore.modFilterEnabled === 'enabled' ? 'bg-sidebar-active text-green-700 dark:text-green-400 font-medium' : 'hover:bg-sidebar-hover text-text-main/70'"
            @click="modStore.modFilterEnabled = modStore.modFilterEnabled === 'enabled' ? null : 'enabled'"
          >
            <span>{{ t('mod.enabled') }}</span>
            <span class="text-green-600 dark:text-green-400 font-medium">{{ modStore.mods.filter(m => m.is_enabled).length }}</span>
          </div>
          <div
            class="flex items-center justify-between text-xs px-4 py-2 rounded-xl cursor-pointer transition-all duration-200"
            :class="modStore.modFilterEnabled === 'disabled' ? 'bg-sidebar-active text-red-600 dark:text-red-400 font-medium' : 'hover:bg-sidebar-hover text-text-main/70'"
            @click="modStore.modFilterEnabled = modStore.modFilterEnabled === 'disabled' ? null : 'disabled'"
          >
            <span>{{ t('mod.disabled') }}</span>
            <span class="text-red-500 dark:text-red-400 font-medium">{{ modStore.mods.filter(m => !m.is_enabled).length }}</span>
          </div>
          <div
            class="flex items-center justify-between text-xs px-4 py-2 rounded-xl cursor-pointer transition-all duration-200"
            :class="modStore.modFilterMissing ? 'bg-sidebar-active text-amber-600 dark:text-amber-400 font-medium' : 'hover:bg-sidebar-hover text-text-main/70'"
            @click="modStore.modFilterMissing = !modStore.modFilterMissing"
          >
            <span>{{ t('mod.missingFile') }}</span>
            <span class="text-amber-600 dark:text-amber-400 font-medium">{{ modStore.modMissingIds.size }}</span>
          </div>
        </div>

        <!-- Game groups -->
        <div class="pt-4 mt-2 border-t border-border-light">
          <p class="px-4 text-[11px] text-text-sub font-medium uppercase tracking-wider mb-2">{{ t('mod.gameGroups') }}</p>
          <div
            class="flex items-center px-4 py-2 rounded-xl cursor-pointer transition-all duration-200"
            :class="
              modStore.modFilterGameId === null
                ? 'bg-sidebar-active text-primary-700 font-medium'
                : 'hover:bg-sidebar-hover text-text-main/70'
            "
            @click="modStore.modFilterGameId = null"
          >
            <span class="mr-2 text-sm">🧩</span>
            <span class="flex-1 text-[13px]">{{ t('mod.allGames') }}</span>
            <span class="text-[11px] text-text-sub/60">{{ modStore.mods.length }}</span>
          </div>
          <div
            v-for="item in gamesWithMods"
            :key="item.game.id"
            class="flex items-center px-4 py-2 rounded-xl cursor-pointer transition-all duration-200"
            :class="
              modStore.modFilterGameId === item.game.id
                ? 'bg-sidebar-active text-primary-700 font-medium'
                : 'hover:bg-sidebar-hover text-text-main/70'
            "
            @click="modStore.modFilterGameId = modStore.modFilterGameId === item.game.id ? null : item.game.id"
          >
            <span class="mr-2 text-sm">🎮</span>
            <span class="flex-1 truncate text-[13px]">{{ item.game.name }}</span>
            <span class="text-[11px] text-text-sub/60">{{ item.count }}</span>
          </div>
          <div
            v-if="independentModCount > 0"
            class="flex items-center px-4 py-2 rounded-xl cursor-pointer transition-all duration-200"
            :class="
              modStore.modFilterGameId === -1
                ? 'bg-sidebar-active text-primary-700 font-medium'
                : 'hover:bg-sidebar-hover text-text-main/70'
            "
            @click="modStore.modFilterGameId = modStore.modFilterGameId === -1 ? null : -1"
          >
            <span class="mr-2 text-sm">📦</span>
            <span class="flex-1 truncate text-[13px]">{{ t('mod.independent') }}</span>
            <span class="text-[11px] text-text-sub/60">{{ independentModCount }}</span>
          </div>
        </div>

        <!-- Tags filter -->
        <div v-if="store.tags.length > 0" class="pt-4 mt-2 border-t border-border-light">
          <p class="px-4 text-[11px] text-text-sub font-medium uppercase tracking-wider mb-2">{{ t('group.tags') }}</p>
          <div
            class="flex items-center px-4 py-2 rounded-xl cursor-pointer transition-all duration-200"
            :class="
              modStore.modFilterTagId === null
                ? 'bg-sidebar-active text-primary-700 font-medium'
                : 'hover:bg-sidebar-hover text-text-main/70'
            "
            @click="modStore.modFilterTagId = null"
          >
            <span class="mr-2 text-sm">🏷️</span>
            <span class="flex-1 text-[13px]">{{ t('group.allTags') }}</span>
            <span class="text-[11px] text-text-sub/60">{{ modStore.mods.length }}</span>
          </div>
          <div
            v-for="tag in store.tags"
            :key="tag.id"
            class="flex items-center px-4 py-2 rounded-xl cursor-pointer transition-all duration-200"
            :class="
              modStore.modFilterTagId === tag.id
                ? 'bg-sidebar-active text-primary-700 font-medium'
                : 'hover:bg-sidebar-hover text-text-main/70'
            "
            @click="modStore.modFilterTagId = modStore.modFilterTagId === tag.id ? null : tag.id"
          >
            <span class="mr-2 text-sm">#</span>
            <span class="flex-1 truncate text-[13px]">{{ tag.name }}</span>
            <span class="text-[11px] text-text-sub/60">{{ tagModCount(tag.id) }}</span>
          </div>
        </div>
      </template>
    </nav>

    <!-- Footer Actions -->
    <div class="px-5 py-6 border-t border-border-light flex gap-2">
      <button
        class="flex-1 py-2.5 text-sm rounded-xl bg-sidebar-btn text-text-sub hover:bg-sidebar-btn-hover hover:text-text-main transition-colors"
        @click="emit('settings')"
      >
        ⚙️ {{ t('settings.title') }}
      </button>
      <button
        class="px-3.5 py-2.5 text-sm rounded-xl bg-sidebar-btn text-text-sub hover:bg-sidebar-btn-hover hover:text-text-main transition-colors"
        :title="t('about.title')"
        @click="emit('about')"
      >
        ℹ️
      </button>
    </div>
  </aside>
  <transition name="modal">
    <ConfirmDialog
      v-if="deletingGroupId !== null"
      :title="t('group.deleteTitle')"
      :message="t('group.confirmDelete', { name: deletingGroupName })"
      :confirm-text="t('common.delete')"
      :danger="true"
      @confirm="confirmDeleteGroup"
      @cancel="deletingGroupId = null"
    />
  </transition>
</template>

<style scoped>
/* 侧边栏自定义背景图（支持模糊度设置） */
.sidebar-bg::before {
  content: "";
  position: absolute;
  inset: 0;
  z-index: 0;
  background-image: var(--sidebar-bg-image);
  background-size: cover;
  background-position: center;
  filter: blur(var(--sidebar-blur, 0px)) brightness(var(--sidebar-brightness, 1));
  transform: scale(1.06);
}
.sidebar-bg > * {
  position: relative;
  z-index: 1;
}
</style>
