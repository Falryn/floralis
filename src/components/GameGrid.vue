<script setup lang="ts">
import { ref, watchEffect } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useGameStore, loadImage } from "../stores/gameStore";
import { useI18n } from "vue-i18n";
import type { Game } from "../types";

const { t } = useI18n();

const props = defineProps<{
  games: Game[];
  viewMode?: "grid" | "list";
}>();

const emit = defineEmits<{
  select: [id: number];
  edit: [id: number];
  launch: [id: number];
  contextmenu: [id: number, x: number, y: number];
}>();

const store = useGameStore();

const coverUrls = ref<Map<number, string>>(new Map());
const emptyImgUrl = ref("");
const lastClickedIndex = ref<number | null>(null);
const hoveredGameId = ref<number | null>(null);

let clickTimer: ReturnType<typeof setTimeout> | null = null;

function handleClick(game: Game, index: number, e: MouseEvent | TouchEvent) {
  // Multi-select mode
  if (store.isSelectMode) {
    // Shift+click for range select
    if ((e as MouseEvent).shiftKey && lastClickedIndex.value !== null) {
      const start = Math.min(lastClickedIndex.value, index);
      const end = Math.max(lastClickedIndex.value, index);
      for (let i = start; i <= end; i++) {
        const g = props.games[i];
        if (!store.selectedGameIds.has(g.id)) {
          store.toggleSelectGame(g.id);
        }
      }
    } else if ((e as MouseEvent).ctrlKey || (e as MouseEvent).metaKey) {
      store.toggleSelectGame(game.id);
    } else {
      store.toggleSelectGame(game.id);
    }
    lastClickedIndex.value = index;
    return;
  }

  if (clickTimer) {
    clearTimeout(clickTimer);
    clickTimer = null;
    store.launchGame(game.id);
  } else {
    clickTimer = setTimeout(() => {
      clickTimer = null;
      emit("select", game.id);
    }, 250);
  }
}

function handleCheckboxClick(game: Game, e: MouseEvent) {
  e.stopPropagation();
  store.toggleSelectGame(game.id);
}

function isSelected(game: Game): boolean {
  return store.selectedGameIds.has(game.id);
}

function handleContextMenu(game: Game, e: MouseEvent) {
  e.preventDefault();
  emit("contextmenu", game.id, e.clientX, e.clientY);
}

// 记录已处理的游戏ID，避免重复加载
const loadedGameIds = new Set<number>();

watchEffect(async () => {
  // Load empty illustration
  if (store.settings.custom_empty_illustration) {
    emptyImgUrl.value = await loadImage(store.settings.custom_empty_illustration);
  } else {
    emptyImgUrl.value = "";
  }
  // 找出需要加载封面的游戏（未加载过的）
  const pending = store.games.filter(
    (g) => g.cover_path && !coverUrls.value.has(g.id) && !loadedGameIds.has(g.id)
  );
  if (pending.length === 0) return;
  // 标记为已处理，防止重复触发
  for (const g of pending) loadedGameIds.add(g.id);
  // 并行加载所有封面
  await Promise.all(
    pending.map(async (game) => {
      try {
        const thumbPath = await invoke<string>("generate_thumbnail", {
          sourcePath: game.cover_path,
          gameId: game.id,
        });
        const url = await loadImage(thumbPath);
        if (url) {
          coverUrls.value.set(game.id, url);
          return;
        }
      } catch (_) {
        // Thumbnail generation failed, fall back to original
      }
      const url = await loadImage(game.cover_path);
      if (url) coverUrls.value.set(game.id, url);
    })
  );
});

function coverUrl(game: Game): string {
  return coverUrls.value.get(game.id) ?? "";
}

function startDrag(game: Game, e: DragEvent) {
  e.dataTransfer?.setData("text/plain", String(game.id));
  e.dataTransfer!.effectAllowed = "move";
}

function onDrop(groupId: number | null, e: DragEvent) {
  e.preventDefault();
  const gameId = Number(e.dataTransfer?.getData("text/plain"));
  if (gameId) {
    store.setGameGroup(gameId, groupId);
  }
}

// Drag reorder
const dragOverGameId = ref<number | null>(null);

function onGameDragOver(targetGame: Game, e: DragEvent) {
  e.preventDefault();
  e.stopPropagation();
  dragOverGameId.value = targetGame.id;
}

function onGameDrop(targetGame: Game, e: DragEvent) {
  e.preventDefault();
  e.stopPropagation();
  dragOverGameId.value = null;
  const draggedId = Number(e.dataTransfer?.getData("text/plain"));
  if (!draggedId || draggedId === targetGame.id) return;
  // Build new order: move dragged game to target position
  const currentIds = props.games.map(g => g.id);
  const dragIdx = currentIds.indexOf(draggedId);
  const targetIdx = currentIds.indexOf(targetGame.id);
  if (dragIdx === -1 || targetIdx === -1) return;
  currentIds.splice(dragIdx, 1);
  currentIds.splice(targetIdx, 0, draggedId);
  store.reorderGames(currentIds);
}

function onGameDragLeave() {
  dragOverGameId.value = null;
}

function formatPlayTime(seconds: number): string {
  if (seconds < 60) return t('game.seconds', { n: seconds });
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 0) return t('game.hoursMinutesShort', { h: hours, m: minutes });
  return t('game.minutesShort', { m: minutes });
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return t('game.never');
  return dateStr.slice(0, 10);
}

function statusLabel(status: string): string {
  const labels: Record<string, string> = {
    not_played: t('game.notPlayed'),
    playing: t('game.playing'),
    completed: t('game.completed'),
    shelved: t('game.shelved'),
  };
  return labels[status] || status;
}

function highlightName(name: string): string {
  const kw = store.searchKeyword.trim();
  if (!kw) return escapeHtml(name);
  const escaped = escapeHtml(name);
  const escapedKw = escapeHtml(kw);
  const regex = new RegExp(`(${escapedKw.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")})`, "gi");
  return escaped.replace(regex, '<mark class="bg-yellow-200/70 text-inherit rounded-sm px-0.5">$1</mark>');
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}
</script>

<template>
  <div>
    <!-- Empty State -->
    <div
      v-if="games.length === 0"
      class="flex flex-col items-center justify-center h-[60vh] text-text-sub"
    >
      <img
        v-if="emptyImgUrl"
        :src="emptyImgUrl"
        class="w-56 h-56 object-contain mb-8 opacity-40 rounded-3xl"
        alt="empty"
      />
      <div v-else class="text-8xl mb-8 opacity-20">🎮</div>
      <template v-if="store.games.length === 0">
        <p class="text-xl font-medium">{{ t('game.noGamesYet') }}</p>
        <p class="text-sm mt-3 opacity-60">{{ t('game.importHint') }}</p>
      </template>
      <template v-else>
        <p class="text-xl font-medium">{{ t('game.noMatch') }}</p>
        <p class="text-sm mt-3 opacity-60">{{ t('game.adjustFilter') }}</p>
      </template>
    </div>

    <!-- Game Grid -->
    <div
      v-else
      :class="props.viewMode === 'list'
        ? 'flex flex-col gap-3'
        : 'grid grid-cols-2 @sm:grid-cols-3 @md:grid-cols-4 @xl:grid-cols-5 gap-6'"
    >
      <!-- Grid View -->
      <template v-if="props.viewMode !== 'list'">
      <div
        v-for="(game, index) in games"
        :key="game.id"
        draggable="true"
        class="group relative rounded-3xl overflow-hidden bg-card shadow-md hover:shadow-2xl transition-all duration-300 cursor-pointer hover:-translate-y-2"
        :class="[{ 'ring-3 ring-primary-400 ring-offset-2 ring-offset-transparent': isSelected(game) }, dragOverGameId === game.id ? 'opacity-50 scale-95' : '']"
        @click="handleClick(game, index, $event)"
        @contextmenu="handleContextMenu(game, $event)"
        @dragstart="startDrag(game, $event)"
        @dragover="onGameDragOver(game, $event)"
        @drop="onGameDrop(game, $event)"
        @dragleave="onGameDragLeave"
        @mouseenter="hoveredGameId = game.id"
        @mouseleave="hoveredGameId = null"
      >
        <!-- Selection checkbox -->
        <div
          v-if="store.isSelectMode"
          class="absolute top-3 left-3 z-10"
          @click="handleCheckboxClick(game, $event)"
        >
          <div
            class="w-6 h-6 rounded-lg border-2 flex items-center justify-center transition-all"
            :class="isSelected(game) ? 'bg-primary-500 border-primary-500' : 'bg-white/70 border-white/50'"
          >
            <svg v-if="isSelected(game)" class="w-4 h-4 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
          </div>
        </div>
        <!-- Status badge -->
        <div
          v-if="game.status && game.status !== 'not_played'"
          class="absolute top-3 right-3 z-10 px-2 py-0.5 rounded-lg text-[10px] font-medium backdrop-blur-sm"
          :class="{
            'bg-green-500/80 text-white': game.status === 'playing',
            'bg-blue-500/80 text-white': game.status === 'completed',
            'bg-yellow-500/80 text-white': game.status === 'shelved',
          }"
        >
          {{ statusLabel(game.status) }}
        </div>
        <div class="aspect-[3/4] bg-primary-50 relative overflow-hidden">
          <img
            v-if="coverUrl(game)"
            :src="coverUrl(game)"
            :alt="game.name"
            class="w-full h-full object-cover"
            loading="lazy"
            @error="coverUrls.delete(game.id)"
          />
          <div
            v-else
            class="w-full h-full flex items-center justify-center text-6xl text-primary-200 bg-gradient-to-br from-primary-50 to-sakura-50"
          >
            🎮
          </div>

          <!-- Bottom gradient + title -->
          <div
            class="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/80 via-black/40 to-transparent p-4 pt-14"
          >
            <h3 class="text-white text-sm font-semibold leading-snug drop-shadow-md" :class="hoveredGameId === game.id ? '' : 'line-clamp-2'" v-html="highlightName(game.name)" />
            <!-- Tag badges -->
            <div v-if="store.gameTags.get(game.id)?.length" class="flex flex-wrap gap-1 mt-1.5">
              <span
                v-for="tag in store.gameTags.get(game.id)!.slice(0, 3)"
                :key="tag.id"
                class="px-1.5 py-0.5 rounded-md bg-white/20 text-white/80 text-[10px] backdrop-blur-sm"
              >
                {{ tag.name }}
              </span>
              <span
                v-if="store.gameTags.get(game.id)!.length > 3"
                class="px-1.5 py-0.5 rounded-md bg-white/20 text-white/80 text-[10px] backdrop-blur-sm"
              >
                +{{ store.gameTags.get(game.id)!.length - 3 }}
              </span>
            </div>
          </div>
        </div>
      </div>
      </template>

      <!-- List View (virtualized) -->
      <template v-else>
      <RecycleScroller
        :items="games"
        :item-height="72"
        key-field="id"
        class="virtual-list"
        v-slot="{ item: game }"
      >
        <div
          draggable="true"
          class="group flex items-center gap-4 px-4 py-3 rounded-2xl bg-card shadow-sm hover:shadow-md transition-all duration-200 cursor-pointer"
          :class="[{ 'ring-2 ring-primary-400': isSelected(game) }]"
          @click="handleClick(game, games.indexOf(game), $event)"
          @contextmenu="handleContextMenu(game, $event)"
          @dragstart="startDrag(game, $event)"
          @dragover="onGameDragOver(game, $event)"
          @drop="onGameDrop(game, $event)"
          @dragleave="onGameDragLeave"
        >
          <!-- Selection checkbox -->
          <div
            v-if="store.isSelectMode"
            class="shrink-0"
            @click="handleCheckboxClick(game, $event)"
          >
            <div
              class="w-5 h-5 rounded-md border-2 flex items-center justify-center transition-all"
              :class="isSelected(game) ? 'bg-primary-500 border-primary-500' : 'bg-input-bg border-border-medium'"
            >
              <svg v-if="isSelected(game)" class="w-3 h-3 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                <polyline points="20 6 9 17 4 12"/>
              </svg>
            </div>
          </div>
          <!-- Cover thumbnail -->
          <div class="w-12 h-12 rounded-xl overflow-hidden bg-primary-50 shrink-0">
            <img v-if="coverUrl(game)" :src="coverUrl(game)" class="w-full h-full object-cover" @error="coverUrls.delete(game.id)" />
            <div v-else class="w-full h-full flex items-center justify-center text-xl text-primary-200">🎮</div>
          </div>
          <!-- Info -->
          <div class="flex-1 min-w-0">
            <h3 class="text-sm font-medium text-text-main truncate" v-html="highlightName(game.name)" />
            <div class="flex items-center gap-2 mt-0.5">
              <span class="text-xs text-text-sub">{{ store.groups.find(g => g.id === game.group_id)?.name ?? t('game.ungrouped') }}</span>
              <span v-if="store.gameTags.get(game.id)?.length" class="flex gap-1">
                <span
                  v-for="tag in store.gameTags.get(game.id)!.slice(0, 3)"
                  :key="tag.id"
                  class="px-1.5 py-0 rounded-md bg-primary-50 text-primary-500 text-[10px]"
                >{{ tag.name }}</span>
              </span>
            </div>
          </div>
          <!-- Play time -->
          <div class="text-right shrink-0">
            <p class="text-xs text-text-sub">{{ formatPlayTime(game.total_play_time) }}</p>
            <p class="text-[10px] text-text-sub/60 mt-0.5">{{ formatDate(game.last_played_at) }}</p>
          </div>
          <!-- Hover action buttons -->
          <div class="flex items-center gap-1 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
            <button
              class="p-1.5 rounded-lg bg-primary-500 text-white hover:bg-primary-600 transition-colors"
              :title="t('game.launchTitle')"
              @click.stop="emit('launch', game.id)"
            >
              <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor">
                <polygon points="5 3 19 12 5 21 5 3"/>
              </svg>
            </button>
            <button
              class="p-1.5 rounded-lg bg-input-bg border border-border-medium text-text-sub hover:bg-primary-50 hover:text-primary-600 transition-colors"
              :title="t('game.editTitle')"
              @click.stop="emit('edit', game.id)"
            >
              <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/>
                <path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/>
              </svg>
            </button>
          </div>
        </div>
      </RecycleScroller>
      </template>
    </div>
  </div>
</template>

<style scoped>
.virtual-list {
  height: 100%;
  min-height: 400px;
}
</style>
