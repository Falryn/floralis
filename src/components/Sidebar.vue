<script setup lang="ts">
import { ref, watchEffect } from "vue";
import { useGameStore, loadImage } from "../stores/gameStore";
import { useI18n } from "vue-i18n";
import type { Group } from "../types";
import ConfirmDialog from "./ConfirmDialog.vue";

const { t } = useI18n();

const emit = defineEmits<{
  import: [];
  settings: [];
  stats: [];
}>();

const store = useGameStore();
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
      backgroundImage: `url(${sidebarBgUrl.value})`,
      backgroundSize: "cover",
      backgroundPosition: "center",
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

const statusOptions = [
  { id: "not_played", icon: "🆕", nameKey: "game.notPlayed" },
  { id: "playing", icon: "▶️", nameKey: "game.playing" },
  { id: "completed", icon: "✅", nameKey: "game.completed" },
  { id: "shelved", icon: "📌", nameKey: "game.shelved" },
];

function statusCount(statusId: string): number {
  return store.games.filter((g) => g.status === statusId).length;
}
</script>

<template>
  <aside
    class="w-72 h-full flex flex-col shrink-0 backdrop-blur-xl"
    style="background: var(--color-sidebar-bg)"
    :style="sidebarBgStyle()"
  >
    <!-- Header -->
    <div class="px-7 py-7 border-b border-border-light">
      <h1 class="text-xl font-bold flex items-center gap-3">
        <img src="/app-icon.png" alt="icon" class="w-8 h-8 rounded-lg" />
        <span class="bg-gradient-to-r from-primary-500 to-sakura-400 bg-clip-text text-transparent">
          花譜
        </span>
      </h1>
    </div>

    <!-- Navigation -->
    <nav class="flex-1 overflow-auto px-4 py-6 space-y-2">
      <!-- All Games -->
      <div
        class="flex items-center px-5 py-3.5 rounded-2xl cursor-pointer transition-all duration-200"
        :class="
          store.selectedGroupId === null
            ? 'bg-sidebar-active text-primary-700 font-medium shadow-sm'
            : 'hover:bg-sidebar-hover text-text-main/70'
        "
        @click="selectGroup(null)"
      >
        <span class="mr-3 text-lg">📚</span>
        <span class="flex-1 text-sm">{{ t('group.allGames') }}</span>
        <span class="text-xs bg-primary-100 text-primary-600 px-2 py-0.5 rounded-lg">{{
          store.games.length
        }}</span>
      </div>

      <!-- Groups -->
      <div
        v-for="group in store.groups"
        :key="group.id"
        draggable="true"
        class="group flex items-center px-5 py-3 rounded-2xl cursor-pointer transition-all duration-200"
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
        <span class="mr-3 text-lg">📁</span>
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
              title="{{ t('group.rename') }}"
            >
              ✏️
            </button>
            <button
              class="p-1 rounded-md hover:bg-red-500/20 text-xs leading-none"
              @click.stop="requestDeleteGroup(group)"
              title="{{ t('group.delete') }}"
            >
              🗑️
            </button>
          </div>
        </template>
      </div>

      <!-- Status filter -->
      <div class="pt-4 mt-2 border-t border-border-light">
        <p class="px-5 text-[10px] text-text-sub font-medium uppercase tracking-wider mb-2">{{ t('group.status') }}</p>
        <div
          class="flex items-center px-5 py-2 rounded-2xl cursor-pointer transition-all duration-200"
          :class="
            store.selectedStatus === null
              ? 'bg-sidebar-active text-primary-700 font-medium'
              : 'hover:bg-sidebar-hover text-text-main/70'
          "
          @click="store.selectedStatus = null"
        >
          <span class="mr-2 text-sm">📊</span>
          <span class="flex-1 text-xs">{{ t('group.allStatus') }}</span>
          <span class="text-[10px] text-text-sub/60">{{ store.games.length }}</span>
        </div>
        <div
          v-for="s in statusOptions"
          :key="s.id"
          class="flex items-center px-5 py-2 rounded-2xl cursor-pointer transition-all duration-200"
          :class="
            store.selectedStatus === s.id
              ? 'bg-sidebar-active text-primary-700 font-medium'
              : 'hover:bg-sidebar-hover text-text-main/70'
          "
          @click="store.selectedStatus = store.selectedStatus === s.id ? null : s.id"
        >
          <span class="mr-2 text-sm">{{ s.icon }}</span>
          <span class="flex-1 truncate text-xs">{{ t(s.nameKey) }}</span>
          <span class="text-[10px] text-text-sub/60">{{ statusCount(s.id) }}</span>
        </div>
      </div>

      <!-- Tags filter -->
      <div v-if="store.tags.length > 0" class="pt-4 mt-2 border-t border-border-light">
        <p class="px-5 text-[10px] text-text-sub font-medium uppercase tracking-wider mb-2">{{ t('group.tags') }}</p>
        <div
          class="flex items-center px-5 py-2 rounded-2xl cursor-pointer transition-all duration-200"
          :class="
            store.selectedTagId === null
              ? 'bg-sidebar-active text-primary-700 font-medium'
              : 'hover:bg-sidebar-hover text-text-main/70'
          "
          @click="store.selectedTagId = null"
        >
          <span class="mr-2 text-sm">🏷️</span>
          <span class="flex-1 text-xs">{{ t('group.allTags') }}</span>
          <span class="text-[10px] text-text-sub/60">{{ store.games.length }}</span>
        </div>
        <div
          v-for="tag in store.tags"
          :key="tag.id"
          class="flex items-center px-5 py-2 rounded-2xl cursor-pointer transition-all duration-200"
          :class="
            store.selectedTagId === tag.id
              ? 'bg-sidebar-active text-primary-700 font-medium'
              : 'hover:bg-sidebar-hover text-text-main/70'
          "
          @click="store.selectedTagId = store.selectedTagId === tag.id ? null : tag.id"
        >
          <span class="mr-2 text-sm">#</span>
          <span class="flex-1 truncate text-xs">{{ tag.name }}</span>
          <span class="text-[10px] text-text-sub/60">{{ tagGameCount(tag.id) }}</span>
        </div>
      </div>
    </nav>

    <!-- Footer Actions -->
    <div class="px-5 py-6 border-t border-border-light space-y-3">
      <div class="flex gap-2">
        <input
          v-model="newGroupName"
          :placeholder="t('group.newGroup')"
          class="flex-1 px-4 py-2.5 text-sm rounded-xl bg-sidebar-input border border-border-light text-text-main placeholder-text-sub/50 outline-none focus:border-primary-400 transition-colors min-w-0"
          @keyup.enter="createGroup"
        />
        <button
          class="px-3 py-2.5 text-sm rounded-xl bg-sidebar-btn hover:bg-sidebar-btn-hover text-text-main transition-colors shrink-0"
          @click="createGroup"
        >
          +
        </button>
      </div>

      <button
        class="w-full py-3 text-sm rounded-xl bg-gradient-to-r from-primary-500 to-sakura-400 text-white font-medium hover:opacity-90 transition-all shadow-lg shadow-primary-500/20"
        @click="emit('import')"
      >
        ✨ {{ t('import.title') }}
      </button>

      <button
        class="w-full py-2.5 text-sm rounded-xl bg-sidebar-btn text-text-sub hover:bg-sidebar-btn-hover hover:text-text-main transition-colors"
        @click="emit('stats')"
      >
        📊 {{ t('stats.title') }}
      </button>

      <button
        class="w-full py-2.5 text-sm rounded-xl bg-sidebar-btn text-text-sub hover:bg-sidebar-btn-hover hover:text-text-main transition-colors"
        @click="emit('settings')"
      >
        ⚙️ {{ t('settings.title') }}
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
