<script setup lang="ts">
import { ref, computed, watch, onUnmounted, watchEffect, nextTick } from "vue";
import { useGameStore, loadImage } from "../stores/gameStore";
import { useLazyCovers } from "../composables/useLazyCovers";
import { useI18n } from "vue-i18n";
import type { Game } from "../types";
import { formatPlayTime as fmtPlayTime, formatDate as fmtDate, highlightText } from "../utils/format";

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
  import: [];
  create: [];
}>();

const store = useGameStore();

const { coverUrls, landscapeIds, vObserveCover, clearCover } = useLazyCovers();
const lastClickedIndex = ref<number | null>(null);
const hoveredGameId = ref<number | null>(null);

// 分块渐进渲染：先渲染首块，哨兵进入视口后追加下一块
const CHUNK_SIZE = 60;
const visibleCount = ref(CHUNK_SIZE);
const displayedGames = computed(() => props.games.slice(0, visibleCount.value));
watch(
  () => props.games,
  () => {
    visibleCount.value = CHUNK_SIZE;
  }
);

const sentinelEl = ref<HTMLElement | null>(null);
let chunkObserver: IntersectionObserver | null = null;

async function appendChunk() {
  if (visibleCount.value >= props.games.length) return;
  visibleCount.value = Math.min(visibleCount.value + CHUNK_SIZE, props.games.length);
  // DOM 更新后哨兵可能仍在视口附近（如窗口很高、每行卡片少），继续补块
  await nextTick();
  const el = sentinelEl.value;
  if (!el) return;
  if (el.getBoundingClientRect().top < window.innerHeight + 600) {
    appendChunk();
  }
}

watch(sentinelEl, (el) => {
  if (chunkObserver) {
    chunkObserver.disconnect();
    chunkObserver = null;
  }
  if (!el) return;
  chunkObserver = new IntersectionObserver(
    (entries) => {
      if (entries.some((e) => e.isIntersecting)) appendChunk();
    },
    { rootMargin: "600px" }
  );
  chunkObserver.observe(el);
});
onUnmounted(() => chunkObserver?.disconnect());

// 空库引导插画（可在设置中自定义）
const emptyIllustrationUrl = ref("");
watchEffect(async () => {
  const path = store.settings.custom_empty_illustration;
  emptyIllustrationUrl.value = path ? await loadImage(path) : "";
});

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

function isLandscape(gameId: number): boolean {
  return landscapeIds.value.has(gameId);
}

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
  return fmtPlayTime(seconds, t, "game", true);
}

function formatDate(dateStr: string | null): string {
  return fmtDate(dateStr, t);
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
  return highlightText(name, store.searchKeyword);
}
</script>

<template>
  <div>
    <!-- Empty State -->
    <div
      v-if="games.length === 0"
      class="flex flex-col items-center justify-center h-[60vh] text-text-sub"
    >
      <template v-if="store.games.length === 0">
        <img
          v-if="emptyIllustrationUrl"
          :src="emptyIllustrationUrl"
          class="w-44 h-44 object-contain mb-6 opacity-80"
          alt=""
        />
        <div v-else class="text-8xl mb-8 opacity-20">🎮</div>
        <p class="text-xl font-medium text-text-main">{{ t('game.noGamesYet') }}</p>
        <p class="text-sm mt-2 opacity-60">{{ t('game.emptyGuideHint') }}</p>
        <div class="flex gap-3 mt-6">
          <button
            class="px-5 py-2.5 rounded-xl bg-gradient-to-r from-sakura-400 to-sakura-500 text-white text-sm font-medium hover:from-sakura-500 hover:to-sakura-500 transition-all shadow-sm"
            @click="emit('import')"
          >
            📦 {{ t('game.emptyImport') }}
          </button>
          <button
            class="px-5 py-2.5 rounded-xl border border-primary-200 text-primary-600 text-sm font-medium hover:bg-primary-50 transition-colors"
            @click="emit('create')"
          >
            ➕ {{ t('game.emptyAdd') }}
          </button>
        </div>
        <p class="text-xs mt-6 opacity-50">{{ t('game.importHint') }}</p>
      </template>
      <template v-else>
        <div class="text-8xl mb-8 opacity-20">🎮</div>
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
        v-for="(game, index) in displayedGames"
        :key="game.id"
        v-observe-cover
        :data-game-id="game.id"
        :data-cover-id="game.id"
        :data-cover-path="game.cover_path"
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
          <template v-if="coverUrl(game)">
            <!-- 横图：模糊背景填充 + 完整前景图 -->
            <template v-if="isLandscape(game.id)">
              <img
                :src="coverUrl(game)"
                class="absolute inset-0 w-full h-full object-cover scale-110 blur-lg opacity-50"
              />
              <img
                :src="coverUrl(game)"
                :alt="game.name"
                class="relative w-full h-full object-contain"
                @error="clearCover(game.id)"
              />
            </template>
            <!-- 竖图/方图：直接填充 -->
            <img
              v-else
              :src="coverUrl(game)"
              :alt="game.name"
              class="w-full h-full object-cover"
              @error="clearCover(game.id)"
            />
          </template>
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
            <h3 class="text-white text-sm font-semibold leading-snug drop-shadow-md truncate" v-html="highlightName(game.name)" />
            <!-- Tag badges（单行溢出隐藏） -->
            <div v-if="store.gameTags.get(game.id)?.length" class="flex gap-1 mt-1.5 overflow-hidden">
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

      <!-- List View -->
      <template v-else>
      <div
        v-for="(game, index) in displayedGames"
        :key="game.id"
        v-observe-cover
        :data-game-id="game.id"
        :data-cover-id="game.id"
        :data-cover-path="game.cover_path"
        draggable="true"
        class="group flex items-center gap-4 px-4 py-3 rounded-2xl bg-card shadow-sm hover:shadow-md transition-all duration-200 cursor-pointer"
        :class="[{ 'ring-2 ring-primary-400': isSelected(game) }, dragOverGameId === game.id ? 'opacity-50 scale-95' : '']"
        @click="handleClick(game, index, $event)"
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
          <img v-if="coverUrl(game)" :src="coverUrl(game)" class="w-full h-full object-cover" @error="clearCover(game.id)" />
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
      </template>
    </div>

    <!-- 分块渲染哨兵：进入视口后追加渲染下一块 -->
    <div v-if="displayedGames.length < games.length" ref="sentinelEl" class="h-px"></div>
  </div>
</template>

<style scoped>
</style>
