<script setup lang="ts">
import { ref, computed, watch, watchEffect } from "vue";
import { useModStore } from "../stores/modStore";
import { useGameStore, loadImage } from "../stores/gameStore";
import { useI18n } from "vue-i18n";
import type { Mod, Tag } from "../types";
import { categoryLabel as catLabel } from "../utils/mod";
import { openInExplorer, formatDate } from "../utils/format";
import DetailPanel from "./DetailPanel.vue";
import DetailSection from "./DetailSection.vue";

const { t } = useI18n();
const modStore = useModStore();
const gameStore = useGameStore();

const emit = defineEmits<{
  editMod: [];
  openDir: [];
  deleteMod: [];
  linkGame: [];
  addTag: [];
}>();

const modTags = ref<Tag[]>([]);
const coverUrl = ref("");

// 保留最近一次有效的 mod 快照：父级 <Transition> 要求子组件始终渲染元素根节点，
// 删除 mod 后 leave 动画期间仍渲染旧内容，避免根节点塌缩为注释占位符
const lastValidMod = ref<Mod | null>(null);
watch(
  () => modStore.selectedMod,
  (m) => {
    if (m) lastValidMod.value = m;
  },
  { immediate: true }
);
const displayMod = computed(() => (modStore.selectedMod ?? lastValidMod.value) as Mod);

watchEffect(async () => {
  const mod = displayMod.value;
  if (mod) {
    modTags.value = await modStore.getModTags(mod.id);
  } else {
    modTags.value = [];
  }
});

watchEffect(async () => {
  const mod = displayMod.value;
  if (mod?.cover_path) {
    coverUrl.value = await loadImage(mod.cover_path) || "";
  } else {
    coverUrl.value = "";
  }
});

function categoryLabel(category: string): string {
  return catLabel(category, t);
}

function getGameName(gameId: number | null): string {
  if (gameId === null) return "";
  const game = gameStore.games.find((g) => g.id === gameId);
  return game ? game.name : "";
}

function openPath(path: string) {
  if (path) openInExplorer(path);
}

function closePanel() {
  modStore.selectedModId = null;
}

// Path items computed for loop rendering
const pathItems = computed(() => {
  const mod = displayMod.value;
  if (!mod) return [];
  const items: { label: string; value: string; key: string }[] = [];
  if (mod.mod_path) items.push({ label: t('mod.modPath'), value: mod.mod_path, key: 'mod' });
  if (mod.install_path) items.push({ label: t('mod.installPath'), value: mod.install_path, key: 'install' });
  if (mod.game_dir) items.push({ label: t('mod.gameDir'), value: mod.game_dir, key: 'game' });
  return items;
});

// Detail info rows
const infoRows = computed(() => {
  const mod = displayMod.value;
  if (!mod) return [];
  const rows: { label: string; value: string; key: string }[] = [];
  if (mod.author) rows.push({ label: t('mod.author'), value: mod.author, key: 'author' });
  if (mod.version) rows.push({ label: t('mod.version'), value: `v${mod.version}`, key: 'version' });
  if (mod.category) rows.push({ label: t('mod.category'), value: categoryLabel(mod.category), key: 'category' });
  if (mod.mod_type && mod.mod_type !== 'file') rows.push({ label: t('mod.modType'), value: mod.mod_type, key: 'modtype' });
  if (mod.original_name) rows.push({ label: t('mod.originalName'), value: mod.original_name, key: 'origname' });
  rows.push({ label: t('mod.createdAt'), value: formatDate(mod.created_at, t), key: 'created' });
  rows.push({ label: t('mod.updatedAt'), value: formatDate(mod.updated_at, t), key: 'updated' });
  return rows;
});
</script>

<template>
  <DetailPanel
    v-if="displayMod"
    :coverUrl="coverUrl"
    coverAspect="video"
    fallbackIcon="🧩"
    @close="closePanel"
  >
    <!-- Name -->
    <div>
      <h2 class="text-xl font-bold text-text-main leading-tight">
        {{ displayMod.name }}
      </h2>
    </div>

    <!-- Enable Status -->
    <DetailSection :label="t('mod.statusLabel')">
      <div class="flex items-center gap-3">
        <button
          class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors"
          :class="displayMod.is_enabled ? 'bg-green-500' : 'bg-border-medium'"
          @click="modStore.toggleModEnabled(displayMod.id)"
        >
          <span
            class="inline-block h-4 w-4 transform rounded-full bg-white transition-transform shadow-sm"
            :class="displayMod.is_enabled ? 'translate-x-6' : 'translate-x-1'"
          />
        </button>
        <span
          class="text-sm font-medium"
          :class="displayMod.is_enabled ? 'text-green-600 dark:text-green-400' : 'text-red-500 dark:text-red-400'"
        >
          {{ displayMod.is_enabled ? t('mod.enabled') : t('mod.disabled') }}
        </span>
      </div>
    </DetailSection>

    <!-- Description -->
    <DetailSection :label="t('mod.description')">
      <p v-if="displayMod.description" class="text-sm text-text-main leading-relaxed">
        {{ displayMod.description }}
      </p>
      <p v-else class="text-sm text-text-sub italic">—</p>
    </DetailSection>

    <!-- Detail Info -->
    <DetailSection :label="t('mod.detailInfo')">
      <div class="space-y-2">
        <div v-for="row in infoRows" :key="row.key" class="flex items-center gap-3">
          <span class="text-xs text-text-sub w-16 shrink-0">{{ row.label }}</span>
          <span class="text-xs text-text-main truncate">{{ row.value }}</span>
        </div>
        <div v-if="displayMod.source_url" class="flex items-center gap-3">
          <span class="text-xs text-text-sub w-16 shrink-0">{{ t('mod.sourceUrl') }}</span>
          <a
            :href="displayMod.source_url"
            target="_blank"
            rel="noopener"
            class="text-xs text-primary-500 hover:text-primary-600 truncate hover:underline"
          >
            {{ displayMod.source_url }}
          </a>
        </div>
      </div>
    </DetailSection>

    <!-- Path Info -->
    <DetailSection :label="t('mod.pathInfo')">
      <div v-if="pathItems.length > 0" class="space-y-3">
        <div
          v-for="item in pathItems"
          :key="item.key"
          class="flex items-start gap-2.5 group/item"
        >
          <span class="mt-0.5 text-text-sub/60 shrink-0">
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/>
            </svg>
          </span>
          <div class="min-w-0 flex-1">
            <p class="text-text-sub text-[10px] font-medium uppercase tracking-wide mb-0.5">{{ item.label }}</p>
            <p
              class="text-text-main text-xs truncate cursor-pointer hover:text-primary-600 transition-colors"
              :title="item.value"
              @click="openPath(item.value)"
            >
              {{ item.value }}
            </p>
          </div>
        </div>
      </div>
      <p v-else class="text-sm text-text-sub italic">—</p>
    </DetailSection>

    <!-- Linked Game -->
    <DetailSection :label="t('mod.linkedGame')">
      <div v-if="displayMod.game_id" class="flex items-center gap-3">
        <span class="text-sm text-text-main">{{ getGameName(displayMod.game_id) }}</span>
        <button
          class="px-3 py-1.5 rounded-lg text-xs text-red-500 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/30 transition-colors border border-red-200 dark:border-red-800"
          @click="modStore.unlinkModFromGame(displayMod.id)"
        >
          {{ t('mod.unlinkGame') }}
        </button>
      </div>
      <div v-else>
        <button
          class="px-3 py-1.5 rounded-lg text-xs text-primary-500 hover:bg-primary-50 transition-colors border border-primary-200"
          @click="emit('linkGame')"
        >
          + {{ t('mod.linkedGame') }}
        </button>
      </div>
    </DetailSection>

    <!-- Tags -->
    <DetailSection :label="t('mod.tags')">
      <div class="flex flex-wrap gap-1.5">
        <span
          v-for="tag in modTags"
          :key="tag.id"
          class="inline-flex items-center px-2.5 py-1 rounded-lg bg-primary-50 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 text-xs font-medium"
        >
          {{ tag.name }}
        </span>
        <button
          class="px-2.5 py-1 rounded-lg text-xs text-primary-500 hover:bg-primary-50 transition-colors border border-dashed border-primary-300"
          @click="emit('addTag')"
        >
          {{ t('mod.addTag') }}
        </button>
      </div>
    </DetailSection>

    <!-- Action Buttons -->
    <div class="flex gap-3 pt-2">
      <button
        class="flex-1 px-5 py-3 rounded-2xl bg-gradient-to-r from-primary-500 to-primary-600 text-white text-sm font-medium hover:from-primary-600 hover:to-primary-700 transition-all shadow-lg shadow-primary-500/20"
        @click="emit('editMod')"
      >
        {{ t('mod.edit') }}
      </button>
      <button
        class="px-5 py-3 rounded-2xl border border-border-medium text-sm text-text-sub hover:bg-input-bg transition-colors"
        @click="emit('openDir')"
      >
        {{ t('mod.openDirectory') }}
      </button>
    </div>

    <!-- Delete Button -->
    <button
      class="w-full px-5 py-3 rounded-2xl bg-red-50 dark:bg-red-900/30 text-red-500 dark:text-red-400 text-sm font-medium hover:bg-red-100 dark:hover:bg-red-900/50 transition-colors border border-red-200 dark:border-red-800"
      @click="emit('deleteMod')"
    >
      {{ t('mod.delete') }}
    </button>
  </DetailPanel>
</template>
