<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { open as openUrl } from "@tauri-apps/plugin-shell";
import { useGameStore } from "../stores/gameStore";
import { useI18n } from "vue-i18n";
import type { UpdateInfo, TagUsage } from "../types";
import ConfirmDialog from "./ConfirmDialog.vue";
import { addToast } from "../composables/useToast";

const { locale, t } = useI18n();

const emit = defineEmits<{
  close: [];
  openIntegrity: [];
}>();

const store = useGameStore();

const activeTab = ref<'general' | 'games' | 'mods'>('general');

const languageOptions = [
  { id: "zh-CN", name: "简体中文" },
  { id: "en-US", name: "English" },
  { id: "ja-JP", name: "日本語" },
];
const currentLocale = ref(localStorage.getItem("floralis-locale") || "zh-CN");

function changeLocale() {
  locale.value = currentLocale.value;
  localStorage.setItem("floralis-locale", currentLocale.value);
}

const lightThemes = [
  { id: "light", icon: "💜", nameKey: "settings.themeLavender" },
  { id: "light-sakura", icon: "🌸", nameKey: "settings.themeSakura" },
  { id: "light-mint", icon: "🍃", nameKey: "settings.themeMint" },
];
const darkThemes = [
  { id: "dark", icon: "🌙", nameKey: "settings.themeNight" },
  { id: "dark-ocean", icon: "🌊", nameKey: "settings.themeOcean" },
  { id: "dark-crimson", icon: "🍷", nameKey: "settings.themeCrimson" },
];

const closeBehaviorOptions = [
  { id: "ask", icon: "❓", nameKey: "settings.closeAsk" },
  { id: "exit", icon: "🚪", nameKey: "settings.closeExit" },
  { id: "minimize", icon: "📌", nameKey: "settings.closeMinimize" },
];

const sevenZipPath = ref("");
const defaultExtractPath = ref("");
const newPassword = ref("");
const showPwdList = ref(false);
const newTagName = ref("");
const sevenZipValid = ref<boolean | null>(null);

// Custom images
const bannerPath = ref("");
const sidebarBgPath = ref("");
const bannerBlur = ref(0);
const bannerBrightness = ref(100);
const sidebarBlur = ref(0);
const sidebarBrightness = ref(100);

onMounted(() => {
  sevenZipPath.value = store.settings.seven_zip_path;
  defaultExtractPath.value = store.settings.default_extract_path;
  bannerPath.value = store.settings.custom_banner;
  sidebarBgPath.value = store.settings.custom_sidebar_bg;
  bannerBlur.value = parseInt(store.settings.banner_blur) || 0;
  bannerBrightness.value = parseInt(store.settings.banner_brightness) || 100;
  sidebarBlur.value = parseInt(store.settings.sidebar_blur) || 0;
  sidebarBrightness.value = parseInt(store.settings.sidebar_brightness) || 100;
  updateRepo.value = store.settings.update_repo;
  igdbClientId.value = store.settings.igdb_client_id;
  igdbClientSecret.value = store.settings.igdb_client_secret;
  autoBackup.value = store.settings.auto_backup !== "false";
  loadTagUsage();
});

async function selectSevenZip() {
  const path = await open({
    filters: [{ name: t('edit.exeFile'), extensions: ["exe"] }],
    multiple: false,
    directory: false,
  });
  if (path) {
    sevenZipPath.value = path as string;
    sevenZipValid.value = await invoke<boolean>("test_seven_zip", {
      path: sevenZipPath.value,
    });
    await savePaths();
  }
}

async function selectExtractPath() {
  const path = await open({ directory: true, multiple: false });
  if (path) {
    defaultExtractPath.value = path as string;
    await savePaths();
  }
}

async function testSevenZip() {
  if (!sevenZipPath.value) return;
  sevenZipValid.value = await invoke<boolean>("test_seven_zip", {
    path: sevenZipPath.value,
  });
}

async function savePaths() {
  try {
    await store.saveSettings(sevenZipPath.value, defaultExtractPath.value);
    addToast(t('settings.saveSuccess'), 'success');
  } catch (e) {
    addToast(t('settings.saveFail') + ': ' + (e as string), 'error');
  }
}

async function addPwd() {
  const pwd = newPassword.value.trim();
  if (!pwd) return;
  await store.addPassword(pwd);
  newPassword.value = "";
}

async function addNewTag() {
  const name = newTagName.value.trim();
  if (!name) return;
  await store.createTag(name);
  newTagName.value = "";
  await loadTagUsage();
}

// ===== Tag management =====
const tagUsages = ref<TagUsage[]>([]);
const showTagList = ref(false);
const editingTagId = ref<number | null>(null);
const editingTagName = ref("");
const deletingTag = ref<TagUsage | null>(null);

async function loadTagUsage() {
  tagUsages.value = await store.getTagUsage();
}

function startRenameTag(tag: TagUsage) {
  editingTagId.value = tag.id;
  editingTagName.value = tag.name;
}

function cancelRenameTag() {
  editingTagId.value = null;
  editingTagName.value = "";
}

async function confirmRenameTag() {
  const name = editingTagName.value.trim();
  if (editingTagId.value !== null && name) {
    await store.renameTag(editingTagId.value, name);
    await loadTagUsage();
  }
  cancelRenameTag();
}

async function confirmDeleteTag() {
  if (deletingTag.value) {
    await store.deleteTag(deletingTag.value.id);
    await loadTagUsage();
  }
  deletingTag.value = null;
}

type ImageKey = "custom_banner" | "custom_sidebar_bg";

const imageKeys: Record<ImageKey, { value: string }> = {
  custom_banner: bannerPath,
  custom_sidebar_bg: sidebarBgPath,
};

async function selectCustomImage(key: ImageKey) {
  const path = await open({
    filters: [{ name: t('edit.coverImage'), extensions: ["jpg", "jpeg", "png", "webp", "gif"] }],
    multiple: false,
    directory: false,
  });
  if (path) {
    try {
      const internal = await store.saveCustomImage(key, path as string);
      imageKeys[key].value = internal;
      addToast(t('settings.imageSaved'), 'success');
    } catch (e) {
      addToast(t('settings.saveFail') + ': ' + (e as string), 'error');
    }
  }
}

async function clearCustomImage(key: ImageKey) {
  try {
    imageKeys[key].value = "";
    await store.saveCustomImage(key, "");
    addToast(t('settings.imageCleared'), 'success');
  } catch (e) {
    addToast(t('settings.saveFail') + ': ' + (e as string), 'error');
  }
}

async function saveImageSetting(key: string, value: number) {
  try {
    await invoke("save_setting", { key, value: String(value) });
    (store.settings as unknown as Record<string, string>)[key] = String(value);
  } catch (e) {
    addToast(t('settings.saveFail') + ': ' + (e as string), 'error');
  }
}

// Backup
const importing = ref(false);

// Cover integrity
interface CoverStatus {
  game_id: number;
  game_name: string;
  cover_path: string;
  exists: boolean;
}
const checkingCovers = ref(false);
const coverResults = ref<CoverStatus[]>([]);
const rescanningId = ref<number | null>(null);

async function checkCoverIntegrity() {
  checkingCovers.value = true;
  coverResults.value = [];
  try {
    coverResults.value = await invoke<CoverStatus[]>("check_cover_integrity");
  } catch (e) {
    addToast(t('settings.checkFail') + ': ' + (e as string), 'error');
  } finally {
    checkingCovers.value = false;
  }
}

async function rescanCover(gameId: number) {
  rescanningId.value = gameId;
  try {
    await invoke("rescan_game_cover", { gameId });
    await store.loadGames();
    // Re-check
    coverResults.value = await invoke<CoverStatus[]>("check_cover_integrity");
    addToast(t('settings.coverUpdated'), 'success');
  } catch (e) {
    addToast(t('settings.saveFail') + ': ' + (e as string), 'error');
  } finally {
    rescanningId.value = null;
  }
}

// Update
const updateRepo = ref("");
const checkingUpdate = ref(false);
const updateInfo = ref<UpdateInfo | null>(null);
const updateError = ref("");

// IGDB
const igdbClientId = ref("");
const igdbClientSecret = ref("");

async function doExport() {
  try {
    const json = await store.exportData();
    const filePath = await save({
      filters: [{ name: "JSON", extensions: ["json"] }],
      defaultPath: "floralis-backup.json",
    });
    if (filePath) {
      await invoke("write_text_file", { path: filePath, content: json });
      addToast(t('settings.exportSuccess'), 'success');
    }
  } catch (e) {
    addToast(t('settings.saveFail') + ': ' + (e as string), 'error');
  }
}

async function doImport() {
  const filePath = await open({
    filters: [{ name: "JSON", extensions: ["json"] }],
    multiple: false,
    directory: false,
  });
  if (!filePath) return;
  importing.value = true;
  try {
    const json = await invoke<string>("read_text_file", { path: filePath as string });
    await store.importData(json);
    addToast(t('settings.importSuccess'), 'success');
  } catch (e) {
    addToast(t('settings.saveFail') + ': ' + (e as string), 'error');
  } finally {
    importing.value = false;
  }
}

async function doBackupDatabase() {
  try {
    const path = await store.backupDatabase();
    addToast(t('settings.backupSuccess', { path }), 'success');
  } catch (e) {
    addToast(t('settings.backupFail') + ': ' + (e as string), 'error');
  }
}

// 每日自动备份开关（后端启动时判定 24 小时间隔）
const autoBackup = ref(true);
async function saveAutoBackup() {
  try {
    const value = autoBackup.value ? "true" : "false";
    await invoke("save_setting", { key: "auto_backup", value });
    store.settings.auto_backup = value;
  } catch (e) {
    addToast(t('settings.saveFail') + ': ' + (e as string), 'error');
  }
}

async function doSaveUpdateRepo() {
  await store.saveUpdateRepo(updateRepo.value.trim());
  addToast(t('settings.updateSaved'), 'success');
}

async function saveIgdb() {
  await store.saveIgdbSettings(igdbClientId.value.trim(), igdbClientSecret.value.trim());
  addToast(t('settings.saveSuccess'), 'success');
}

async function doCheckUpdate() {
  checkingUpdate.value = true;
  updateInfo.value = null;
  updateError.value = "";
  try {
    updateInfo.value = await store.checkForUpdate();
  } catch (e) {
    updateError.value = e as string;
  } finally {
    checkingUpdate.value = false;
  }
}
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm"
    @click.self="emit('close')"
  >
    <div class="modal-panel bg-modal-bg rounded-3xl shadow-2xl w-[600px] max-h-[80vh] overflow-hidden flex flex-col">
      <!-- Header -->
      <div class="flex items-center justify-between px-8 py-6 border-b border-border-light shrink-0">
        <h2 class="text-lg font-bold text-text-main">⚙️ {{ t('settings.title') }}</h2>
        <button
          class="p-1.5 rounded-lg hover:bg-primary-50 text-text-sub transition-colors"
          @click="emit('close')"
        >
          ✕
        </button>
      </div>

      <!-- Tab Bar -->
      <div class="flex items-center gap-1 px-8 pt-4 pb-1 shrink-0">
        <button
          v-for="tab in (['general', 'games', 'mods'] as const)"
          :key="tab"
          class="px-4 py-2 rounded-xl text-sm font-medium transition-all"
          :class="activeTab === tab
            ? 'bg-primary-500 text-white shadow-sm'
            : 'text-text-sub hover:bg-primary-50 hover:text-text-main'"
          @click="activeTab = tab"
        >
          {{ t(`settings.tab${tab.charAt(0).toUpperCase() + tab.slice(1)}`) }}
        </button>
      </div>

      <div class="settings-scroll px-8 py-8 space-y-8 overflow-auto flex-1" style="scrollbar-width: none;">
        <!-- ===== General Tab ===== -->
        <template v-if="activeTab === 'general'">
        <!-- Language -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-2">{{ t('settings.language') }}</label>
          <div class="grid grid-cols-3 gap-2">
            <button
              v-for="lang in languageOptions"
              :key="lang.id"
              class="py-2.5 px-2 rounded-xl border-2 text-xs font-medium transition-all"
              :class="currentLocale === lang.id
                ? 'border-primary-500 bg-primary-50 text-primary-700'
                : 'border-border-medium text-text-sub hover:border-primary-300'"
              @click="currentLocale = lang.id; changeLocale()"
            >
              {{ lang.name }}
            </button>
          </div>
        </div>

        <!-- Theme -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-2">{{ t('settings.theme') }}</label>
          <!-- Light themes -->
          <p class="text-xs text-text-sub mb-1.5">{{ t('settings.lightThemes') }}</p>
          <div class="grid grid-cols-3 gap-2 mb-3">
            <button
              v-for="th in lightThemes"
              :key="th.id"
              class="py-2.5 px-2 rounded-xl border-2 text-xs font-medium transition-all"
              :class="store.settings.theme === th.id
                ? 'border-primary-500 bg-primary-50 text-primary-700'
                : 'border-border-medium text-text-sub hover:border-primary-300'"
              @click="store.saveTheme(th.id)"
            >
              {{ th.icon }} {{ t(th.nameKey) }}
            </button>
          </div>
          <!-- Dark themes -->
          <p class="text-xs text-text-sub mb-1.5">{{ t('settings.darkThemes') }}</p>
          <div class="grid grid-cols-3 gap-2">
            <button
              v-for="th in darkThemes"
              :key="th.id"
              class="py-2.5 px-2 rounded-xl border-2 text-xs font-medium transition-all"
              :class="store.settings.theme === th.id
                ? 'border-primary-500 bg-primary-50 text-primary-700'
                : 'border-border-medium text-text-sub hover:border-primary-300'"
              @click="store.saveTheme(th.id)"
            >
              {{ th.icon }} {{ t(th.nameKey) }}
            </button>
          </div>
        </div>

        <!-- Custom Images -->
        <div class="border-t border-border-light pt-4 space-y-3">
          <label class="block text-sm font-medium text-text-main">{{ t('settings.customImages') }}</label>

          <div class="flex items-center gap-2">
            <span class="text-xs text-text-sub w-20 shrink-0">{{ t('settings.banner') }}</span>
            <span class="flex-1 text-xs text-text-sub truncate">{{ bannerPath || t('settings.notSet') }}</span>
            <button
              class="px-3 py-1.5 text-xs rounded-lg border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors"
              @click="selectCustomImage('custom_banner')"
            >
              {{ t('settings.selectImage') }}
            </button>
            <button
              v-if="bannerPath"
              class="px-2 py-1.5 text-xs rounded-lg text-red-400 hover:bg-red-50 transition-colors"
              @click="clearCustomImage('custom_banner')"
            >
              ✕
            </button>
          </div>

          <div class="flex items-center gap-2">
            <span class="text-xs text-text-sub w-20 shrink-0">{{ t('settings.sidebarBg') }}</span>
            <span class="flex-1 text-xs text-text-sub truncate">{{ sidebarBgPath || t('settings.notSet') }}</span>
            <button
              class="px-3 py-1.5 text-xs rounded-lg border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors"
              @click="selectCustomImage('custom_sidebar_bg')"
            >
              {{ t('settings.selectImage') }}
            </button>
            <button
              v-if="sidebarBgPath"
              class="px-2 py-1.5 text-xs rounded-lg text-red-400 hover:bg-red-50 transition-colors"
              @click="clearCustomImage('custom_sidebar_bg')"
            >
              ✕
            </button>
          </div>

          <!-- Banner blur & brightness -->
          <div class="flex items-center gap-3">
            <span class="text-xs text-text-sub w-20 shrink-0">{{ t('settings.bannerBlur') }}</span>
            <input
              type="range"
              v-model.number="bannerBlur"
              min="0"
              max="20"
              step="1"
              class="flex-1 accent-primary-500"
              @change="saveImageSetting('banner_blur', bannerBlur)"
            />
            <span class="text-xs text-text-sub w-10 text-right shrink-0">{{ bannerBlur }}px</span>
          </div>
          <div class="flex items-center gap-3">
            <span class="text-xs text-text-sub w-20 shrink-0">{{ t('settings.bannerBrightness') }}</span>
            <input
              type="range"
              v-model.number="bannerBrightness"
              min="30"
              max="150"
              step="5"
              class="flex-1 accent-primary-500"
              @change="saveImageSetting('banner_brightness', bannerBrightness)"
            />
            <span class="text-xs text-text-sub w-10 text-right shrink-0">{{ bannerBrightness }}%</span>
          </div>
          <!-- Sidebar blur & brightness -->
          <div class="flex items-center gap-3">
            <span class="text-xs text-text-sub w-20 shrink-0">{{ t('settings.sidebarBlur') }}</span>
            <input
              type="range"
              v-model.number="sidebarBlur"
              min="0"
              max="20"
              step="1"
              class="flex-1 accent-primary-500"
              @change="saveImageSetting('sidebar_blur', sidebarBlur)"
            />
            <span class="text-xs text-text-sub w-10 text-right shrink-0">{{ sidebarBlur }}px</span>
          </div>
          <div class="flex items-center gap-3">
            <span class="text-xs text-text-sub w-20 shrink-0">{{ t('settings.sidebarBrightness') }}</span>
            <input
              type="range"
              v-model.number="sidebarBrightness"
              min="30"
              max="150"
              step="5"
              class="flex-1 accent-primary-500"
              @change="saveImageSetting('sidebar_brightness', sidebarBrightness)"
            />
            <span class="text-xs text-text-sub w-10 text-right shrink-0">{{ sidebarBrightness }}%</span>
          </div>
        </div>

        <!-- Close Behavior -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-2">{{ t('settings.closeBehavior') }}</label>
          <div class="grid grid-cols-3 gap-2">
            <button
              v-for="opt in closeBehaviorOptions"
              :key="opt.id"
              class="py-2.5 px-2 rounded-xl border-2 text-xs font-medium transition-all"
              :class="(store.settings.close_behavior || 'ask') === opt.id
                ? 'border-primary-500 bg-primary-50 text-primary-700'
                : 'border-border-medium text-text-sub hover:border-primary-300'"
              @click="store.saveCloseBehavior(opt.id)"
            >
              {{ opt.icon }} {{ t(opt.nameKey) }}
            </button>
          </div>
        </div>

        <!-- 7z Path -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('settings.sevenZip') }}</label>
          <div class="flex gap-2">
            <input
              v-model="sevenZipPath"
              class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
              :placeholder="t('settings.sevenZipPlaceholder')"
              @change="savePaths"
            />
            <button
              class="px-4 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors shrink-0"
              @click="selectSevenZip"
            >
              {{ t('settings.browse') }}
            </button>
            <button
              class="px-4 py-2.5 text-sm rounded-xl bg-primary-500 text-white hover:bg-primary-600 transition-colors shrink-0"
              @click="testSevenZip"
            >
              {{ t('settings.test') }}
            </button>
          </div>
          <p v-if="sevenZipValid === true" class="text-xs text-green-500 mt-1">{{ t('settings.sevenZipOk') }}</p>
          <p v-else-if="sevenZipValid === false" class="text-xs text-red-500 mt-1">{{ t('settings.sevenZipFail') }}</p>
        </div>

        <!-- Tag Management -->
        <div class="border-t border-border-light pt-4">
          <div class="flex items-center justify-between mb-2">
            <label class="block text-sm font-medium text-text-main">{{ t('settings.tagManagement') }}</label>
            <button
              class="flex items-center gap-1 px-2 py-1 rounded-lg text-xs text-text-sub hover:bg-primary-50 hover:text-text-main transition-colors"
              @click="showTagList = !showTagList"
            >
              <span class="inline-block transition-transform duration-200" :class="showTagList ? 'rotate-90' : ''">▶</span>
              {{ showTagList ? t('settings.collapseList') : t('settings.expandList') }}
              <span v-if="tagUsages.length > 0" class="text-[10px] bg-primary-100 text-primary-600 px-1.5 py-0.5 rounded-md">{{ tagUsages.length }}</span>
            </button>
          </div>
          <div v-if="showTagList" class="space-y-1.5">
            <!-- Add new tag -->
            <div class="flex gap-2 mb-2">
              <input
                v-model="newTagName"
                class="flex-1 px-3 py-2 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
                :placeholder="t('settings.newTagPlaceholder')"
                @keyup.enter="addNewTag"
              />
              <button
                class="px-3 py-2 text-sm rounded-xl bg-primary-500 text-white hover:bg-primary-600 transition-colors shrink-0"
                @click="addNewTag"
              >
                {{ t('settings.addTag') }}
              </button>
            </div>
            <div
              v-for="tag in tagUsages"
              :key="tag.id"
              class="flex items-center gap-2 px-3 py-2 rounded-xl bg-primary-50 dark:bg-primary-900/20"
            >
              <template v-if="editingTagId === tag.id">
                <input
                  v-model="editingTagName"
                  class="flex-1 px-2 py-1 text-sm rounded-lg border border-primary-300 bg-input-bg text-text-main outline-none focus:border-primary-400 min-w-0"
                  @keyup.enter="confirmRenameTag"
                  @keyup.escape="cancelRenameTag"
                  @blur="confirmRenameTag"
                />
                <button
                  class="text-xs text-green-500 hover:text-green-600 transition-colors shrink-0"
                  @mousedown.prevent="confirmRenameTag"
                >
                  ✓
                </button>
                <button
                  class="text-xs text-text-sub hover:text-text-main transition-colors shrink-0"
                  @mousedown.prevent="cancelRenameTag"
                >
                  ✕
                </button>
              </template>
              <template v-else>
                <span class="flex-1 text-sm text-text-main truncate">{{ tag.name }}</span>
                <span class="text-[10px] text-text-sub shrink-0">🎮 {{ tag.game_count }} · 🧩 {{ tag.mod_count }}</span>
                <button
                  class="p-1 rounded-lg text-sm text-text-sub hover:text-primary-500 hover:bg-primary-100 transition-colors shrink-0"
                  :title="t('common.rename')"
                  @click="startRenameTag(tag)"
                >
                  ✏️
                </button>
                <button
                  class="p-1 rounded-lg text-sm text-red-400 hover:text-red-500 hover:bg-red-50 transition-colors shrink-0"
                  :title="t('common.delete')"
                  @click="deletingTag = tag"
                >
                  🗑️
                </button>
              </template>
            </div>
            <p v-if="tagUsages.length === 0" class="text-xs text-text-sub italic">
              {{ t('settings.noTagsYet') }}
            </p>
          </div>
        </div>

        <!-- Backup -->
        <div class="border-t border-border-light pt-4 space-y-3">
          <label class="block text-sm font-medium text-text-main">{{ t('settings.backup') }}</label>
          <div class="flex gap-3">
            <button
              class="flex-1 py-2.5 rounded-xl border border-primary-200 text-sm text-text-sub hover:bg-primary-50 transition-colors"
              @click="doExport"
            >
              📤 {{ t('settings.exportBackup') }}
            </button>
            <button
              class="flex-1 py-2.5 rounded-xl border border-primary-200 text-sm text-text-sub hover:bg-primary-50 transition-colors"
              :disabled="importing"
              @click="doImport"
            >
              {{ importing ? t('settings.importing') : '📥 ' + t('settings.importBackup') }}
            </button>
          </div>
          <p class="text-xs text-text-sub">{{ t('settings.backupNote') }}</p>
          <div class="flex gap-3 pt-2">
            <button
              class="flex-1 py-2.5 rounded-xl border border-primary-200 text-sm text-text-sub hover:bg-primary-50 transition-colors"
              @click="doBackupDatabase"
            >
              💾 {{ t('settings.dbBackup') }}
            </button>
          </div>
          <p class="text-xs text-text-sub">{{ t('settings.dbBackupNote') }}</p>
          <label class="flex items-center gap-2 text-sm text-text-sub cursor-pointer pt-1">
            <input
              type="checkbox"
              v-model="autoBackup"
              class="rounded accent-primary-500"
              @change="saveAutoBackup"
            />
            {{ t('settings.autoBackup') }}
          </label>
          <p class="text-xs text-text-sub">{{ t('settings.autoBackupNote') }}</p>
        </div>

        <!-- Update -->
        <div class="border-t border-border-light pt-4 space-y-3">
          <label class="block text-sm font-medium text-text-main">{{ t('settings.update') }}</label>
          <div class="flex gap-2">
            <button
              class="px-4 py-2.5 text-sm rounded-xl bg-primary-500 text-white hover:bg-primary-600 transition-colors shrink-0"
              :disabled="checkingUpdate"
              @click="doCheckUpdate"
            >
              {{ checkingUpdate ? t('settings.checkingUpdate') : t('settings.checkUpdate') }}
            </button>
          </div>
          <p v-if="updateError" class="text-xs text-red-500">✗ {{ updateError }}</p>
          <div v-if="updateInfo" class="px-4 py-3 rounded-xl bg-code-bg space-y-1.5">
            <p class="text-xs text-text-sub">{{ t('settings.currentVersion', { version: updateInfo.current_version }) }}</p>
            <p v-if="updateInfo.available" class="text-sm text-primary-500 font-medium">
              {{ t('settings.newVersion', { version: updateInfo.latest_version }) }}
              <button
                v-if="updateInfo.release_url"
                class="ml-2 text-xs underline text-primary-400 hover:text-primary-600"
                @click="openUrl(updateInfo.release_url)"
              >
                {{ t('settings.viewRelease') }}
              </button>
            </p>
            <p v-else class="text-sm text-text-main">{{ t('settings.upToDate') }}</p>
            <p v-if="updateInfo.release_notes && updateInfo.available" class="text-xs text-text-sub whitespace-pre-line mt-1">{{ updateInfo.release_notes.slice(0, 300) }}</p>
          </div>
        </div>

        <!-- Keyboard Shortcuts -->
        <div class="border-t border-border-light pt-4">
          <label class="block text-sm font-medium text-text-main mb-3">{{ t('settings.shortcuts') }}</label>
          <div class="grid grid-cols-2 gap-2">
            <div class="flex items-center justify-between px-3 py-2 rounded-xl bg-code-bg">
              <span class="text-xs text-text-sub">{{ t('settings.shortcutLaunch') }}</span>
              <kbd class="px-2 py-0.5 rounded-md bg-input-bg text-text-main text-xs font-mono border border-border-medium">Enter / Space</kbd>
            </div>
            <div class="flex items-center justify-between px-3 py-2 rounded-xl bg-code-bg">
              <span class="text-xs text-text-sub">{{ t('settings.shortcutDelete') }}</span>
              <kbd class="px-2 py-0.5 rounded-md bg-input-bg text-text-main text-xs font-mono border border-border-medium">Delete</kbd>
            </div>
            <div class="flex items-center justify-between px-3 py-2 rounded-xl bg-code-bg">
              <span class="text-xs text-text-sub">{{ t('settings.shortcutClose') }}</span>
              <kbd class="px-2 py-0.5 rounded-md bg-input-bg text-text-main text-xs font-mono border border-border-medium">Esc</kbd>
            </div>
            <div class="flex items-center justify-between px-3 py-2 rounded-xl bg-code-bg">
              <span class="text-xs text-text-sub">{{ t('settings.shortcutSearch') }}</span>
              <kbd class="px-2 py-0.5 rounded-md bg-input-bg text-text-main text-xs font-mono border border-border-medium">Ctrl+F</kbd>
            </div>
            <div class="flex items-center justify-between px-3 py-2 rounded-xl bg-code-bg">
              <span class="text-xs text-text-sub">{{ t('settings.shortcutSettings') }}</span>
              <kbd class="px-2 py-0.5 rounded-md bg-input-bg text-text-main text-xs font-mono border border-border-medium">Ctrl+,</kbd>
            </div>
            <div class="flex items-center justify-between px-3 py-2 rounded-xl bg-code-bg">
              <span class="text-xs text-text-sub">{{ t('settings.shortcutDblClick') }}</span>
              <span class="text-xs text-text-main">{{ t('settings.shortcutDblClickAction') }}</span>
            </div>
          </div>
        </div>
        </template>

        <!-- ===== Games Tab ===== -->
        <template v-if="activeTab === 'games'">
        <!-- Default Extract Path -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('settings.extractPath') }}</label>
          <div class="flex gap-2">
            <input
              v-model="defaultExtractPath"
              class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
              :placeholder="t('settings.extractPathPlaceholder')"
              @change="savePaths"
            />
            <button
              class="px-4 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors shrink-0"
              @click="selectExtractPath"
            >
              {{ t('settings.browse') }}
            </button>
          </div>
        </div>

        <!-- Passwords -->
        <div class="border-t border-border-light pt-4">
          <div class="flex items-center justify-between mb-2">
            <label class="block text-sm font-medium text-text-main">{{ t('settings.passwords') }}</label>
            <button
              class="flex items-center gap-1 px-2 py-1 rounded-lg text-xs text-text-sub hover:bg-primary-50 hover:text-text-main transition-colors"
              @click="showPwdList = !showPwdList"
            >
              <span class="inline-block transition-transform duration-200" :class="showPwdList ? 'rotate-90' : ''">▶</span>
              {{ showPwdList ? t('settings.collapseList') : t('settings.expandList') }}
              <span v-if="store.passwords.length > 0" class="text-[10px] bg-primary-100 text-primary-600 px-1.5 py-0.5 rounded-md">{{ store.passwords.length }}</span>
            </button>
          </div>
          <div class="flex gap-2 mb-3">
            <input
              v-model="newPassword"
              class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
              :placeholder="t('settings.passwordPlaceholder')"
              @keyup.enter="addPwd"
            />
            <button
              class="px-4 py-2.5 text-sm rounded-xl bg-sakura-400 text-white hover:bg-sakura-500 transition-colors shrink-0"
              @click="addPwd"
            >
              {{ t('settings.addPassword') }}
            </button>
          </div>
          <div v-if="showPwdList" class="space-y-1.5">
            <div
              v-for="pwd in store.passwords"
              :key="pwd"
              class="flex items-center justify-between px-3 py-2 rounded-xl bg-primary-50 text-sm"
            >
              <span class="text-text-main font-mono text-xs">{{ pwd }}</span>
              <button
                class="text-xs text-red-400 hover:text-red-500 transition-colors"
                @click="store.removePassword(pwd)"
              >
                {{ t('common.delete') }}
              </button>
            </div>
            <p v-if="store.passwords.length === 0" class="text-xs text-text-sub italic">
              {{ t('settings.noPasswords') }}
            </p>
          </div>
        </div>

        <!-- IGDB -->
        <div class="space-y-3">
          <label class="block text-sm font-medium text-text-main">{{ t('settings.igdb') }}</label>
          <p class="text-xs text-text-sub">{{ t('settings.igdbHint') }}</p>
          <div class="flex gap-2">
            <input
              v-model="igdbClientId"
              class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
              :placeholder="t('settings.igdbClientId')"
            />
          </div>
          <div class="flex gap-2">
            <input
              v-model="igdbClientSecret"
              type="password"
              class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
              :placeholder="t('settings.igdbClientSecret')"
            />
            <button
              class="px-4 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors shrink-0"
              @click="saveIgdb"
            >
              {{ t('settings.save') }}
            </button>
          </div>
        </div>

        <!-- Cover Integrity -->
        <div class="border-t border-border-light pt-4 space-y-3">
          <label class="block text-sm font-medium text-text-main">{{ t('settings.coverManagement') }}</label>
          <button
            class="w-full py-2.5 rounded-xl border border-primary-200 text-sm text-text-sub hover:bg-primary-50 transition-colors"
            :disabled="checkingCovers"
            @click="checkCoverIntegrity"
          >
            {{ checkingCovers ? t('settings.checking') : t('settings.checkCover') }}
          </button>
          <div v-if="coverResults.length > 0" class="space-y-1.5 max-h-48 overflow-auto">
            <div
              v-for="item in coverResults.filter(r => !r.exists)"
              :key="item.game_id"
              class="flex items-center justify-between px-3 py-2 rounded-xl bg-red-50 border border-red-200/50"
            >
              <span class="text-xs text-red-600 truncate flex-1 mr-2">{{ item.game_name }}</span>
              <button
                class="px-2.5 py-1 text-xs rounded-lg bg-primary-500 text-white hover:bg-primary-600 transition-colors shrink-0"
                :disabled="rescanningId === item.game_id"
                @click="rescanCover(item.game_id)"
              >
                {{ rescanningId === item.game_id ? t('settings.rescanning') : t('settings.rescan') }}
              </button>
            </div>
            <p v-if="coverResults.filter(r => !r.exists).length === 0" class="text-xs text-green-500">
              {{ t('settings.allCoversOk') }}
            </p>
          </div>
          <p class="text-xs text-text-sub">{{ t('settings.coverNote') }}</p>
          <button
            class="w-full py-2.5 rounded-xl border border-primary-200 text-sm text-text-sub hover:bg-primary-50 transition-colors"
            @click="emit('openIntegrity'); emit('close')"
          >
            🩺 {{ t('settings.integrityCheckup') }}
          </button>
        </div>
        </template>

        <!-- ===== Mods Tab ===== -->
        <template v-if="activeTab === 'mods'">
        <div class="flex flex-col items-center justify-center py-16 text-text-sub">
          <div class="text-5xl mb-4 opacity-30">🧩</div>
          <p class="text-sm">{{ t('settings.noModSettings') }}</p>
        </div>
        </template>
      </div>
    </div>
    <transition name="modal">
      <ConfirmDialog
        v-if="deletingTag"
        :title="t('settings.deleteTagTitle')"
        :message="t('settings.deleteTagConfirm', { name: deletingTag.name })"
        :confirm-text="t('common.delete')"
        :danger="true"
        @confirm="confirmDeleteTag"
        @cancel="deletingTag = null"
      />
    </transition>
  </div>
</template>

<style scoped>
.settings-scroll::-webkit-scrollbar {
  display: none;
}
</style>
