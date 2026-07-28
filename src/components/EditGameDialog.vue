<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useGameStore } from "../stores/gameStore";
import { useI18n } from "vue-i18n";
import type { Game } from "../types";
import CustomSelect from "./CustomSelect.vue";
import ConfirmDialog from "./ConfirmDialog.vue";

const { t } = useI18n();

const props = defineProps<{
  game: Game;
}>();

const emit = defineEmits<{
  close: [];
}>();

const store = useGameStore();

const name = ref(props.game.name);
const exePath = ref(props.game.exe_path);
const launchArgs = ref(props.game.launch_args);
const coverPath = ref(props.game.cover_path);
const savePath = ref(props.game.save_path);
const notes = ref(props.game.notes);
const selectedGroupId = ref<number | null>(props.game.group_id);
const scriptPath = ref(props.game.script_path);
const scriptArgs = ref(props.game.script_args);
const installPath = ref(props.game.install_path);

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

// Tag management
const newTagName = ref("");
const showTagPicker = ref(false);

const currentGameTags = () => store.gameTags.get(props.game.id) ?? [];

const availableTags = () => {
  const current = new Set(currentGameTags().map((t) => t.id));
  return store.tags.filter((t) => !current.has(t.id));
};

async function handleAddTag() {
  const name = newTagName.value.trim();
  if (!name) return;
  const existing = store.tags.find((t) => t.name === name);
  let tagId: number;
  if (existing) {
    tagId = existing.id;
  } else {
    tagId = await store.createTag(name);
  }
  await store.addGameTag(props.game.id, tagId);
  newTagName.value = "";
  showTagPicker.value = false;
}

async function handleRemoveTag(tagId: number) {
  await store.removeGameTag(props.game.id, tagId);
}

async function handlePickTag(tagId: number) {
  await store.addGameTag(props.game.id, tagId);
  showTagPicker.value = false;
}

onMounted(async () => {
  await store.loadGameTags(props.game.id);
});

// VNDB search
interface VndbItem {
  id: string;
  title: string;
  image?: { url?: string } | null;
  description?: string | null;
}
const vndbResults = ref<VndbItem[]>([]);
const vndbSearching = ref(false);
const vndbError = ref("");
const showVndbResults = ref(false);

async function searchVndb() {
  vndbSearching.value = true;
  vndbError.value = "";
  vndbResults.value = [];
  try {
    vndbResults.value = await invoke<VndbItem[]>("search_vndb", { query: name.value });
    showVndbResults.value = true;
  } catch (e) {
    vndbError.value = e as string;
  } finally {
    vndbSearching.value = false;
  }
}

async function applyVndbResult(item: VndbItem) {
  // Apply title
  name.value = item.title;
  // Apply description as notes
  if (item.description) {
    notes.value = item.description.slice(0, 500);
  }
  // Download cover if available
  const imgUrl = item.image?.url;
  if (imgUrl) {
    try {
      const localPath = await invoke<string>("download_vndb_cover", {
        url: imgUrl,
        gameId: props.game.id,
      });
      coverPath.value = localPath;
    } catch (e) {
      console.warn("VNDB 封面下载失败:", e);
    }
  }
  showVndbResults.value = false;
}

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
        gameId: props.game.id,
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
  const result = await invoke<string>("scan_game_cover", { id: props.game.id });
  if (result) coverPath.value = result;
}

async function rescanSave() {
  const result = await invoke<string>("scan_game_save", { id: props.game.id });
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
      if (!confirm(t('edit.pathNotFound', { path: exePath.value }))) return;
    }
  }
  if (installPath.value) {
    const exists = await invoke<boolean>("check_path_exists", { path: installPath.value });
    if (!exists) {
      if (!confirm(t('edit.pathNotFound', { path: installPath.value }))) return;
    }
  }

  await store.updateGame({
    ...props.game,
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
  });
  emit("close");
}

async function remove() {
  showDeleteConfirm.value = true;
}

async function confirmDelete() {
  showDeleteConfirm.value = false;
  await store.deleteGame(props.game.id);
  emit("close");
}

// IGDB search
interface IgdbItem {
  id: number;
  name: string;
  cover?: { url?: string } | null;
  summary?: string | null;
}
const igdbResults = ref<IgdbItem[]>([]);
const igdbSearching = ref(false);
const igdbError = ref("");
const showIgdbResults = ref(false);
const dataSource = ref<'vndb' | 'igdb'>('vndb');

async function searchIgdb() {
  const clientId = store.settings.igdb_client_id;
  const clientSecret = store.settings.igdb_client_secret;
  if (!clientId || !clientSecret) {
    igdbError.value = t('edit.igdbNotConfigured');
    return;
  }
  igdbSearching.value = true;
  igdbError.value = "";
  igdbResults.value = [];
  try {
    igdbResults.value = await invoke<IgdbItem[]>("search_igdb", {
      query: name.value,
      clientId,
      clientSecret,
    });
    showIgdbResults.value = true;
  } catch (e) {
    igdbError.value = e as string;
  } finally {
    igdbSearching.value = false;
  }
}

async function doSearch() {
  if (dataSource.value === 'vndb') await searchVndb();
  else await searchIgdb();
}

async function applyIgdbResult(item: IgdbItem) {
  name.value = item.name;
  if (item.summary) notes.value = item.summary.slice(0, 500);
  const imgUrl = item.cover?.url;
  if (imgUrl) {
    try {
      const localPath = await invoke<string>("download_igdb_cover", {
        url: imgUrl,
        gameId: props.game.id,
      });
      coverPath.value = localPath;
    } catch (e) {
      console.warn("IGDB cover download failed:", e);
    }
  }
  showIgdbResults.value = false;
}

</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm"
    @click.self="emit('close')"
  >
    <div class="modal-panel bg-modal-bg rounded-3xl shadow-2xl w-[600px] max-h-[80vh] overflow-auto">
      <!-- Header -->
      <div class="flex items-center justify-between px-8 py-6 border-b border-border-light">
        <h2 class="text-lg font-bold text-text-main">{{ t('edit.title') }}</h2>
        <button
          class="p-1.5 rounded-lg hover:bg-primary-50 text-text-sub transition-colors"
          @click="emit('close')"
        >
          ✕
        </button>
      </div>

      <div class="px-8 py-8 space-y-6">
        <!-- Name -->
        <div>
          <div class="flex items-center justify-between mb-1.5">
            <label class="block text-sm font-medium text-text-main">{{ t('edit.gameName') }}</label>
            <div class="flex items-center gap-1.5">
              <select
                v-model="dataSource"
                class="px-2 py-1 text-xs rounded-lg border border-primary-200 bg-input-bg text-text-sub outline-none"
              >
                <option value="vndb">VNDB</option>
                <option value="igdb">IGDB</option>
              </select>
              <button
                class="px-2.5 py-1 text-xs rounded-lg border border-primary-200 text-primary-500 hover:bg-primary-50 transition-colors"
                :disabled="vndbSearching || igdbSearching || !name.trim()"
                @click="doSearch"
              >
                {{ (vndbSearching || igdbSearching) ? t('edit.searching') : '🔍 ' + (dataSource === 'vndb' ? t('edit.vndbMatch') : t('edit.igdbMatch')) }}
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
        </div>

        <!-- Group -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('edit.group') }}</label>
          <CustomSelect v-model="selectedGroupId" :options="groupOptions" :placeholder="t('game.ungrouped')" />
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

        <!-- Cover -->
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
              class="px-3 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors shrink-0"
              @click="rescanCover"
            >
              {{ t('edit.scan') }}
            </button>
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
            class="w-full px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors resize-none"
            :placeholder="t('edit.notesPlaceholder')"
          ></textarea>
        </div>

        <!-- Tags -->
        <div>
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

        <!-- Actions -->
        <div class="flex gap-2 pt-2">
          <button
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
</template>
