<script setup lang="ts">
import { ref, computed, watchEffect } from "vue";
import { open as openFile } from "@tauri-apps/plugin-dialog";
import { useGameStore, loadImage } from "../stores/gameStore";
import { useModStore } from "../stores/modStore";
import { useI18n } from "vue-i18n";
import type { Game, PlaySession, LaunchAction } from "../types";
import { formatPlayTime as fmtPlayTime, formatDate as fmtDate, openInExplorer } from "../utils/format";
import DetailPanel from "./DetailPanel.vue";
import DetailSection from "./DetailSection.vue";

const { t } = useI18n();

const props = defineProps<{
  game: Game;
}>();

const emit = defineEmits<{
  close: [];
  edit: [];
  launch: [actionId?: number];
  manageMods: [];
}>();

const store = useGameStore();
const modStore = useModStore();

const gameMods = computed(() => {
  return modStore.mods.filter(m => m.game_id === props.game.id);
});

const coverUrl = ref("");
const playSessions = ref<PlaySession[]>([]);
const screenshots = ref<{ id: number; path: string; url: string }[]>([]);
const showAllSessions = ref(false);

// 附加启动入口下拉
const launchActions = ref<LaunchAction[]>([]);
const showLaunchMenu = ref(false);

watchEffect(async () => {
  if (props.game.cover_path) {
    coverUrl.value = await loadImage(props.game.cover_path);
  } else {
    coverUrl.value = "";
  }
});

watchEffect(async () => {
  playSessions.value = await store.getPlaySessions(props.game.id, showAllSessions.value ? 50 : 5);
  await store.loadGameTags(props.game.id);
  launchActions.value = await store.loadLaunchActions(props.game.id);
  showLaunchMenu.value = false;
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
  return fmtPlayTime(seconds, t);
}

function formatDate(dateStr: string | null): string {
  return fmtDate(dateStr, t, true);
}

/** 会话时间范围显示：开始时间 + 结束时刻（或进行中） */
function sessionRange(session: PlaySession): string {
  const start = session.start_time.slice(11, 16);
  if (!session.end_time) return `${start} – ${t('game.sessionOngoing')}`;
  return `${start} – ${session.end_time.slice(11, 16)}`;
}

// 手动修正总游玩时长
const editingPlayTime = ref(false);
const editHours = ref(0);
const editMinutes = ref(0);

function startEditPlayTime() {
  editHours.value = Math.floor(props.game.total_play_time / 3600);
  editMinutes.value = Math.floor((props.game.total_play_time % 3600) / 60);
  editingPlayTime.value = true;
}

async function savePlayTime() {
  const seconds = Math.max(0, (editHours.value || 0) * 3600 + (editMinutes.value || 0) * 60);
  await store.setGamePlayTime(props.game.id, seconds);
  editingPlayTime.value = false;
}

function openPath(path: string) {
  if (path) openInExplorer(path);
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

const NOTES_MAX_LENGTH = 200;

const truncatedNotes = computed(() => {
  const notes = props.game.notes ?? "";
  if (notes.length <= NOTES_MAX_LENGTH) return notes;
  return notes.slice(0, NOTES_MAX_LENGTH) + "…";
});
</script>

<template>
  <DetailPanel
    class="game-detail-panel"
    :data-game-id="props.game.id"
    :coverUrl="coverUrl"
    coverAspect="square"
    fallbackIcon="🎮"
    :showClose="false"
    @close="emit('close')"
  >
    <!-- 标题与分组信息（移至封面下方） -->
    <div class="px-6">
      <h2 class="text-xl font-bold text-text-main leading-tight">
        {{ game.name }}
      </h2>
      <p class="text-xs text-text-sub mt-2 flex items-center gap-1.5">
        <span class="inline-block w-2 h-2 rounded-full bg-primary-400" />
        {{ groupName() }}
      </p>
    </div>

    <!-- Status Selector -->
    <DetailSection :label="t('game.gameStatus')">
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
    </DetailSection>

    <!-- Rating -->
    <DetailSection :label="t('game.rating')">
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
        <span v-else class="text-xs text-text-main ml-2">{{ game.rating }}/10</span>
      </div>
    </DetailSection>

    <!-- Actions -->
    <div class="flex gap-3">
        <div class="flex-1 relative">
          <div class="flex gap-2">
            <button
              class="flex-1 px-5 py-3 rounded-2xl bg-gradient-to-r from-primary-500 to-primary-600 text-white text-sm font-medium hover:from-primary-600 hover:to-primary-700 transition-all shadow-lg shadow-primary-500/20"
              @click="emit('launch')"
            >
              ▶ {{ t('game.launch') }}
            </button>
            <button
              v-if="launchActions.length > 0"
              class="px-3 py-3 rounded-2xl bg-primary-500 text-white text-sm hover:bg-primary-600 transition-colors shadow-lg shadow-primary-500/20"
              :title="t('edit.launchActions')"
              @click="showLaunchMenu = !showLaunchMenu"
            >
              ▾
            </button>
          </div>
          <!-- 附加入口下拉（透明遮罩点击关闭） -->
          <template v-if="showLaunchMenu">
            <div class="fixed inset-0 z-10" @click="showLaunchMenu = false"></div>
            <div class="absolute left-0 right-0 top-full mt-1.5 z-20 bg-modal-bg border border-border-light rounded-xl shadow-2xl py-1">
              <button
                v-for="action in launchActions"
                :key="action.id"
                class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 transition-colors truncate"
                :title="action.program_path"
                @click="emit('launch', action.id); showLaunchMenu = false"
              >
                ▶ {{ action.name }}
              </button>
            </div>
          </template>
        </div>
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
          <span class="flex items-center gap-2">
            <span class="text-sm text-text-main font-semibold">{{ formatPlayTime(game.total_play_time) }}</span>
            <button
              v-if="!editingPlayTime"
              class="text-xs text-primary-500 hover:text-primary-600 transition-colors"
              :title="t('game.adjustPlayTime')"
              @click="startEditPlayTime"
            >
              ✏️ {{ t('game.adjustPlayTime') }}
            </button>
          </span>
        </div>
        <!-- 修正时长：时/分输入 -->
        <div v-if="editingPlayTime" class="flex items-center gap-2">
          <input
            v-model.number="editHours"
            type="number"
            min="0"
            class="w-16 px-2 py-1.5 text-xs rounded-lg border border-primary-200 bg-input-bg outline-none focus:border-primary-400 transition-colors"
          />
          <span class="text-xs text-text-sub">{{ t('game.hoursUnit') }}</span>
          <input
            v-model.number="editMinutes"
            type="number"
            min="0"
            max="59"
            class="w-16 px-2 py-1.5 text-xs rounded-lg border border-primary-200 bg-input-bg outline-none focus:border-primary-400 transition-colors"
          />
          <span class="text-xs text-text-sub">{{ t('game.minutesUnit') }}</span>
          <div class="flex-1"></div>
          <button
            class="px-2 py-1 text-xs rounded-lg bg-primary-500 text-white hover:bg-primary-600 transition-colors"
            @click="savePlayTime"
          >
            {{ t('common.save') }}
          </button>
          <button
            class="px-2 py-1 text-xs rounded-lg border border-border-medium text-text-sub hover:bg-primary-50 transition-colors"
            @click="editingPlayTime = false"
          >
            {{ t('common.cancel') }}
          </button>
        </div>
        <div class="flex justify-between items-center">
          <span class="text-xs text-text-sub font-medium">{{ t('game.lastPlayed') }}</span>
          <span class="text-sm text-text-main">{{ formatDate(game.last_played_at) }}</span>
        </div>
        <div v-if="playSessions.length > 0" class="pt-2 border-t border-border-light">
          <div class="flex justify-between items-center mb-2">
            <p class="text-xs text-text-sub font-medium">{{ t('game.recentRecords') }}</p>
            <button
              class="text-xs text-primary-500 hover:text-primary-600 transition-colors"
              @click="showAllSessions = !showAllSessions"
            >
              {{ showAllSessions ? t('game.collapseSessions') : t('game.showAllSessions') }}
            </button>
          </div>
          <div class="space-y-1.5 max-h-48 overflow-auto">
            <div
              v-for="session in playSessions"
              :key="session.id"
              class="flex justify-between items-center text-xs gap-2"
            >
              <span class="text-text-sub shrink-0">{{ formatDate(session.start_time) }}</span>
              <span class="text-text-sub/80 flex-1 text-center truncate">{{ sessionRange(session) }}</span>
              <span
                class="text-text-main shrink-0"
                :class="{ 'text-green-500 font-medium': !session.end_time }"
              >
                {{ formatPlayTime(session.duration_seconds) }}
              </span>
            </div>
          </div>
        </div>
      </div>

    <!-- Tags (read-only) -->
    <DetailSection :label="t('game.tags')">
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
    </DetailSection>

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
            <p class="text-text-main text-xs leading-relaxed" :title="game.notes">{{ truncatedNotes }}</p>
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

    <!-- Linked Mods -->
      <div class="bg-code-bg rounded-2xl p-4 space-y-3">
        <div class="flex justify-between items-center">
          <p class="text-xs text-text-sub font-medium">{{ t('mod.linkedMods') }} ({{ gameMods.length }})</p>
        </div>
        <div v-if="gameMods.length > 0" class="space-y-2">
          <div
            v-for="mod in gameMods"
            :key="mod.id"
            class="flex items-center justify-between py-1.5"
          >
            <span class="text-xs text-text-main truncate flex-1">{{ mod.name }}</span>
            <button
              class="ml-2 shrink-0 w-8 h-5 rounded-full transition-colors relative"
              :class="mod.is_enabled ? 'bg-primary-500' : 'bg-gray-300'"
              @click="modStore.toggleModEnabled(mod.id)"
            >
              <span
                class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-all"
                :class="mod.is_enabled ? 'left-3.5' : 'left-0.5'"
              />
            </button>
          </div>
        </div>
        <p v-else class="text-xs text-text-sub italic">{{ t('mod.noMods') }}</p>
        <div class="flex gap-2 pt-1">
          <button
            class="flex-1 px-3 py-2 text-xs rounded-xl border border-border-medium text-text-sub hover:bg-primary-50 hover:text-text-main transition-colors"
            @click="emit('manageMods')"
          >
            {{ t('mod.manageMods') }}
          </button>
        </div>
      </div>
  </DetailPanel>
</template>
