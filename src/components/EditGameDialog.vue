<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useGameStore } from "../stores/gameStore";
import { useI18n } from "vue-i18n";
import type { Game } from "../types";
import { useGameMetadataSearch, useLaunchActions } from "../composables/useGameMetadataSearch";
import CustomSelect from "./CustomSelect.vue";
import ConfirmDialog from "./ConfirmDialog.vue";

const { t } = useI18n();

export interface CreateGameData {
  name?: string;
  install_path?: string;
  exe_path?: string;
  cover_path?: string;
  save_path?: string;
  script_path?: string;
  script_args?: string;
}

const props = defineProps<{
  game?: Game;
  createData?: CreateGameData;
  gameId?: number; // Edit mode: load game by ID (from context menu)
}>();

const emit = defineEmits<{
  close: [];
  created: [];
}>();

// 编辑模式：传入 game 对象或 gameId（右键菜单/详情页编辑走 gameId）
const isCreateMode = computed(() => !props.game && !props.gameId);

const store = useGameStore();

const activeTab = ref<'game' | 'mod'>('game');

// Local reactive state for edit mode (loaded from gameId or props.game)
const editableGame = ref<Game | undefined>(props.game);

const name = ref(editableGame.value?.name ?? props.createData?.name ?? "");
const exePath = ref(editableGame.value?.exe_path ?? props.createData?.exe_path ?? "");
const launchArgs = ref(editableGame.value?.launch_args ?? "");
const coverPath = ref(editableGame.value?.cover_path ?? props.createData?.cover_path ?? "");
const savePath = ref(editableGame.value?.save_path ?? props.createData?.save_path ?? "");
const notes = ref(editableGame.value?.notes ?? "");
const selectedGroupId = ref<number | null>(editableGame.value?.group_id ?? null);
const scriptPath = ref(editableGame.value?.script_path ?? props.createData?.script_path ?? "");
const scriptArgs = ref(editableGame.value?.script_args ?? props.createData?.script_args ?? "");
const installPath = ref(editableGame.value?.install_path ?? props.createData?.install_path ?? "");
const defaultModDir = ref(editableGame.value?.default_mod_dir ?? "");
const modNamingPattern = ref(editableGame.value?.mod_naming_pattern ?? "");
const modUsesLoadOrder = ref(editableGame.value?.mod_uses_load_order ?? false);
const trackedProcessName = ref(editableGame.value?.tracked_process_name ?? "");

// ===== 高级启动配置（折叠区块，默认收起）=====
const showAdvancedLaunch = ref(false);

// 附加启动入口草稿：composable 管理加载/编辑/脏检测/持久化
const {
  actionDrafts, configuredActionCount,
  loadActionsFor, addActionDraft, removeActionDraft,
  actionsDirty, persistActions,
} = useLaunchActions();
async function selectActionProgram(index: number) {
  const path = await open({
    filters: [{ name: t('edit.exeFile'), extensions: ["exe", "bat", "cmd"] }],
    multiple: false,
    directory: false,
  });
  if (path) actionDrafts.value[index].program_path = path as string;
}

const groupOptions = computed(() => [
  { label: t('game.ungrouped'), value: null as number | null },
  ...store.groups.map((g) => ({ label: g.name, value: g.id as number | null })),
]);

async function selectExe() {
  const path = await open({
    filters: [{ name: t('edit.exeFile'), extensions: ["exe", "bat", "cmd"] }],
    multiple: false,
    directory: false,
  });
  if (path) exePath.value = path as string;
}

const coverCopying = ref(false);
const showDeleteConfirm = ref(false);
const showPathConfirm = ref(false);
const pathWarningMsg = ref("");

// Tag management
const newTagName = ref("");
const showTagPicker = ref(false);

// Load game data if gameId provided (for context menu edit)
watch(() => props.gameId, async (id) => {
  if (id) {
    const game = store.games.find(g => g.id === id);
    if (game) {
      // Update local reactive state
      editableGame.value = game;
      name.value = game.name;
      exePath.value = game.exe_path;
      launchArgs.value = game.launch_args;
      coverPath.value = game.cover_path;
      savePath.value = game.save_path;
      notes.value = game.notes;
      selectedGroupId.value = game.group_id;
      scriptPath.value = game.script_path;
      scriptArgs.value = game.script_args;
      installPath.value = game.install_path;
      defaultModDir.value = game.default_mod_dir;
      modNamingPattern.value = game.mod_naming_pattern;
      modUsesLoadOrder.value = game.mod_uses_load_order;
      trackedProcessName.value = game.tracked_process_name;
      await store.loadGameTags(id);
      await loadActionsFor(id);
    }
  }
}, { immediate: true });

const currentGameTags = () => editableGame.value ? store.gameTags.get(editableGame.value.id) ?? [] : [];

const availableTags = () => {
  const current = new Set(currentGameTags().map((t) => t.id));
  return store.tags.filter((t) => !current.has(t.id));
};

async function handleAddTag() {
  if (!editableGame.value) return;
  const name = newTagName.value.trim();
  if (!name) return;
  const existing = store.tags.find((t) => t.name === name);
  let tagId: number;
  if (existing) {
    tagId = existing.id;
  } else {
    tagId = await store.createTag(name);
  }
  await store.addGameTag(editableGame.value.id, tagId);
  newTagName.value = "";
  showTagPicker.value = false;
}

async function handleRemoveTag(tagId: number) {
  if (!editableGame.value) return;
  await store.removeGameTag(editableGame.value.id, tagId);
}

async function handlePickTag(tagId: number) {
  if (!editableGame.value) return;
  await store.addGameTag(editableGame.value.id, tagId);
  showTagPicker.value = false;
}

// 四数据源元数据搜索（Bangumi / Steam / VNDB / IGDB）：状态与逻辑见 composable
const {
  dataSource, searching,
  vndbResults, vndbSearching, vndbError, showVndbResults,
  igdbResults, igdbSearching, igdbError, showIgdbResults,
  bangumiResults, bangumiSearching, bangumiError, showBangumiResults,
  steamResults, steamSearching, steamError, showSteamResults,
  pendingCoverUrl, pendingCoverSource, pendingSteamAppId,
  doSearch, searchBangumi,
  applyVndbResult, applyIgdbResult, applyBangumiResult, applySteamResult,
} = useGameMetadataSearch({
  name,
  isCreateMode,
  gameId: () => editableGame.value?.id,
  handlers: {
    setName: (v) => { name.value = v; },
    setNotes: (v) => { notes.value = v; },
    setCover: (v) => { coverPath.value = v; },
  },
});

onMounted(async () => {
  if (editableGame.value) {
    await store.loadGameTags(editableGame.value.id);
    await loadActionsFor(editableGame.value.id);
  } else if (name.value.trim()) {
    // 创建模式：自动触发 Bangumi 搜索
    await searchBangumi();
  }
});

async function selectCover() {
  const path = await open({
    filters: [{ name: t('edit.coverImage'), extensions: ["jpg", "jpeg", "png", "webp"] }],
    multiple: false,
    directory: false,
  });
  if (path) {
    coverCopying.value = true;
    try {
      const stored = await invoke<string>("copy_cover_to_storage", {
        sourcePath: path as string,
        gameId: editableGame.value?.id ?? null,
      });
      coverPath.value = stored;
    } catch (e) {
      console.error("封面复制失败:", e);
      coverPath.value = path as string;
    } finally {
      coverCopying.value = false;
    }
  }
}

async function rescanCover() {
  if (!editableGame.value) return;
  const result = await invoke<string>("scan_game_cover", { id: editableGame.value.id });
  if (result) coverPath.value = result;
}

async function rescanSave() {
  if (!editableGame.value) return;
  const result = await invoke<string>("scan_game_save", { id: editableGame.value.id });
  if (result) savePath.value = result;
}

async function selectScript() {
  const path = await open({
    filters: [{ name: t('edit.scriptPath'), extensions: ["bat", "cmd", "ps1", "sh", "py"] }],
    multiple: false,
    directory: false,
  });
  if (path) scriptPath.value = path as string;
}

async function selectInstallDir() {
  const path = await open({ directory: true, multiple: false });
  if (path) installPath.value = path as string;
}

async function save() {
  // 路径有效性校验
  if (exePath.value) {
    const exists = await invoke<boolean>("check_path_exists", { path: exePath.value });
    if (!exists) {
      pathWarningMsg.value = t('edit.pathNotFound', { path: exePath.value });
      showPathConfirm.value = true;
      return;
    }
  }
  if (installPath.value) {
    const exists = await invoke<boolean>("check_path_exists", { path: installPath.value });
    if (!exists) {
      pathWarningMsg.value = t('edit.pathNotFound', { path: installPath.value });
      showPathConfirm.value = true;
      return;
    }
  }
  await doSave();
}

async function doSave() {
  if (isCreateMode.value) {
    // Create mode: add game
    const gameId = await store.addGame({
      name: name.value,
      group_id: selectedGroupId.value,
      install_path: installPath.value,
      exe_path: exePath.value,
      launch_args: launchArgs.value,
      cover_path: "",
      save_path: savePath.value,
      notes: notes.value,
      script_path: scriptPath.value,
      script_args: scriptArgs.value,
      status: "not_played",
      rating: 0,
      sort_order: 0,
      default_mod_dir: "",
      mod_naming_pattern: "",
      mod_uses_load_order: false,
      tracked_process_name: trackedProcessName.value,
      is_favorite: false,
    });
    // Download pending cover from VNDB/IGDB/Bangumi
    let finalCover = coverPath.value;
    if (pendingCoverUrl.value && pendingCoverSource.value) {
      try {
        const cmdMap: Record<string, string> = {
          vndb: 'download_vndb_cover',
          igdb: 'download_igdb_cover',
          bangumi: 'download_bangumi_cover',
          steam: 'download_steam_cover',
        };
        const cmd = cmdMap[pendingCoverSource.value] || 'download_bangumi_cover';
        const invokeArgs: Record<string, unknown> = { gameId };
        if (pendingCoverSource.value === 'steam') {
          invokeArgs.appId = pendingSteamAppId.value;
        } else {
          invokeArgs.url = pendingCoverUrl.value;
        }
        const localPath = await invoke<string>(cmd, invokeArgs);
        finalCover = localPath;
      } catch (e) {
        console.warn('封面下载失败:', e);
        finalCover = "";
      }
    }
    // Update game with final cover and auto-scan
    if (finalCover || true) {
      await invoke('update_game', {
        id: gameId,
        name: name.value,
        groupId: selectedGroupId.value,
        installPath: installPath.value,
        exePath: exePath.value,
        launchArgs: launchArgs.value,
        coverPath: finalCover,
        savePath: savePath.value,
        notes: notes.value,
        scriptPath: scriptPath.value,
        scriptArgs: scriptArgs.value,
        defaultModDir: "",
        modNamingPattern: "",
        modUsesLoadOrder: false,
        trackedProcessName: trackedProcessName.value,
      });
    }
    // Auto-scan cover & save if no cover was set
    if (!finalCover) {
      await invoke('scan_game_cover', { id: gameId }).catch(() => {});
    }
    await invoke('scan_game_save', { id: gameId }).catch(() => {});
    // 附加启动入口（新建时 gameId 刚生成，有配置才写入）
    if (actionDrafts.value.some((d) => d.program_path.trim())) {
      await persistActions(gameId);
    }
    await store.loadGames();
    emit('created');
    emit('close');
  } else {
    // Edit mode: update game（editableGame 同时覆盖 props.game 与 gameId 两种来源）
    await store.updateGame({
      ...editableGame.value!,
      name: name.value,
      group_id: selectedGroupId.value,
      install_path: installPath.value,
      exe_path: exePath.value,
      launch_args: launchArgs.value,
      cover_path: coverPath.value,
      save_path: savePath.value,
      notes: notes.value,
      script_path: scriptPath.value,
      script_args: scriptArgs.value,
      default_mod_dir: defaultModDir.value,
      mod_naming_pattern: modNamingPattern.value,
      mod_uses_load_order: modUsesLoadOrder.value,
      tracked_process_name: trackedProcessName.value,
    });
    // 附加启动入口：仅在内容有变更时写入
    if (actionsDirty()) {
      await persistActions(editableGame.value!.id);
    }
    emit("close");
  }
}

function onPathConfirm() {
  showPathConfirm.value = false;
  doSave();
}

async function remove() {
  showDeleteConfirm.value = true;
}

async function confirmDelete() {
  showDeleteConfirm.value = false;
  if (editableGame.value) {
    await store.deleteGame(editableGame.value.id);
  }
  emit("close");
}

</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm"
    @click.self="emit('close')"
  >
    <div class="modal-panel bg-modal-bg rounded-3xl shadow-2xl w-[600px] max-h-[80vh] flex flex-col overflow-hidden">
      <!-- Header -->
      <div class="flex items-center justify-between px-8 py-6 border-b border-border-light">
        <h2 class="text-lg font-bold text-text-main">{{ isCreateMode ? t('edit.addTitle') : t('edit.title') }}</h2>
        <button
          class="p-1.5 rounded-lg hover:bg-primary-50 text-text-sub transition-colors"
          @click="emit('close')"
        >
          ✕
        </button>
      </div>

      <!-- Tab Bar (hidden in create mode) -->
      <div v-if="!isCreateMode" class="flex items-center gap-1 px-8 pt-4 pb-1">
        <button
          v-for="tab in (['game', 'mod'] as const)"
          :key="tab"
          class="px-4 py-2 rounded-xl text-sm font-medium transition-all"
          :class="activeTab === tab
            ? 'bg-primary-500 text-white shadow-sm'
            : 'text-text-sub hover:bg-primary-50 hover:text-text-main'"
          @click="activeTab = tab"
        >
          {{ tab === 'game' ? '🎮 ' + t('edit.tabGame') : '🧩 ' + t('edit.tabMod') }}
        </button>
      </div>

      <div class="px-8 py-6 space-y-6 flex-1 min-h-0 overflow-auto edit-scroll">
        <!-- ===== Game Tab ===== -->
        <template v-if="activeTab === 'game'">
        <!-- Name -->
        <div>
          <div class="flex items-center justify-between mb-1.5">
            <label class="block text-sm font-medium text-text-main">{{ t('edit.gameName') }}</label>
            <div class="flex items-center gap-1.5">
              <CustomSelect
                v-model="dataSource"
                :options="[
                  { label: 'Bangumi', value: 'bangumi' },
                  { label: 'Steam', value: 'steam' },
                  { label: 'VNDB', value: 'vndb' },
                  { label: 'IGDB', value: 'igdb' },
                ]"
                class="w-24"
              />
              <button
                class="px-2.5 py-1 text-xs rounded-lg border border-primary-200 text-primary-500 hover:bg-primary-50 transition-colors"
                :disabled="searching || !name.trim()"
                @click="doSearch"
              >
                {{ searching ? t('edit.searching') : '🔍 ' + (dataSource === 'vndb' ? t('edit.vndbMatch') : dataSource === 'igdb' ? t('edit.igdbMatch') : dataSource === 'steam' ? t('edit.steamMatch') : t('edit.bangumiMatch')) }}
              </button>
            </div>
          </div>
          <input
            v-model="name"
            class="w-full px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors"
            :placeholder="t('edit.gameName')"
          />
          <!-- VNDB results -->
          <div v-if="vndbError" class="mt-1.5 text-xs text-red-500">{{ vndbError }}</div>
          <div v-if="showVndbResults" class="mt-2 space-y-1.5 max-h-48 overflow-auto">
            <div v-if="vndbResults.length === 0" class="text-xs text-text-sub italic">{{ t('edit.noResults') }}</div>
            <button
              v-for="item in vndbResults"
              :key="item.id"
              class="w-full flex items-center gap-3 px-3 py-2 rounded-xl bg-code-bg hover:bg-primary-50 transition-colors text-left"
              @click="applyVndbResult(item)"
            >
              <div class="w-10 h-10 rounded-lg overflow-hidden bg-primary-100 shrink-0">
                <img v-if="item.image?.url" :src="item.image.url" class="w-full h-full object-cover" />
                <div v-else class="w-full h-full flex items-center justify-center text-lg text-primary-300">🎮</div>
              </div>
              <div class="min-w-0 flex-1">
                <p class="text-sm text-text-main font-medium truncate">{{ item.title }}</p>
                <p class="text-[10px] text-text-sub truncate">{{ item.id }}</p>
              </div>
            </button>
          </div>
          <!-- IGDB results -->
          <div v-if="igdbError" class="mt-1.5 text-xs text-red-500">{{ igdbError }}</div>
          <div v-if="showIgdbResults" class="mt-2 space-y-1.5 max-h-48 overflow-auto">
            <div v-if="igdbResults.length === 0" class="text-xs text-text-sub italic">{{ t('edit.noResults') }}</div>
            <button
              v-for="item in igdbResults"
              :key="item.id"
              class="w-full flex items-center gap-3 px-3 py-2 rounded-xl bg-code-bg hover:bg-primary-50 transition-colors text-left"
              @click="applyIgdbResult(item)"
            >
              <div class="w-10 h-10 rounded-lg overflow-hidden bg-primary-100 shrink-0">
                <img v-if="item.cover?.url" :src="'https:' + item.cover.url" class="w-full h-full object-cover" />
                <div v-else class="w-full h-full flex items-center justify-center text-lg text-primary-300">🎮</div>
              </div>
              <div class="min-w-0 flex-1">
                <p class="text-sm text-text-main font-medium truncate">{{ item.name }}</p>
                <p class="text-[10px] text-text-sub truncate">ID: {{ item.id }}</p>
              </div>
            </button>
          </div>
          <!-- Bangumi results -->
          <div v-if="bangumiError" class="mt-1.5 text-xs text-red-500">{{ bangumiError }}</div>
          <div v-if="showBangumiResults" class="mt-2 space-y-1.5 max-h-48 overflow-auto">
            <div v-if="bangumiResults.length === 0" class="text-xs text-text-sub italic">{{ t('edit.noResults') }}</div>
            <button
              v-for="item in bangumiResults"
              :key="item.id"
              class="w-full flex items-center gap-3 px-3 py-2 rounded-xl bg-code-bg hover:bg-primary-50 transition-colors text-left"
              @click="applyBangumiResult(item)"
            >
              <div class="w-10 h-10 rounded-lg overflow-hidden bg-primary-100 shrink-0">
                <img v-if="item.images?.large || item.images?.common" :src="item.images?.large || item.images?.common" class="w-full h-full object-cover" />
                <div v-else class="w-full h-full flex items-center justify-center text-lg text-primary-300">🎮</div>
              </div>
              <div class="min-w-0 flex-1">
                <p class="text-sm text-text-main font-medium truncate">{{ item.nameCn || item.name }}</p>
                <p class="text-[10px] text-text-sub truncate">ID: {{ item.id }}</p>
              </div>
            </button>
          </div>
          <!-- Steam results -->
          <div v-if="steamError" class="mt-1.5 text-xs text-red-500">{{ steamError }}</div>
          <div v-if="showSteamResults" class="mt-2 space-y-1.5 max-h-48 overflow-auto">
            <div v-if="steamResults.length === 0" class="text-xs text-text-sub italic">{{ t('edit.noResults') }}</div>
            <button
              v-for="item in steamResults"
              :key="item.id"
              class="w-full flex items-center gap-3 px-3 py-2 rounded-xl bg-code-bg hover:bg-primary-50 transition-colors text-left"
              @click="applySteamResult(item)"
            >
              <div class="w-10 h-10 rounded-lg overflow-hidden bg-primary-100 shrink-0">
                <img :src="`https://cdn.akamai.steamstatic.com/steam/apps/${item.id}/header.jpg`" class="w-full h-full object-cover" />
              </div>
              <div class="min-w-0 flex-1">
                <p class="text-sm text-text-main font-medium truncate">{{ item.name }}</p>
                <p class="text-[10px] text-text-sub truncate">AppID: {{ item.id }}</p>
              </div>
            </button>
          </div>
        </div>

        <!-- Cover（提前到分组前） -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('edit.coverImage') }}</label>
          <div class="flex gap-2">
            <input
              v-model="coverPath"
              class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
              :placeholder="t('edit.coverImage')"
            />
            <button
              class="px-3 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors shrink-0"
              @click="selectCover"
            >
              {{ t('edit.select') }}
            </button>
            <button
              v-if="!isCreateMode"
              class="px-3 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors shrink-0"
              @click="rescanCover"
            >
              {{ t('edit.scan') }}
            </button>
          </div>
        </div>

        <!-- Group -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('edit.group') }}</label>
          <CustomSelect v-model="selectedGroupId" :options="groupOptions" :placeholder="t('game.ungrouped')" searchable />
        </div>

        <!-- Tags（挪后到分组后，仅编辑模式） -->
        <div v-if="!isCreateMode">
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('edit.tags') }}</label>
          <div class="flex flex-wrap gap-1.5 mb-2">
            <span
              v-for="tag in currentGameTags()"
              :key="tag.id"
              class="inline-flex items-center gap-1 px-2.5 py-1 rounded-lg bg-primary-50 text-primary-600 text-xs font-medium"
            >
              {{ tag.name }}
              <button
                class="hover:text-red-500 transition-colors leading-none"
                @click="handleRemoveTag(tag.id)"
              >
                ×
              </button>
            </span>
            <span
              v-if="currentGameTags().length === 0"
              class="text-xs text-text-sub italic"
            >
              {{ t('edit.noTags') }}
            </span>
          </div>
          <div class="flex gap-2">
            <input
              v-model="newTagName"
              :placeholder="t('edit.newTag')"
              class="flex-1 px-3 py-1.5 text-xs rounded-lg border border-primary-200 bg-input-bg text-text-main placeholder-text-sub/50 outline-none focus:border-primary-400 transition-colors"
              @keyup.enter="handleAddTag"
            />
            <button
              class="px-3 py-1.5 text-xs rounded-lg bg-primary-500 text-white hover:bg-primary-600 transition-colors"
              @click="handleAddTag"
            >
              {{ t('edit.addTag') }}
            </button>
            <button
              class="px-3 py-1.5 text-xs rounded-lg border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors"
              @click="showTagPicker = !showTagPicker"
            >
              {{ t('edit.selectTag') }}
            </button>
          </div>
          <div
            v-if="showTagPicker && availableTags().length > 0"
            class="mt-2 flex flex-wrap gap-1.5"
          >
            <button
              v-for="tag in availableTags()"
              :key="tag.id"
              class="px-2.5 py-1 rounded-lg border border-primary-200 text-xs text-text-sub hover:bg-primary-50 hover:text-primary-600 hover:border-primary-300 transition-colors"
              @click="handlePickTag(tag.id)"
            >
              + {{ tag.name }}
            </button>
          </div>
        </div>

        <!-- Install Path -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('edit.installDir') }}</label>
          <div class="flex gap-2">
            <input
              v-model="installPath"
              class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
              :placeholder="t('import.installDirPlaceholder')"
            />
            <button
              class="px-4 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors shrink-0"
              @click="selectInstallDir"
            >
              {{ t('edit.browse') }}
            </button>
          </div>
        </div>

        <!-- Exe Path -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('edit.exeFile') }}</label>
          <div class="flex gap-2">
            <input
              v-model="exePath"
              class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
              :placeholder="t('import.exePlaceholder')"
            />
            <button
              class="px-4 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors shrink-0"
              @click="selectExe"
            >
              {{ t('edit.browse') }}
            </button>
          </div>
        </div>

        <!-- 高级启动配置（默认折叠）：启动参数 / 启动脚本 / 附加启动入口 / 追踪进程 -->
        <div class="border border-primary-200 rounded-xl">
          <button
            type="button"
            class="w-full flex items-center gap-2 px-4 py-3 text-sm font-medium text-text-main hover:bg-primary-50 transition-colors"
            @click="showAdvancedLaunch = !showAdvancedLaunch"
          >
            <span class="text-xs text-text-sub transition-transform" :class="showAdvancedLaunch ? 'rotate-90' : ''">▶</span>
            {{ t('edit.advancedLaunch') }}
            <span
              v-if="configuredActionCount > 0"
              class="px-1.5 py-0.5 rounded-full bg-primary-100 text-primary-600 text-[10px]"
            >
              {{ configuredActionCount }}
            </span>
          </button>
          <div v-if="showAdvancedLaunch" class="px-4 pb-4 space-y-4">
            <!-- Launch Args -->
            <div>
              <label class="block text-sm font-medium text-text-main mb-1.5">
                {{ t('edit.launchArgs') }} <span class="text-text-sub font-normal">({{ t('edit.launchArgsHint') }})</span>
              </label>
              <input
                v-model="launchArgs"
                class="w-full px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors font-mono"
                :placeholder="t('edit.launchArgsPlaceholder')"
              />
            </div>

            <!-- Script Path -->
            <div>
              <label class="block text-sm font-medium text-text-main mb-1.5">
                {{ t('edit.scriptPath') }} <span class="text-text-sub font-normal">({{ t('edit.scriptPathHint') }})</span>
              </label>
              <div class="flex gap-2">
                <input
                  v-model="scriptPath"
                  class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
                  :placeholder="t('edit.scriptPath') + ' (bat/cmd/ps1)'"
                />
                <button
                  class="px-4 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors shrink-0"
                  @click="selectScript"
                >
                  {{ t('edit.browse') }}
                </button>
              </div>
            </div>

            <!-- Script Args -->
            <div v-if="scriptPath">
              <label class="block text-sm font-medium text-text-main mb-1.5">
                {{ t('edit.scriptArgs') }} <span class="text-text-sub font-normal">({{ t('edit.scriptArgsHint') }})</span>
              </label>
              <input
                v-model="scriptArgs"
                class="w-full px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors font-mono"
                :placeholder="t('edit.scriptArgs')"
              />
            </div>

            <!-- 附加启动入口 -->
            <div>
              <div class="flex items-center justify-between mb-1.5">
                <label class="text-sm font-medium text-text-main">{{ t('edit.launchActions') }}</label>
                <button
                  class="px-2.5 py-1 text-xs rounded-lg border border-primary-200 text-primary-500 hover:bg-primary-50 transition-colors"
                  @click="addActionDraft"
                >
                  + {{ t('edit.addLaunchAction') }}
                </button>
              </div>
              <div v-if="actionDrafts.length === 0" class="text-xs text-text-sub italic mb-1">
                {{ t('edit.noLaunchActions') }}
              </div>
              <div
                v-for="(draft, idx) in actionDrafts"
                :key="idx"
                class="p-3 rounded-xl bg-input-bg space-y-1.5 mb-2"
              >
                <div class="flex gap-2">
                  <input
                    v-model="draft.name"
                    :placeholder="t('edit.launchActionNamePlaceholder')"
                    class="w-40 px-3 py-2 text-sm rounded-lg border border-primary-200 outline-none focus:border-primary-400 transition-colors"
                  />
                  <input
                    v-model="draft.program_path"
                    :placeholder="t('edit.launchActionProgram')"
                    class="flex-1 px-3 py-2 text-sm rounded-lg border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
                  />
                  <button
                    class="px-3 py-2 text-sm rounded-lg border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors shrink-0"
                    @click="selectActionProgram(idx)"
                  >
                    {{ t('edit.browse') }}
                  </button>
                  <button
                    class="px-2.5 py-2 text-sm rounded-lg border border-red-200 text-red-400 hover:bg-red-50 transition-colors shrink-0"
                    :title="t('common.delete')"
                    @click="removeActionDraft(idx)"
                  >
                    ✕
                  </button>
                </div>
                <input
                  v-model="draft.args"
                  :placeholder="t('edit.launchActionArgs')"
                  class="w-full px-3 py-2 text-sm rounded-lg border border-primary-200 outline-none focus:border-primary-400 transition-colors font-mono"
                />
              </div>
            </div>

            <!-- Tracked Process Name -->
            <div>
              <label class="block text-sm font-medium text-text-main mb-1.5">
                {{ t('edit.trackedProcessName') }} <span class="text-text-sub font-normal">({{ t('edit.trackedProcessNameHint') }})</span>
              </label>
              <input
                v-model="trackedProcessName"
                class="w-full px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors font-mono"
                :placeholder="t('edit.trackedProcessNamePlaceholder')"
              />
            </div>
          </div>
        </div>

        <!-- Save Path -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('edit.savePath') }}</label>
          <div class="flex gap-2">
            <input
              v-model="savePath"
              class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
              :placeholder="t('edit.savePath')"
            />
            <button
              v-if="!isCreateMode"
              class="px-3 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors shrink-0"
              @click="rescanSave"
            >
              {{ t('edit.scan') }}
            </button>
          </div>
        </div>

        <!-- Notes -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('edit.notes') }}</label>
          <textarea
            v-model="notes"
            rows="3"
            class="w-full px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors resize-none edit-scroll"
            :placeholder="t('edit.notesPlaceholder')"
          ></textarea>
        </div>

        </template>

        <!-- ===== Mod Tab ===== -->
        <template v-if="activeTab === 'mod'">
        <!-- Default Mod Directory -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">
            {{ t('edit.defaultModDir') }} <span class="text-text-sub font-normal">({{ t('edit.defaultModDirHint') }})</span>
          </label>
          <input
            v-model="defaultModDir"
            class="w-full px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors"
            :placeholder="t('edit.defaultModDirPlaceholder')"
          />
        </div>

        <!-- Mod Naming Pattern -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">
            {{ t('edit.modNamingPattern') }} <span class="text-text-sub font-normal">({{ t('edit.modNamingPatternHint') }})</span>
          </label>
          <input
            v-model="modNamingPattern"
            class="w-full px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors"
            :placeholder="t('edit.modNamingPatternPlaceholder')"
          />
        </div>

        <!-- Mod Load Order -->
        <div class="flex items-center gap-3">
          <label class="text-sm font-medium text-text-main">{{ t('edit.modUsesLoadOrder') }}</label>
          <button
            class="relative w-10 h-5.5 rounded-full transition-colors"
            :class="modUsesLoadOrder ? 'bg-primary-500' : 'bg-gray-300'"
            @click="modUsesLoadOrder = !modUsesLoadOrder"
          >
            <span
              class="absolute top-0.5 w-4.5 h-4.5 rounded-full bg-white shadow transition-transform"
              :class="modUsesLoadOrder ? 'left-5' : 'left-0.5'"
            ></span>
          </button>
          <span class="text-xs text-text-sub">{{ t('edit.modUsesLoadOrderHint') }}</span>
        </div>
        </template>

        <!-- Actions -->
        <div class="flex gap-2 pt-2">
          <button
            v-if="!isCreateMode"
            class="px-4 py-2.5 rounded-xl border border-red-200 text-sm text-red-400 hover:bg-red-50 transition-colors"
            @click="remove"
          >
            🗑️ {{ t('edit.delete') }}
          </button>
          <div class="flex-1"></div>
          <button
            class="px-4 py-2.5 rounded-xl border border-primary-200 text-sm text-text-sub hover:bg-primary-50 transition-colors"
            @click="emit('close')"
          >
            {{ t('edit.cancel') }}
          </button>
          <button
            class="px-6 py-2.5 rounded-xl bg-gradient-to-r from-primary-500 to-primary-600 text-white text-sm font-medium hover:from-primary-600 hover:to-primary-700 transition-all shadow-sm"
            @click="save"
          >
            {{ t('edit.save') }}
          </button>
        </div>
      </div>
    </div>
    <transition name="modal">
      <ConfirmDialog
        v-if="showDeleteConfirm"
        :title="t('game.delete')"
        :message="t('game.confirmDelete')"
        :confirm-text="t('common.delete')"
        :danger="true"
        @confirm="confirmDelete"
        @cancel="showDeleteConfirm = false"
      />
    </transition>
    <transition name="modal">
      <ConfirmDialog
        v-if="showPathConfirm"
        :title="t('edit.title')"
        :message="pathWarningMsg"
        :confirm-text="t('common.confirm')"
        @confirm="onPathConfirm"
        @cancel="showPathConfirm = false"
      />
    </transition>
  </div>
</template>

<style scoped>
.edit-scroll {
  scrollbar-width: none;
}
.edit-scroll::-webkit-scrollbar {
  display: none;
}
</style>
