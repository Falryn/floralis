<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "../utils/invoke";
import { useI18n } from "vue-i18n";
import PlayCalendar from "./PlayCalendar.vue";
import { formatPlayTime as fmtPlayTime } from "../utils/format";

const { t } = useI18n();

interface GameStats {
  total_games: number;
  total_play_time: number;
  not_played: number;
  playing: number;
  completed: number;
  shelved: number;
}

const emit = defineEmits<{
  close: [];
}>();

const stats = ref<GameStats | null>(null);

onMounted(async () => {
  try {
    stats.value = await invoke<GameStats>("get_game_stats");
  } catch (e) {
    console.error("Failed to load stats:", e);
  }
});

function formatPlayTime(seconds: number): string {
  return fmtPlayTime(seconds, t, "stats");
}

function formatPlayTimeDays(seconds: number): string {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  if (days > 0) return t('stats.daysHours', { d: days, h: hours });
  return t('stats.hoursOnly', { h: hours });
}
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm"
    @click.self="emit('close')"
  >
    <div class="modal-panel bg-modal-bg rounded-3xl shadow-2xl w-[500px] max-h-[80vh] overflow-hidden flex flex-col">
      <!-- Header -->
      <div class="flex items-center justify-between px-8 py-6 border-b border-border-light shrink-0">
        <h2 class="text-lg font-bold text-text-main">📊 {{ t('stats.title') }}</h2>
        <button
          class="p-1.5 rounded-lg hover:bg-primary-50 text-text-sub transition-colors"
          @click="emit('close')"
        >
          ✕
        </button>
      </div>

      <div class="px-8 py-8 overflow-auto flex-1 space-y-6" v-if="stats">
        <!-- Overview Cards -->
        <div class="grid grid-cols-2 gap-4">
          <div class="p-4 rounded-2xl bg-gradient-to-br from-primary-50 to-primary-100/50 border border-primary-200/50">
            <p class="text-xs text-primary-600 font-medium">{{ t('stats.totalGames') }}</p>
            <p class="text-2xl font-bold text-primary-700 mt-1">{{ stats.total_games }}</p>
          </div>
          <div class="p-4 rounded-2xl bg-gradient-to-br from-sakura-50 to-sakura-100/50 border border-sakura-200/50">
            <p class="text-xs text-sakura-600 font-medium">{{ t('stats.totalPlayTime') }}</p>
            <p class="text-2xl font-bold text-sakura-700 mt-1">{{ formatPlayTimeDays(stats.total_play_time) }}</p>
          </div>
        </div>

        <!-- Status Breakdown -->
        <div>
          <p class="text-sm font-medium text-text-main mb-3">{{ t('stats.statusDistribution') }}</p>
          <div class="space-y-3">
            <div class="flex items-center gap-3">
              <span class="text-sm w-16">🆕 {{ t('stats.notPlayed') }}</span>
              <div class="flex-1 h-6 bg-gray-100 rounded-full overflow-hidden">
                <div
                  class="h-full bg-gray-400 rounded-full transition-all duration-500"
                  :style="{ width: stats.total_games ? (stats.not_played / stats.total_games * 100) + '%' : '0%' }"
                />
              </div>
              <span class="text-sm text-text-sub w-8 text-right">{{ stats.not_played }}</span>
            </div>
            <div class="flex items-center gap-3">
              <span class="text-sm w-16">▶️ {{ t('stats.playing') }}</span>
              <div class="flex-1 h-6 bg-green-50 rounded-full overflow-hidden">
                <div
                  class="h-full bg-green-500 rounded-full transition-all duration-500"
                  :style="{ width: stats.total_games ? (stats.playing / stats.total_games * 100) + '%' : '0%' }"
                />
              </div>
              <span class="text-sm text-text-sub w-8 text-right">{{ stats.playing }}</span>
            </div>
            <div class="flex items-center gap-3">
              <span class="text-sm w-16">✅ {{ t('stats.completed') }}</span>
              <div class="flex-1 h-6 bg-blue-50 rounded-full overflow-hidden">
                <div
                  class="h-full bg-blue-500 rounded-full transition-all duration-500"
                  :style="{ width: stats.total_games ? (stats.completed / stats.total_games * 100) + '%' : '0%' }"
                />
              </div>
              <span class="text-sm text-text-sub w-8 text-right">{{ stats.completed }}</span>
            </div>
            <div class="flex items-center gap-3">
              <span class="text-sm w-16">📌 {{ t('stats.shelved') }}</span>
              <div class="flex-1 h-6 bg-yellow-50 rounded-full overflow-hidden">
                <div
                  class="h-full bg-yellow-500 rounded-full transition-all duration-500"
                  :style="{ width: stats.total_games ? (stats.shelved / stats.total_games * 100) + '%' : '0%' }"
                />
              </div>
              <span class="text-sm text-text-sub w-8 text-right">{{ stats.shelved }}</span>
            </div>
          </div>
        </div>

        <!-- Completion Rate -->
        <div class="p-4 rounded-2xl bg-code-bg border border-border-light">
          <div class="flex justify-between items-center">
            <span class="text-sm text-text-sub">{{ t('stats.completionRate') }}</span>
            <span class="text-lg font-bold text-text-main">
              {{ stats.total_games ? Math.round(stats.completed / stats.total_games * 100) : 0 }}%
            </span>
          </div>
          <div class="mt-2 h-2 bg-gray-200 rounded-full overflow-hidden">
            <div
              class="h-full bg-gradient-to-r from-primary-500 to-sakura-400 rounded-full transition-all duration-500"
              :style="{ width: stats.total_games ? (stats.completed / stats.total_games * 100) + '%' : '0%' }"
            />
          </div>
        </div>

        <!-- Average Play Time -->
        <div class="p-4 rounded-2xl bg-code-bg border border-border-light">
          <div class="flex justify-between items-center">
            <span class="text-sm text-text-sub">{{ t('stats.avgPlayTime') }}</span>
            <span class="text-lg font-bold text-text-main">
              {{ stats.total_games ? formatPlayTime(Math.round(stats.total_play_time / stats.total_games)) : t('stats.seconds', { n: 0 }) }}
            </span>
          </div>
        </div>

        <!-- Play Calendar -->
        <PlayCalendar />
      </div>
    </div>
  </div>
</template>
