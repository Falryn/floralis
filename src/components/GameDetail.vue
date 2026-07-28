<script setup lang="ts">
import { ref, watchEffect } from "vue";
import { open } from "@tauri-apps/plugin-shell";
import { open as openFile } from "@tauri-apps/plugin-dialog";
import { useGameStore, loadImage } from "../stores/gameStore";
import { useI18n } from "vue-i18n";
import type { Game, PlaySession } from "../types";

const { t } = useI18n();

const props = defineProps<{
  game: Game;
}>();

const emit = defineEmits<{
  close: [];
  edit: [];
  launch: [];
}>();

const store = useGameStore();

const coverUrl = ref("");
const playSessions = ref<PlaySession[]>([]);
const screenshots = ref<{ id: number; path: string; url: string }[]>([]);

watchEffect(async () => {
  if (props.game.cover_path) {
    coverUrl.value = await loadImage(props.game.cover_path);
  } else {
    coverUrl.value = "";
  }
});

watchEffect(async () => {
  playSessions.value = await store.getPlaySessions(props.game.id, 5);
  await store.loadGameTags(props.game.id);
  // Load screenshots
  const raw = await store.getGameScreenshots(props.game.id);
  const loaded = [];
  for (const s of raw) {
    const url = await loadImage(s.path);
    if (url) loaded.push({ id: s.id, path: s.path, url });
  }
  screenshots.value = loaded;
});

async function addScreenshot() {
  const paths = await openFile({
    multiple: true,
    filters: [{ name: t('game.screenshots'), extensions: ["jpg", "jpeg", "png", "webp"] }],
  });
  if (!paths) return;
  for (const p of paths) {
    await store.addGameScreenshot(props.game.id, p as string);
  }
  // Reload screenshots
  const raw = await store.getGameScreenshots(props.game.id);
  const loaded = [];
  for (const s of raw) {
    const url = await loadImage(s.path);
    if (url) loaded.push({ id: s.id, path: s.path, url });
  }
  screenshots.value = loaded;
}

async function removeScreenshot(id: number) {
  await store.deleteGameScreenshot(id);
  screenshots.value = screenshots.value.filter((s) => s.id !== id);
}

const currentGameTags = () => store.gameTags.get(props.game.id) ?? [];

function formatPlayTime(seconds: number): string {
  if (seconds < 60) return t('game.seconds', { n: seconds });
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 0) return t('game.hoursMinutes', { h: hours, m: minutes });
  return t('game.minutesOnly', { m: minutes });
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return t('game.never');
  return dateStr.slice(0, 16).replace("T", " ");
}

function openPath(path: string) {
  if (path) open(path).catch(() => {});
}

const groupName = () => {
  if (!props.game.group_id) return t('game.ungrouped');
  return store.groups.find((g) => g.id === props.game.group_id)?.name ?? t('game.ungrouped');
};

const statusOptions = [
  { id: "not_played", icon: "🆕", nameKey: "game.notPlayed", activeClass: "border-gray-400 bg-gray-100 text-gray-700" },
  { id: "playing", icon: "▶️", nameKey: "game.playing", activeClass: "border-green-500 bg-green-50 text-green-700" },
  { id: "completed", icon: "✅", nameKey: "game.completed", activeClass: "border-blue-500 bg-blue-50 text-blue-700" },
  { id: "shelved", icon: "📌", nameKey: "game.shelved", activeClass: "border-yellow-500 bg-yellow-50 text-yellow-700" },
];

function setRating(score: number) {
  // Toggle: clicking same score clears it
  const newRating = props.game.rating === score ? 0 : score;
  store.setGameRating(props.game.id, newRating);
}

function starColor(starIndex: number): string {
  // rating is 1-10, each star = 2 points (5 stars max)
  const threshold = starIndex * 2;
  if (props.game.rating >= threshold) return "text-yellow-400";
  if (props.game.rating >= threshold - 1) return "text-yellow-400/50";
  return "text-gray-300";
}
</script>

<template>
  <div
    class="game-detail-panel w-[400px] h-full bg-detail-bg border-l border-border-light flex flex-col overflow-auto shadow-xl"
  >
    <!-- Cover Image -->
    <div class="relative shrink-0">
      <div class="aspect-square bg-gradient-to-br from-primary-50 to-sakura-50 overflow-hidden">
        <!-- Blurred background layer -->
        <img
          v-if="coverUrl"
          :src="coverUrl"
          class="absolute inset-0 w-full h-full object-cover scale-110 blur-xl opacity-60"
          @error="coverUrl = ''"
        />
        <!-- Main cover image -->
        <img
          v-if="coverUrl"
          :src="coverUrl"
          :alt="game.name"
          class="relative w-full h-full object-cover"
          @error="coverUrl = ''"
        />
        <div
          v-else
          class="w-full h-full flex items-center justify-center text-7xl text-primary-200"
        >
          🎮
        </div>
      </div>
      <!-- Overlay gradient -->
      <div class="absolute inset-x-0 bottom-0 h-20 bg-gradient-to-t from-detail-bg to-transparent" />
      <button
        class="absolute top-4 right-4 w-10 h-10 flex items-center justify-center rounded-xl bg-overlay-white backdrop-blur-sm hover:bg-white/90 shadow-md transition-all text-sm"
        @click="emit('close')"
      >
        ✕
      </button>
    </div>

    <!-- Info -->
    <div class="flex-1 px-8 pb-8 -mt-4 relative space-y-8 overflow-auto">
      <div>
        <h2 class="text-xl font-bold text-text-main leading-tight">
          {{ game.name }}
        </h2>
        <p class="text-xs text-text-sub mt-2 flex items-center gap-1.5">
          <span class="inline-block w-2 h-2 rounded-full bg-primary-400" />
          {{ groupName() }}
        </p>
      </div>

      <!-- Status Selector -->
      <div>
        <p class="text-text-sub text-xs mb-2 font-medium uppercase tracking-wide">{{ t('game.gameStatus') }}</p>
        <div class="flex gap-2">
          <button
            v-for="s in statusOptions"
            :key="s.id"
            class="flex-1 py-2 px-2 rounded-xl border-2 text-xs font-medium transition-all"
            :class="game.status === s.id
              ? s.activeClass
              : 'border-border-medium text-text-sub hover:border-primary-300'"
            @click="store.setGameStatus(game.id, s.id)"
          >
            {{ s.icon }} {{ t(s.nameKey) }}
          </button>
        </div>
      </div>

      <!-- Rating -->
      <div>
        <p class="text-text-sub text-xs mb-2 font-medium uppercase tracking-wide">
          {{ t('game.rating') }} <span v-if="game.rating > 0" class="normal-case text-text-main">{{ game.rating }}/10</span>
        </p>
        <div class="flex items-center gap-1">
          <button
            v-for="i in 5"
            :key="i"
            class="p-0.5 transition-transform hover:scale-125"
            @click="setRating(i * 2)"
          >
            <svg class="w-6 h-6" :class="starColor(i)" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/>
            </svg>
          </button>
          <span v-if="game.rating === 0" class="text-xs text-text-sub ml-2">{{ t('game.clickToRate') }}</span>
        </div>
      </div>

      <!-- Actions -->
      <div class="flex gap-3">
        <button
          class="flex-1 px-5 py-3 rounded-2xl bg-gradient-to-r from-primary-500 to-primary-600 text-white text-sm font-medium hover:from-primary-600 hover:to-primary-700 transition-all shadow-lg shadow-primary-500/20"
          @click="emit('launch')"
        >
          ▶ {{ t('game.launch') }}
        </button>
        <button
          class="px-5 py-3 rounded-2xl border border-border-medium text-sm text-text-sub hover:bg-code-bg transition-colors"
          @click="emit('edit')"
        >
          {{ t('game.edit') }}
        </button>
      </div>

      <!-- Play Time Stats -->
      <div class="bg-code-bg rounded-2xl p-4 space-y-3">
        <div class="flex justify-between items-center">
          <span class="text-xs text-text-sub font-medium">{{ t('game.totalPlayTime') }}</span>
          <span class="text-sm text-text-main font-semibold">{{ formatPlayTime(game.total_play_time) }}</span>
        </div>
        <div class="flex justify-between items-center">
          <span class="text-xs text-text-sub font-medium">{{ t('game.lastPlayed') }}</span>
          <span class="text-sm text-text-main">{{ formatDate(game.last_played_at) }}</span>
        </div>
        <div v-if="playSessions.length > 0" class="pt-2 border-t border-border-light">
          <p class="text-xs text-text-sub font-medium mb-2">{{ t('game.recentRecords') }}</p>
          <div class="space-y-1.5">
            <div
              v-for="session in playSessions"
              :key="session.id"
              class="flex justify-between text-xs"
            >
              <span class="text-text-sub">{{ formatDate(session.start_time) }}</span>
              <span class="text-text-main">{{ formatPlayTime(session.duration_seconds) }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Tags (read-only) -->
      <div>
        <p class="text-text-sub text-xs mb-2 font-medium uppercase tracking-wide">{{ t('game.tags') }}</p>
        <div class="flex flex-wrap gap-1.5">
          <span
            v-for="tag in currentGameTags()"
            :key="tag.id"
            class="inline-flex items-center px-2.5 py-1 rounded-lg bg-primary-50 text-primary-600 text-xs font-medium"
          >
            {{ tag.name }}
          </span>
          <span
            v-if="currentGameTags().length === 0"
            class="text-xs text-text-sub italic cursor-pointer hover:text-primary-500 transition-colors"
            @click="emit('edit')"
          >
            {{ t('game.clickToAddTag') }}
          </span>
        </div>
      </div>

      <!-- File Paths (with icons) -->
      <div class="space-y-5 text-sm">
        <div v-if="game.exe_path" class="flex items-start gap-2.5 group/item">
          <span class="mt-0.5 text-text-sub/60 shrink-0" :title="t('game.exePath')">
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <rect x="4" y="4" width="16" height="16" rx="2"/>
              <path d="M9 9h6M9 12h6M9 15h4"/>
            </svg>
          </span>
          <div class="min-w-0 flex-1">
            <p class="text-text-sub text-[10px] font-medium uppercase tracking-wide mb-0.5">{{ t('game.exePath') }}</p>
            <p
              class="text-text-main text-xs truncate cursor-pointer hover:text-primary-600 transition-colors"
              @click="openPath(game.exe_path)"
              :title="game.exe_path"
            >
              {{ game.exe_path }}
            </p>
          </div>
        </div>

        <div v-if="game.launch_args" class="flex items-start gap-2.5">
          <span class="mt-0.5 text-text-sub/60 shrink-0">
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M4 6h16M4 12h16M4 18h10"/>
            </svg>
          </span>
          <div class="min-w-0 flex-1">
            <p class="text-text-sub text-[10px] font-medium uppercase tracking-wide mb-0.5">{{ t('game.launchArgs') }}</p>
            <p class="text-text-main font-mono text-xs bg-code-bg rounded-lg px-2.5 py-1.5">
              {{ game.launch_args }}
            </p>
          </div>
        </div>

        <div v-if="game.script_path" class="flex items-start gap-2.5 group/item">
          <span class="mt-0.5 text-text-sub/60 shrink-0" :title="t('game.scriptPath')">
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/>
              <polyline points="14 2 14 8 20 8"/>
            </svg>
          </span>
          <div class="min-w-0 flex-1">
            <p class="text-text-sub text-[10px] font-medium uppercase tracking-wide mb-0.5">{{ t('game.scriptPath') }}</p>
            <p
              class="text-text-main text-xs truncate cursor-pointer hover:text-primary-600 transition-colors"
              @click="openPath(game.script_path)"
              :title="game.script_path"
            >
              {{ game.script_path }}
            </p>
            <p v-if="game.script_args" class="text-text-sub text-[10px] mt-0.5 font-mono">
              {{ t('game.params') }} {{ game.script_args }}
            </p>
          </div>
        </div>

        <div class="flex items-start gap-2.5 group/item">
          <span class="mt-0.5 text-text-sub/60 shrink-0" :title="t('game.installPath')">
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/>
            </svg>
          </span>
          <div class="min-w-0 flex-1">
            <p class="text-text-sub text-[10px] font-medium uppercase tracking-wide mb-0.5">{{ t('game.installPath') }}</p>
            <p
              class="text-text-main text-xs truncate cursor-pointer hover:text-primary-600 transition-colors"
              @click="openPath(game.install_path)"
              :title="game.install_path"
            >
              {{ game.install_path || t('game.notSet') }}
            </p>
          </div>
        </div>

        <div class="flex items-start gap-2.5 group/item">
          <span class="mt-0.5 text-text-sub/60 shrink-0" :title="t('game.savePath')">
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M19 21H5a2 2 0 01-2-2V5a2 2 0 012-2h11l5 5v11a2 2 0 01-2 2z"/>
              <polyline points="17 21 17 13 7 13 7 21"/>
              <polyline points="7 3 7 8 15 8"/>
            </svg>
          </span>
          <div class="min-w-0 flex-1">
            <p class="text-text-sub text-[10px] font-medium uppercase tracking-wide mb-0.5">{{ t('game.savePath') }}</p>
            <p
              v-if="game.save_path"
              class="text-text-main text-xs truncate cursor-pointer hover:text-primary-600 transition-colors"
              @click="openPath(game.save_path)"
              :title="game.save_path"
            >
              {{ game.save_path }}
            </p>
            <p v-else class="text-text-sub text-xs italic">{{ t('game.noSaveDetected') }}</p>
          </div>
        </div>

        <div v-if="game.notes" class="flex items-start gap-2.5">
          <span class="mt-0.5 text-text-sub/60 shrink-0">
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/>
              <path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/>
            </svg>
          </span>
          <div class="min-w-0 flex-1">
            <p class="text-text-sub text-[10px] font-medium uppercase tracking-wide mb-0.5">{{ t('game.notes') }}</p>
            <p class="text-text-main text-xs leading-relaxed">{{ game.notes }}</p>
          </div>
        </div>
      </div>

      <!-- Screenshots -->
      <div class="bg-code-bg rounded-2xl p-4 space-y-3">
        <div class="flex justify-between items-center">
          <p class="text-xs text-text-sub font-medium">{{ t('game.screenshots') }}</p>
          <button
            class="text-xs text-primary-500 hover:text-primary-600 transition-colors"
            @click="addScreenshot"
          >
            {{ t('game.addScreenshot') }}
          </button>
        </div>
        <div v-if="screenshots.length > 0" class="grid grid-cols-3 gap-2">
          <div
            v-for="s in screenshots"
            :key="s.id"
            class="relative group aspect-square rounded-lg overflow-hidden bg-black/10"
          >
            <img :src="s.url" class="w-full h-full object-cover" />
            <button
              class="absolute top-1 right-1 w-5 h-5 rounded-full bg-black/60 text-white text-xs opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center"
              @click="removeScreenshot(s.id)"
            >
              ×
            </button>
          </div>
        </div>
        <p v-else class="text-xs text-text-sub italic">{{ t('game.noScreenshots') }}</p>
      </div>
    </div>
  </div>
</template>
