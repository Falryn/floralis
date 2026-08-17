<script setup lang="ts">
import { ref, computed, watchEffect } from "vue";
import { useModStore } from "../stores/modStore";
import { useGameStore, loadImage } from "../stores/gameStore";
import { useI18n } from "vue-i18n";
import MultiSelect from "./MultiSelect.vue";
import ViewToolbar from "./ViewToolbar.vue";
import ConfirmDialog from "./ConfirmDialog.vue";
import type { Mod } from "../types";
import { MOD_CATEGORIES, categoryLabel as catLabel } from "../utils/mod";
import { highlightText } from "../utils/format";

const { t } = useI18n();
const modStore = useModStore();
const gameStore = useGameStore();

const emit = defineEmits<{
  importMod: [];
  scanDir: [];
  profiles: [];
  contextMenu: [modId: number, x: number, y: number];
  editMod: [modId: number];
}>();

const showAddMenu = ref(false);

const viewMode = ref<"grid" | "list">("grid");
const groupByGame = ref(true);
const collapsedGroups = ref<Set<string>>(new Set());

// Batch confirm state

const showBatchDeleteConfirm = ref(false);

const categoryOptions = computed(() =>
  MOD_CATEGORIES.map(c => ({ label: t(`mod.cat.${c}`), value: c }))
);

const sortOptions = computed(() => [
  { label: t('mod.sortRecent'), value: 'recent' },
  { label: t('mod.sortNameAsc'), value: 'name_asc' },
  { label: t('mod.sortNameDesc'), value: 'name_desc' },
  { label: t('mod.sortEnabled'), value: 'enabled' },
]);

// Mod cover image loading
const coverUrls = ref<Map<number, string>>(new Map());
const loadedCoverIds = new Set<number>();

watchEffect(async () => {
  const pending = modStore.mods.filter(
    m => m.cover_path && !coverUrls.value.has(m.id) && !loadedCoverIds.has(m.id)
  );
  if (pending.length === 0) return;
  for (const m of pending) loadedCoverIds.add(m.id);
  await Promise.all(
    pending.map(async (mod) => {
      const url = await loadImage(mod.cover_path);
      if (url) coverUrls.value.set(mod.id, url);
    })
  );
});

function modCoverUrl(mod: Mod): string {
  return coverUrls.value.get(mod.id) ?? "";
}

function categoryLabel(category: string): string {
  return catLabel(category, t);
}

// Group mods by game for grouped view
interface ModGroup {
  key: string;
  label: string;
  mods: Mod[];
  enabledCount: number;
}

const groupedMods = computed((): ModGroup[] => {
  const map = new Map<string, Mod[]>();
  const independentKey = '__independent__';

  for (const mod of modStore.filteredMods) {
    const key = mod.game_id !== null ? `game_${mod.game_id}` : independentKey;
    if (!map.has(key)) map.set(key, []);
    map.get(key)!.push(mod);
  }

  const groups: ModGroup[] = [];

  // Game-linked groups first
  for (const game of gameStore.games) {
    const key = `game_${game.id}`;
    const mods = map.get(key);
    if (mods && mods.length > 0) {
      groups.push({
        key,
        label: game.name,
        mods,
        enabledCount: mods.filter(m => m.is_enabled).length,
      });
      map.delete(key);
    }
  }

  // Independent mods last
  const independent = map.get(independentKey);
  if (independent && independent.length > 0) {
    groups.push({
      key: independentKey,
      label: t('mod.independent'),
      mods: independent,
      enabledCount: independent.filter(m => m.is_enabled).length,
    });
  }

  return groups;
});

function toggleGroup(key: string) {
  const s = new Set(collapsedGroups.value);
  if (s.has(key)) s.delete(key);
  else s.add(key);
  collapsedGroups.value = s;
}

function isCollapsed(key: string): boolean {
  return collapsedGroups.value.has(key);
}

function getGameName(gameId: number | null): string {
  if (gameId === null) return "";
  const game = gameStore.games.find((g) => g.id === gameId);
  return game ? game.name : "";
}

// 单击延时：避免与双击编辑冲突（单击打开详情，双击直接编辑）
let clickTimer: ReturnType<typeof setTimeout> | null = null;
function handleModClick(id: number) {
  if (modStore.isModSelectMode) {
    modStore.toggleSelectMod(id);
    return;
  }
  if (clickTimer) clearTimeout(clickTimer);
  clickTimer = setTimeout(() => {
    modStore.selectedModId = id;
  }, 220);
}

function handleModDblClick(id: number) {
  if (clickTimer) clearTimeout(clickTimer);
  emit('editMod', id);
}

function highlightName(name: string): string {
  return highlightText(name, modStore.modSearchKeyword);
}


function confirmBatchDelete() {
  showBatchDeleteConfirm.value = false;
  modStore.batchDeleteMods();
}
</script>

<template>
  <div class="flex flex-col h-full">
    <!-- Top bar -->
    <div class="shrink-0 px-8 pt-8">
      <ViewToolbar
        :title="t('mod.title')"
        :subtitle="t('mod.modCount', { count: modStore.filteredMods.length })"
        v-model:searchModelValue="modStore.modSearchKeyword"
        :searchPlaceholder="t('mod.searchPlaceholder')"
        v-model:sortModelValue="modStore.modSortType"
        :sortOptions="sortOptions"
        sortClass="w-28"
        :showSelectMode="true"
        :isSelectMode="modStore.isModSelectMode"
        v-model:viewMode="viewMode"
        @enterSelectMode="modStore.isModSelectMode = true"
      >
        <template #subtitle-extra>
          <span v-if="modStore.isModSelectMode" class="text-primary-500 font-medium ml-2">
            {{ t('app.selectedCount', { count: modStore.selectedModIds.size }) }}
          </span>
        </template>
        <template #filters>
          <!-- Category multi-select filter -->
          <MultiSelect
            v-model="modStore.modFilterCategories"
            :options="categoryOptions"
            :all-label="t('mod.allCategories')"
            :placeholder="t('mod.category')"
            class="w-32"
          />
          <!-- Group by game toggle -->
          <button
            class="px-3 py-2 text-sm rounded-xl border transition-colors"
            :class="groupByGame ? 'border-primary-300/50 bg-primary-500/10 text-primary-600 dark:text-primary-400' : 'border-border-medium bg-input-bg text-text-sub hover:bg-primary-50 hover:text-text-main'"
            :title="t('mod.groupByGame')"
            @click="groupByGame = !groupByGame"
          >
            <svg class="w-4 h-4 inline-block mr-1 -mt-0.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/>
            </svg>
            {{ t('mod.groupByGame') }}
          </button>
          <!-- Mod profiles -->
          <button
            class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-sub hover:bg-primary-50 hover:text-text-main transition-colors"
            :title="t('mod.profilesTitle')"
            @click="emit('profiles')"
          >
            <svg class="w-4 h-4 inline-block mr-1 -mt-0.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
            </svg>
            {{ t('mod.profiles') }}
          </button>
        </template>
        <template #batch-actions>
          <button
            class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-main hover:bg-primary-50 transition-colors"
            @click="modStore.selectAllMods()"
          >
            {{ t('app.selectAll') }}
          </button>
          <button
            class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-main hover:bg-primary-50 transition-colors"
            @click="modStore.batchToggleEnabled(true)"
          >
            {{ t('mod.batchEnable') }}
          </button>
          <button
            class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-main hover:bg-primary-50 transition-colors"
            @click="modStore.batchToggleEnabled(false)"
          >
            {{ t('mod.batchDisable') }}
          </button>
          <button
            class="px-3 py-2 text-sm rounded-xl bg-red-500 text-white hover:bg-red-600 transition-colors"
            @click="showBatchDeleteConfirm = true"
          >
            {{ t('common.delete') }}
          </button>
          <button
            class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-sub hover:bg-code-bg transition-colors"
            @click="modStore.clearModSelection()"
          >
            {{ t('app.cancel') }}
          </button>
        </template>
      </ViewToolbar>
    </div>

    <!-- Content Area -->
    <div class="flex-1 overflow-auto px-8 pt-2">
      <!-- Empty State -->
      <div
        v-if="modStore.filteredMods.length === 0"
        class="flex flex-col items-center justify-center h-full text-text-sub"
      >
        <div class="text-7xl mb-6 opacity-20">🧩</div>
        <template v-if="modStore.mods.length === 0">
          <p class="text-xl font-medium">{{ t('mod.noMods') }}</p>
          <p class="text-sm mt-3 opacity-60">{{ t('mod.addMod') }} / {{ t('mod.createNew') }}</p>
        </template>
        <template v-else>
          <p class="text-xl font-medium">{{ t('mod.noMatch') }}</p>
          <p class="text-sm mt-3 opacity-60">{{ t('mod.adjustFilter') }}</p>
        </template>
      </div>

      <!-- Grouped View (by game) -->
      <div v-else-if="groupByGame" class="space-y-4">
        <div v-for="group in groupedMods" :key="group.key" class="space-y-2">
          <!-- Group Header -->
          <button
            class="flex items-center gap-2 w-full px-3 py-2 rounded-xl text-left transition-colors hover:bg-input-bg/50"
            @click="toggleGroup(group.key)"
          >
            <svg
              class="w-4 h-4 text-text-sub/60 transition-transform duration-200 shrink-0"
              :class="{ 'rotate-90': !isCollapsed(group.key) }"
              viewBox="0 0 24 24" fill="currentColor"
            >
              <path d="M8.59 16.59L13.17 12 8.59 7.41 10 6l6 6-6 6z"/>
            </svg>
            <span class="text-sm font-semibold text-text-main flex-1 truncate">{{ group.label }}</span>
            <span class="text-[10px] text-text-sub/60 shrink-0">
              {{ group.mods.length }} {{ t('mod.modCountShort', { count: group.mods.length }) }}
              <span v-if="group.enabledCount > 0" class="text-green-500/70 ml-1">
                ({{ group.enabledCount }} {{ t('mod.enabledShort') }})
              </span>
            </span>
          </button>

          <!-- Group Content -->
          <div v-show="!isCollapsed(group.key)">
            <!-- Grid -->
            <div v-if="viewMode === 'grid'" class="grid grid-cols-2 @sm:grid-cols-3 @md:grid-cols-4 @xl:grid-cols-5 gap-3 pl-6">
              <div
                v-for="mod in group.mods"
                :key="mod.id"
                class="group relative flex flex-col h-[150px] rounded-2xl overflow-hidden bg-card shadow-md hover:shadow-2xl transition-all duration-300 cursor-pointer hover:-translate-y-1"
                :class="{ 'ring-2 ring-primary-400': modStore.isModSelectMode ? modStore.selectedModIds.has(mod.id) : modStore.selectedModId === mod.id }"
                @click="handleModClick(mod.id)"
                @dblclick="handleModDblClick(mod.id)"
                @contextmenu.prevent="emit('contextMenu', mod.id, $event.clientX, $event.clientY)"
              >
                <!-- Select mode checkbox -->
                <div v-if="modStore.isModSelectMode" class="absolute top-2 left-2 z-10 w-5 h-5 rounded-md border-2 flex items-center justify-center transition-all"
                  :class="modStore.selectedModIds.has(mod.id) ? 'bg-primary-500 border-primary-500 text-white' : 'border-white/80 bg-black/20'">
                  <svg v-if="modStore.selectedModIds.has(mod.id)" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><polyline points="20 6 9 17 4 12"/></svg>
                </div>
                <!-- Missing file badge -->
                <div v-if="modStore.modMissingIds.has(mod.id)" class="absolute top-1.5 right-1.5 z-10 px-1.5 py-0.5 rounded-md bg-amber-500/90 text-white text-[9px] font-medium shadow-sm" :title="t('mod.missingFileHint')">
                  ⚠ {{ t('mod.missingFile') }}
                </div>
                <!-- 封面区（无封面时使用固定默认样式） -->
                <div class="h-[100px] shrink-0 overflow-hidden bg-gradient-to-br from-slate-200 to-slate-300 dark:from-slate-700 dark:to-slate-800">
                  <img v-if="modCoverUrl(mod)" :src="modCoverUrl(mod)" class="w-full h-full object-cover" loading="lazy" />
                  <div v-else class="w-full h-full flex items-center justify-center text-3xl opacity-40">🧩</div>
                </div>
                <!-- 信息区 -->
                <div class="flex-1 min-h-0 px-2.5 py-1.5 flex flex-col">
                  <div class="flex items-center gap-1.5">
                    <h3 class="flex-1 min-w-0 text-xs font-bold text-text-main truncate" v-html="highlightName(mod.name)" />
                    <button
                      class="shrink-0 w-5 h-5 rounded-md flex items-center justify-center transition-all"
                      :class="mod.is_enabled
                        ? 'bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400 hover:bg-green-200 dark:hover:bg-green-900/50'
                        : 'bg-red-100 dark:bg-red-900/30 text-red-500 dark:text-red-400 hover:bg-red-200 dark:hover:bg-red-900/50'"
                      @click.stop="modStore.toggleModEnabled(mod.id)"
                    >
                      <svg v-if="mod.is_enabled" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                        <polyline points="20 6 9 17 4 12"/>
                      </svg>
                      <svg v-else class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                        <path d="M18 6L6 18M6 6l12 12"/>
                      </svg>
                    </button>
                  </div>
                  <div class="mt-auto flex items-center gap-1.5 text-[10px] text-text-sub overflow-hidden">
                    <span v-if="mod.version" class="px-1 py-0.5 rounded bg-primary-50 dark:bg-primary-900/30 text-primary-500 dark:text-primary-400 font-mono text-[9px] shrink-0">
                      v{{ mod.version }}
                    </span>
                    <span v-if="mod.category" class="px-1.5 py-0.5 rounded bg-sakura-50 dark:bg-sakura-900/30 text-sakura-500 dark:text-sakura-400 text-[9px] shrink-0">
                      {{ categoryLabel(mod.category) }}
                    </span>
                    <span v-if="mod.author" class="ml-auto truncate">{{ mod.author }}</span>
                  </div>
                </div>
              </div>
            </div>
            <!-- List -->
            <div v-else class="flex flex-col gap-1.5 pl-6">
              <div
                v-for="mod in group.mods"
                :key="mod.id"
                class="group flex items-center gap-3 px-4 py-2.5 rounded-2xl bg-card shadow-sm hover:shadow-md transition-all duration-200 cursor-pointer"
                :class="{ 'ring-2 ring-primary-400': modStore.isModSelectMode ? modStore.selectedModIds.has(mod.id) : modStore.selectedModId === mod.id }"
                @click="handleModClick(mod.id)"
                @dblclick="handleModDblClick(mod.id)"
                @contextmenu.prevent="emit('contextMenu', mod.id, $event.clientX, $event.clientY)"
              >
                <!-- Select mode checkbox -->
                <div v-if="modStore.isModSelectMode" class="shrink-0 w-5 h-5 rounded-md border-2 flex items-center justify-center transition-all"
                  :class="modStore.selectedModIds.has(mod.id) ? 'bg-primary-500 border-primary-500 text-white' : 'border-border-medium bg-input-bg'">
                  <svg v-if="modStore.selectedModIds.has(mod.id)" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><polyline points="20 6 9 17 4 12"/></svg>
                </div>
                <div class="shrink-0 w-7 h-7 rounded-lg overflow-hidden flex items-center justify-center shadow-sm bg-gradient-to-br from-slate-200 to-slate-300 dark:from-slate-700 dark:to-slate-800">
                  <img v-if="modCoverUrl(mod)" :src="modCoverUrl(mod)" class="w-full h-full object-cover" />
                  <span v-else class="text-sm opacity-40">🧩</span>
                </div>
                <div class="flex-1 min-w-0">
                  <h3 class="text-sm font-medium text-text-main truncate" v-html="highlightName(mod.name)" />
                  <div class="flex items-center gap-2 mt-0.5">
                    <span v-if="mod.description" class="text-[10px] text-text-sub/70 truncate max-w-[200px]">{{ mod.description }}</span>
                    <span v-if="mod.version" class="text-[10px] text-text-sub font-mono">v{{ mod.version }}</span>
                    <span v-if="mod.author" class="text-[10px] text-text-sub truncate max-w-[100px]">{{ mod.author }}</span>
                  </div>
                </div>
                <span v-if="modStore.modMissingIds.has(mod.id)" class="shrink-0 px-1.5 py-0.5 rounded-md text-[10px] font-medium bg-amber-100 dark:bg-amber-900/30 text-amber-600 dark:text-amber-400" :title="t('mod.missingFileHint')">⚠ {{ t('mod.missingFile') }}</span>
                <span v-if="mod.category" class="shrink-0 text-[9px] px-1.5 py-0.5 rounded bg-sakura-50 dark:bg-sakura-900/30 text-sakura-500 dark:text-sakura-400">{{ categoryLabel(mod.category) }}</span>
                <span
                  class="shrink-0 px-1.5 py-0.5 rounded-md text-[10px] font-medium"
                  :class="mod.is_enabled
                    ? 'bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400'
                    : 'bg-red-100 dark:bg-red-900/30 text-red-500 dark:text-red-400'"
                >
                  {{ mod.is_enabled ? t('mod.enabled') : t('mod.disabled') }}
                </span>
                <button
                  class="shrink-0 w-7 h-7 rounded-lg flex items-center justify-center transition-all shadow-sm"
                  :class="mod.is_enabled
                    ? 'bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400 hover:bg-green-200 dark:hover:bg-green-900/50'
                    : 'bg-red-100 dark:bg-red-900/30 text-red-500 dark:text-red-400 hover:bg-red-200 dark:hover:bg-red-900/50'"
                  @click.stop="modStore.toggleModEnabled(mod.id)"
                >
                  <svg v-if="mod.is_enabled" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                    <polyline points="20 6 9 17 4 12"/>
                  </svg>
                  <svg v-else class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                    <path d="M18 6L6 18M6 6l12 12"/>
                  </svg>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Flat Grid View -->
      <div
        v-else-if="viewMode === 'grid'"
        class="grid grid-cols-2 @sm:grid-cols-3 @md:grid-cols-4 @xl:grid-cols-5 gap-4"
      >
        <div
          v-for="mod in modStore.filteredMods"
          :key="mod.id"
          class="group relative flex flex-col h-[164px] rounded-2xl overflow-hidden bg-card shadow-md hover:shadow-2xl transition-all duration-300 cursor-pointer hover:-translate-y-1"
          :class="{ 'ring-2 ring-primary-400': modStore.isModSelectMode ? modStore.selectedModIds.has(mod.id) : modStore.selectedModId === mod.id }"
          @click="handleModClick(mod.id)"
          @dblclick="handleModDblClick(mod.id)"
          @contextmenu.prevent="emit('contextMenu', mod.id, $event.clientX, $event.clientY)"
        >
          <!-- Select mode checkbox -->
          <div v-if="modStore.isModSelectMode" class="absolute top-2 left-2 z-10 w-5 h-5 rounded-md border-2 flex items-center justify-center transition-all"
            :class="modStore.selectedModIds.has(mod.id) ? 'bg-primary-500 border-primary-500 text-white' : 'border-white/80 bg-black/20'">
            <svg v-if="modStore.selectedModIds.has(mod.id)" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><polyline points="20 6 9 17 4 12"/></svg>
          </div>
          <!-- Missing file badge -->
          <div v-if="modStore.modMissingIds.has(mod.id)" class="absolute top-1.5 right-1.5 z-10 px-1.5 py-0.5 rounded-md bg-amber-500/90 text-white text-[9px] font-medium shadow-sm" :title="t('mod.missingFileHint')">
            ⚠ {{ t('mod.missingFile') }}
          </div>
          <!-- 封面区（无封面时使用固定默认样式） -->
          <div class="h-28 shrink-0 overflow-hidden bg-gradient-to-br from-slate-200 to-slate-300 dark:from-slate-700 dark:to-slate-800">
            <img v-if="modCoverUrl(mod)" :src="modCoverUrl(mod)" class="w-full h-full object-cover" loading="lazy" />
            <div v-else class="w-full h-full flex items-center justify-center text-4xl opacity-40">🧩</div>
          </div>
          <!-- 信息区 -->
          <div class="flex-1 min-h-0 px-3 py-1.5 flex flex-col">
            <div class="flex items-center gap-2">
              <h3 class="flex-1 min-w-0 text-sm font-bold text-text-main truncate" v-html="highlightName(mod.name)" />
              <button
                class="shrink-0 w-6 h-6 rounded-lg flex items-center justify-center transition-all"
                :class="mod.is_enabled
                  ? 'bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400 hover:bg-green-200 dark:hover:bg-green-900/50'
                  : 'bg-red-100 dark:bg-red-900/30 text-red-500 dark:text-red-400 hover:bg-red-200 dark:hover:bg-red-900/50'"
                :title="mod.is_enabled ? t('mod.disabled') : t('mod.enabled')"
                @click.stop="modStore.toggleModEnabled(mod.id)"
              >
                <svg v-if="mod.is_enabled" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                  <polyline points="20 6 9 17 4 12"/>
                </svg>
                <svg v-else class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                  <path d="M18 6L6 18M6 6l12 12"/>
                </svg>
              </button>
            </div>
            <div class="mt-auto flex items-center gap-1.5 text-[10px] text-text-sub overflow-hidden">
              <span v-if="mod.version" class="px-1 py-0.5 rounded bg-primary-50 dark:bg-primary-900/30 text-primary-500 dark:text-primary-400 font-mono text-[9px] shrink-0">
                v{{ mod.version }}
              </span>
              <span v-if="mod.category" class="px-1.5 py-0.5 rounded bg-sakura-50 dark:bg-sakura-900/30 text-sakura-500 dark:text-sakura-400 text-[9px] shrink-0">
                {{ categoryLabel(mod.category) }}
              </span>
              <span v-if="mod.author" class="ml-auto truncate">{{ mod.author }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Flat List View -->
      <div v-else class="flex flex-col gap-2">
        <div
          v-for="mod in modStore.filteredMods"
          :key="mod.id"
          class="group flex items-center gap-3 px-4 py-3 rounded-2xl bg-card shadow-sm hover:shadow-md transition-all duration-200 cursor-pointer"
          :class="{ 'ring-2 ring-primary-400': modStore.isModSelectMode ? modStore.selectedModIds.has(mod.id) : modStore.selectedModId === mod.id }"
          @click="handleModClick(mod.id)"
          @dblclick="handleModDblClick(mod.id)"
          @contextmenu.prevent="emit('contextMenu', mod.id, $event.clientX, $event.clientY)"
        >
          <!-- Select mode checkbox -->
          <div v-if="modStore.isModSelectMode" class="shrink-0 w-5 h-5 rounded-md border-2 flex items-center justify-center transition-all"
            :class="modStore.selectedModIds.has(mod.id) ? 'bg-primary-500 border-primary-500 text-white' : 'border-border-medium bg-input-bg'">
            <svg v-if="modStore.selectedModIds.has(mod.id)" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><polyline points="20 6 9 17 4 12"/></svg>
          </div>
          <div class="shrink-0 w-8 h-8 rounded-lg overflow-hidden flex items-center justify-center shadow-sm bg-gradient-to-br from-slate-200 to-slate-300 dark:from-slate-700 dark:to-slate-800">
            <img v-if="modCoverUrl(mod)" :src="modCoverUrl(mod)" class="w-full h-full object-cover" />
            <span v-else class="text-base opacity-40">🧩</span>
          </div>
          <div class="flex-1 min-w-0">
            <h3 class="text-sm font-medium text-text-main truncate" v-html="highlightName(mod.name)" />
            <div class="flex items-center gap-2 mt-0.5">
              <span v-if="mod.description" class="text-[10px] text-text-sub/70 truncate max-w-[240px]">{{ mod.description }}</span>
              <span v-if="mod.version" class="text-[10px] text-text-sub font-mono">v{{ mod.version }}</span>
              <span v-if="mod.author" class="text-[10px] text-text-sub truncate max-w-[100px]">{{ mod.author }}</span>
            </div>
          </div>
          <span v-if="modStore.modMissingIds.has(mod.id)" class="shrink-0 px-1.5 py-0.5 rounded-md text-[10px] font-medium bg-amber-100 dark:bg-amber-900/30 text-amber-600 dark:text-amber-400" :title="t('mod.missingFileHint')">⚠ {{ t('mod.missingFile') }}</span>
          <span v-if="mod.category" class="shrink-0 text-[9px] px-1.5 py-0.5 rounded bg-sakura-50 dark:bg-sakura-900/30 text-sakura-500 dark:text-sakura-400">{{ categoryLabel(mod.category) }}</span>
          <span class="shrink-0 text-[10px] text-text-sub/60 truncate max-w-[120px]">
            {{ mod.game_id ? getGameName(mod.game_id) : t('mod.independent') }}
          </span>
          <span
            class="shrink-0 px-1.5 py-0.5 rounded-md text-[10px] font-medium"
            :class="mod.is_enabled
              ? 'bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400'
              : 'bg-red-100 dark:bg-red-900/30 text-red-500 dark:text-red-400'"
          >
            {{ mod.is_enabled ? t('mod.enabled') : t('mod.disabled') }}
          </span>
          <button
            class="shrink-0 w-7 h-7 rounded-lg flex items-center justify-center transition-all shadow-sm"
            :class="mod.is_enabled
              ? 'bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400 hover:bg-green-200 dark:hover:bg-green-900/50'
              : 'bg-red-100 dark:bg-red-900/30 text-red-500 dark:text-red-400 hover:bg-red-200 dark:hover:bg-red-900/50'"
            @click.stop="modStore.toggleModEnabled(mod.id)"
          >
            <svg v-if="mod.is_enabled" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            <svg v-else class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
              <path d="M18 6L6 18M6 6l12 12"/>
            </svg>
          </button>
        </div>
      </div>
    </div>

    <!-- Bottom Action Bar -->
    <div class="flex items-center justify-between px-8 py-6 border-t border-border-light shrink-0">
      <div class="flex items-center gap-2">
        <div class="relative">
          <button
            class="px-4 py-2.5 rounded-xl bg-gradient-to-r from-primary-500 to-primary-600 text-white text-sm font-medium hover:from-primary-600 hover:to-primary-700 transition-all shadow-lg shadow-primary-500/20"
            @click="showAddMenu = !showAddMenu"
          >
            + {{ t('mod.addMod') }} ▾
          </button>
          <teleport to="body">
            <div v-if="showAddMenu" class="fixed inset-0 z-10" @click="showAddMenu = false"></div>
          </teleport>
          <div
            v-if="showAddMenu"
            class="absolute bottom-full left-0 mb-2 w-56 py-1.5 rounded-xl bg-modal-bg border border-border-light shadow-xl z-20"
          >
            <button
              class="w-full px-4 py-2.5 text-left hover:bg-primary-50 dark:hover:bg-primary-900/30 transition-colors"
              @click="showAddMenu = false; emit('importMod')"
            >
              <span class="block text-sm text-text-main">📦 {{ t('mod.importModFile') }}</span>
              <span class="block text-[11px] text-text-sub mt-0.5">{{ t('mod.importModFileDesc') }}</span>
            </button>
            <button
              class="w-full px-4 py-2.5 text-left hover:bg-primary-50 dark:hover:bg-primary-900/30 transition-colors"
              @click="showAddMenu = false; emit('scanDir')"
            >
              <span class="block text-sm text-text-main">🔍 {{ t('mod.scanDirectory') }}</span>
              <span class="block text-[11px] text-text-sub mt-0.5">{{ t('mod.scanDirectoryDesc') }}</span>
            </button>
          </div>
        </div>
      </div>

    </div>



    <!-- Batch Delete Confirm -->
    <ConfirmDialog
      v-if="showBatchDeleteConfirm"
      :title="t('common.delete')"
      :message="t('mod.batchDeleteConfirm', { count: modStore.selectedModIds.size })"
      :confirm-text="t('common.delete')"
      :danger="true"
      @confirm="confirmBatchDelete"
      @cancel="showBatchDeleteConfirm = false"
    />
  </div>
</template>
