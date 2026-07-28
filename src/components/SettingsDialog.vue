<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { open as openUrl } from "@tauri-apps/plugin-shell";
import { useGameStore } from "../stores/gameStore";
import { useI18n } from "vue-i18n";
import type { UpdateInfo } from "../types";

const { locale, t } = useI18n();

const emit = defineEmits<{
  close: [];
}>();

const store = useGameStore();

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
const sevenZipValid = ref<boolean | null>(null);
const saveMsg = ref<{ text: string; ok: boolean } | null>(null);
let saveMsgTimer: ReturnType<typeof setTimeout> | null = null;

// Custom images
const bannerPath = ref("");
const sidebarBgPath = ref("");
const emptyIllustPath = ref("");

onMounted(() => {
  sevenZipPath.value = store.settings.seven_zip_path;
  defaultExtractPath.value = store.settings.default_extract_path;
  bannerPath.value = store.settings.custom_banner;
  sidebarBgPath.value = store.settings.custom_sidebar_bg;
  emptyIllustPath.value = store.settings.custom_empty_illustration;
  updateRepo.value = store.settings.update_repo;
  igdbClientId.value = store.settings.igdb_client_id;
  igdbClientSecret.value = store.settings.igdb_client_secret;
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
  }
}

async function selectExtractPath() {
  const path = await open({ directory: true, multiple: false });
  if (path) {
    defaultExtractPath.value = path as string;
  }
}

async function testSevenZip() {
  if (!sevenZipPath.value) return;
  sevenZipValid.value = await invoke<boolean>("test_seven_zip", {
    path: sevenZipPath.value,
  });
}

async function saveAll() {
  try {
    await store.saveSettings(sevenZipPath.value, defaultExtractPath.value);
    showSaveMsg(t('settings.saveSuccess'), true);
  } catch (e) {
    showSaveMsg(t('settings.saveFail') + ': ' + (e as string), false);
  }
}

function showSaveMsg(text: string, ok: boolean) {
  saveMsg.value = { text, ok };
  if (saveMsgTimer) clearTimeout(saveMsgTimer);
  saveMsgTimer = setTimeout(() => { saveMsg.value = null; }, 2500);
}

async function addPwd() {
  const pwd = newPassword.value.trim();
  if (!pwd) return;
  await store.addPassword(pwd);
  newPassword.value = "";
}

type ImageKey = "custom_banner" | "custom_sidebar_bg" | "custom_empty_illustration";

const imageKeys: Record<ImageKey, { value: string }> = {
  custom_banner: bannerPath,
  custom_sidebar_bg: sidebarBgPath,
  custom_empty_illustration: emptyIllustPath,
};

async function selectCustomImage(key: ImageKey) {
  const path = await open({
    filters: [{ name: t('edit.coverImage'), extensions: ["jpg", "jpeg", "png", "webp", "gif"] }],
    multiple: false,
    directory: false,
  });
  if (path) {
    try {
      imageKeys[key].value = path as string;
      await store.saveCustomImage(key, path as string);
      showSaveMsg(t('settings.imageSaved'), true);
    } catch (e) {
      showSaveMsg(t('settings.saveFail') + ': ' + (e as string), false);
    }
  }
}

async function clearCustomImage(key: ImageKey) {
  try {
    imageKeys[key].value = "";
    await store.saveCustomImage(key, "");
    showSaveMsg(t('settings.imageCleared'), true);
  } catch (e) {
    showSaveMsg(t('settings.saveFail') + ': ' + (e as string), false);
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
    showSaveMsg(t('settings.checkFail') + ': ' + (e as string), false);
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
    showSaveMsg(t('settings.coverUpdated'), true);
  } catch (e) {
    showSaveMsg(t('settings.saveFail') + ': ' + (e as string), false);
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
      showSaveMsg(t('settings.exportSuccess'), true);
    }
  } catch (e) {
    showSaveMsg(t('settings.saveFail') + ': ' + (e as string), false);
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
    showSaveMsg(t('settings.importSuccess'), true);
  } catch (e) {
    showSaveMsg(t('settings.saveFail') + ': ' + (e as string), false);
  } finally {
    importing.value = false;
  }
}

async function doBackupDatabase() {
  try {
    const path = await store.backupDatabase();
    showSaveMsg(t('settings.backupSuccess', { path }), true);
  } catch (e) {
    showSaveMsg(t('settings.backupFail') + ': ' + (e as string), false);
  }
}

async function doSaveUpdateRepo() {
  await store.saveUpdateRepo(updateRepo.value.trim());
  showSaveMsg(t('settings.updateSaved'), true);
}

async function saveIgdb() {
  await invoke("save_setting", { key: "igdb_client_id", value: igdbClientId.value.trim() });
  await invoke("save_setting", { key: "igdb_client_secret", value: igdbClientSecret.value.trim() });
  store.settings.igdb_client_id = igdbClientId.value.trim();
  store.settings.igdb_client_secret = igdbClientSecret.value.trim();
  showSaveMsg(t('settings.saveSuccess'), true);
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

      <div class="settings-scroll px-8 py-8 space-y-8 overflow-auto flex-1" style="scrollbar-width: none;">
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

        <!-- Default Extract Path -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('settings.extractPath') }}</label>
          <div class="flex gap-2">
            <input
              v-model="defaultExtractPath"
              class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
              :placeholder="t('settings.extractPathPlaceholder')"
            />
            <button
              class="px-4 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors shrink-0"
              @click="selectExtractPath"
            >
              {{ t('settings.browse') }}
            </button>
          </div>
        </div>

        <!-- Save Settings -->
        <div>
          <button
            class="w-full py-2.5 rounded-xl bg-primary-500 text-white text-sm font-medium hover:bg-primary-600 transition-colors shadow-sm"
            @click="saveAll"
          >
            💾 {{ t('settings.saveSettings') }}
          </button>
          <p v-if="saveMsg" :class="['text-xs mt-1.5 text-center transition-opacity', saveMsg.ok ? 'text-green-500' : 'text-red-500']">
            {{ saveMsg.ok ? '✓' : '✗' }} {{ saveMsg.text }}
          </p>
        </div>

        <!-- Passwords -->
        <div class="border-t border-border-light pt-4">
          <label class="block text-sm font-medium text-text-main mb-2">{{ t('settings.passwords') }}</label>
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
          <div class="space-y-1.5">
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

          <div class="flex items-center gap-2">
            <span class="text-xs text-text-sub w-20 shrink-0">{{ t('settings.emptyIllust') }}</span>
            <span class="flex-1 text-xs text-text-sub truncate">{{ emptyIllustPath || t('settings.notSet') }}</span>
            <button
              class="px-3 py-1.5 text-xs rounded-lg border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors"
              @click="selectCustomImage('custom_empty_illustration')"
            >
              {{ t('settings.selectImage') }}
            </button>
            <button
              v-if="emptyIllustPath"
              class="px-2 py-1.5 text-xs rounded-lg text-red-400 hover:bg-red-50 transition-colors"
              @click="clearCustomImage('custom_empty_illustration')"
            >
              ✕
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
        </div>

        <!-- Update -->
        <div class="border-t border-border-light pt-4 space-y-3">
          <label class="block text-sm font-medium text-text-main">{{ t('settings.update') }}</label>
          <div class="flex gap-2">
            <input
              v-model="updateRepo"
              class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
              :placeholder="t('settings.updateRepoPlaceholder')"
            />
            <button
              class="px-4 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors shrink-0"
              @click="doSaveUpdateRepo"
            >
              {{ t('settings.save') }}
            </button>
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

        <!-- IGDB -->
        <div class="border-t border-border-light pt-4 space-y-3">
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
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-scroll::-webkit-scrollbar {
  display: none;
}
</style>
