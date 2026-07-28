<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useGameStore } from "../stores/gameStore";
import { useI18n } from "vue-i18n";
import type { ExtractResult } from "../types";
import CustomSelect from "./CustomSelect.vue";

const { t } = useI18n();

const emit = defineEmits<{
  close: [];
}>();

const props = defineProps<{
  initialPath?: string;
}>();

const store = useGameStore();

type Step = "select" | "extract" | "confirm";
type ImportMode = "archive" | "local";

const step = ref<Step>("select");
const importMode = ref<ImportMode>("local");

// Archive mode
const archivePath = ref("");
const selectedPassword = ref("");
const useAutoPasswords = ref(true);
const customDestPath = ref("");
const extracting = ref(false);
const extractResult = ref<ExtractResult | null>(null);

// Local mode
const localDirPath = ref(props.initialPath || "");
const scanning = ref(false);

// Form for confirm step
const gameName = ref("");
const exePath = ref("");
const coverPath = ref("");
const extractDir = ref("");
const savePath = ref("");
const scriptPath = ref("");
const scriptArgs = ref("");
const installPath = ref("");
const coverCopying = ref(false);

const hasPasswords = computed(() => store.passwords.length > 0);
const passwordOptions = computed(() => [
  { label: t('import.noPassword'), value: "" },
  ...store.passwords.map((pwd) => ({ label: pwd, value: pwd })),
]);

async function selectArchive() {
  const path = await open({
    filters: [
      {
        name: t('import.title'),
        extensions: ["zip", "rar", "7z", "tar", "gz", "bz2", "xz"],
      },
    ],
    multiple: false,
    directory: false,
  });
  if (path) {
    archivePath.value = path as string;
  }
}

async function selectLocalDir() {
  const path = await open({
    directory: true,
    multiple: false,
  });
  if (path) {
    localDirPath.value = path as string;
  }
}

async function selectDestPath() {
  const path = await open({
    directory: true,
    multiple: false,
  });
  if (path) {
    customDestPath.value = path as string;
  }
}

async function scanLocalGame() {
  if (!localDirPath.value) return;
  scanning.value = true;
  try {
    const result = await invoke<ExtractResult>("scan_local_game", {
      dirPath: localDirPath.value,
    });
    gameName.value = result.detected_name;
    exePath.value = result.exe_path;
    coverPath.value = result.cover_path;
    extractDir.value = result.extract_dir;
    installPath.value = result.extract_dir;
    savePath.value = result.save_path || "";
    step.value = "confirm";
  } catch (e) {
    console.error(e);
  } finally {
    scanning.value = false;
  }
}

async function doExtract() {
  extracting.value = true;

  let password: string | undefined;
  if (selectedPassword.value) {
    password = selectedPassword.value;
  }

  // Try auto passwords if enabled
  if (useAutoPasswords.value && !password && store.passwords.length > 0) {
    for (const pwd of store.passwords) {
      const result = await invoke<ExtractResult>("extract_game", {
        archivePath: archivePath.value,
        destPath: customDestPath.value || null,
        password: pwd,
      });
      if (result.success) {
        extractResult.value = result;
        gameName.value = result.detected_name;
        exePath.value = result.exe_path;
        coverPath.value = result.cover_path;
        extractDir.value = result.extract_dir;
        installPath.value = result.extract_dir;
        savePath.value = result.save_path || "";
        extracting.value = false;
        step.value = "confirm";
        return;
      }
    }
    // All passwords failed, try without password
    const result = await invoke<ExtractResult>("extract_game", {
      archivePath: archivePath.value,
      destPath: customDestPath.value || null,
      password: null,
    });
    extractResult.value = result;
  } else {
    const result = await invoke<ExtractResult>("extract_game", {
      archivePath: archivePath.value,
      destPath: customDestPath.value || null,
      password: password || null,
    });
    extractResult.value = result;
  }

  extracting.value = false;

  if (extractResult.value?.success) {
    gameName.value = extractResult.value.detected_name;
    exePath.value = extractResult.value.exe_path;
    coverPath.value = extractResult.value.cover_path;
    extractDir.value = extractResult.value.extract_dir;
    installPath.value = extractResult.value.extract_dir;
    savePath.value = extractResult.value.save_path || "";
    step.value = "confirm";
  }
}

async function selectCoverForImport() {
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
        gameId: null,
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

async function selectScript() {
  const path = await open({
    filters: [{ name: t('edit.scriptPath'), extensions: ["bat", "cmd", "ps1", "sh", "py"] }],
    multiple: false,
    directory: false,
  });
  if (path) scriptPath.value = path as string;
}

async function selectInstallDir() {
  const path = await open({
    directory: true,
    multiple: false,
  });
  if (path) installPath.value = path as string;
}

async function confirmAdd() {
  const name = gameName.value || t('import.title');
  const dup = store.games.find((g) => g.name.toLowerCase() === name.toLowerCase());
  if (dup) {
    if (!confirm(t('import.duplicateWarning', { name: dup.name }))) return;
  }

  const gameId = await store.addGame({
    name,
    group_id: null,
    install_path: installPath.value || extractDir.value,
    exe_path: exePath.value,
    launch_args: "",
    cover_path: coverPath.value,
    save_path: savePath.value,
    notes: "",
    script_path: scriptPath.value,
    script_args: scriptArgs.value,
    status: "not_played",
    rating: 0,
    sort_order: 0,
  });

  // Auto-scan cover and save after adding
  try {
    const [scannedCover, scannedSave] = await Promise.all([
      invoke<string>("scan_game_cover", { id: gameId }),
      invoke<string>("scan_game_save", { id: gameId }),
    ]);
    if (scannedCover || scannedSave) {
      await store.loadGames();
    }
  } catch (e) {
    console.error("自动扫描失败:", e);
  }

  emit("close");
}

function goBack() {
  step.value = "select";
  extractResult.value = null;
  archivePath.value = "";
  localDirPath.value = "";
  customDestPath.value = "";
  selectedPassword.value = "";
  savePath.value = "";
  installPath.value = "";
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
        <h2 class="text-lg font-bold text-text-main">
          {{ step === "select" ? "✨ " + t('import.title') : step === "extract" ? t('import.extracting') : t('import.confirmTitle') }}
        </h2>
        <button
          class="p-1.5 rounded-lg hover:bg-primary-50 text-text-sub transition-colors"
          @click="emit('close')"
        >
          ✕
        </button>
      </div>

      <!-- Step 1: Select Source -->
      <div v-if="step === 'select'" class="px-8 py-8 space-y-7">
        <!-- Import Mode Toggle -->
        <div class="flex gap-2 p-1 bg-primary-50 rounded-xl">
          <button
            class="flex-1 py-2 text-sm rounded-lg transition-all font-medium"
            :class="
              importMode === 'local'
                ? 'bg-white-solid text-primary-600 shadow-sm'
                : 'text-text-sub hover:text-text-main'
            "
            @click="importMode = 'local'"
          >
            📁 {{ t('import.localGame') }}
          </button>
          <button
            class="flex-1 py-2 text-sm rounded-lg transition-all font-medium"
            :class="
              importMode === 'archive'
                ? 'bg-white-solid text-primary-600 shadow-sm'
                : 'text-text-sub hover:text-text-main'
            "
            @click="importMode = 'archive'"
          >
            📦 {{ t('import.archive') }}
          </button>
        </div>

        <!-- Local Mode: Select Directory -->
        <template v-if="importMode === 'local'">
          <div>
            <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('import.selectGameDir') }}</label>
            <div class="flex gap-2">
              <input
                :value="localDirPath"
                readonly
                :placeholder="t('import.selectGameDir')"
                class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 bg-input-bg outline-none truncate cursor-pointer hover:border-primary-300 transition-colors"
                @click="selectLocalDir"
              />
              <button
                class="px-4 py-2.5 text-sm rounded-xl bg-primary-500 text-white hover:bg-primary-600 transition-colors shrink-0"
                @click="selectLocalDir"
              >
                {{ t('import.browse') }}
              </button>
            </div>
            <p class="text-xs text-text-sub mt-1.5">
              {{ t('import.selectDirHint') }}
            </p>
          </div>

          <button
            :disabled="!localDirPath || scanning"
            class="w-full py-3 rounded-xl bg-gradient-to-r from-sakura-400 to-sakura-500 text-white font-medium hover:from-sakura-500 hover:to-sakura-500 transition-all disabled:opacity-50 disabled:cursor-not-allowed shadow-sm"
            @click="scanLocalGame"
          >
            {{ scanning ? t('import.scanning') : t('import.scanGame') }}
          </button>
        </template>

        <!-- Archive Mode -->
        <template v-if="importMode === 'archive'">
          <div>
            <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('import.selectArchive') }}</label>
            <div class="flex gap-2">
              <input
                :value="archivePath"
                readonly
                :placeholder="t('import.selectArchiveHint')"
                class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 bg-input-bg outline-none truncate cursor-pointer hover:border-primary-300 transition-colors"
                @click="selectArchive"
              />
              <button
                class="px-4 py-2.5 text-sm rounded-xl bg-primary-500 text-white hover:bg-primary-600 transition-colors shrink-0"
                @click="selectArchive"
              >
                {{ t('import.browse') }}
              </button>
            </div>
          </div>

          <div>
            <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('import.password') }}</label>
            <div class="space-y-2">
              <label v-if="hasPasswords" class="flex items-center gap-2 text-sm">
                <input type="checkbox" v-model="useAutoPasswords" class="rounded accent-primary-500" />
                <span class="text-text-sub">{{ t('import.autoPassword', { count: store.passwords.length }) }}</span>
              </label>
              <CustomSelect v-model="selectedPassword" :options="passwordOptions" :placeholder="t('import.noPassword')" />
            </div>
          </div>

          <div>
            <label class="block text-sm font-medium text-text-main mb-1.5">
              {{ t('import.extractTo') }} <span class="text-text-sub font-normal">({{ t('import.extractToHint') }})</span>
            </label>
            <div class="flex gap-2">
              <input
                v-model="customDestPath"
                readonly
                :placeholder="t('import.useDefaultPath')"
                class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 bg-input-bg outline-none truncate cursor-pointer hover:border-primary-300 transition-colors"
                @click="selectDestPath"
              />
              <button
                class="px-4 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors shrink-0"
                @click="selectDestPath"
              >
                {{ t('import.browse') }}
              </button>
            </div>
          </div>

          <button
            :disabled="!archivePath || extracting"
            class="w-full py-3 rounded-xl bg-gradient-to-r from-sakura-400 to-sakura-500 text-white font-medium hover:from-sakura-500 hover:to-sakura-500 transition-all disabled:opacity-50 disabled:cursor-not-allowed shadow-sm"
            @click="doExtract"
          >
            {{ extracting ? t('import.extracting') : t('import.startExtract') }}
          </button>
        </template>
      </div>

      <!-- Extract Error -->
      <div v-if="step === 'select' && extractResult && !extractResult.success" class="px-8 py-6">
        <div class="p-4 rounded-xl bg-red-50 border border-red-200">
          <p class="text-sm text-red-600 font-medium">{{ t('import.extractFailed') }}</p>
          <p class="text-xs text-red-500 mt-1">{{ extractResult.error }}</p>
        </div>
        <button
          class="w-full mt-3 py-2 rounded-xl border border-primary-200 text-sm text-text-sub hover:bg-primary-50 transition-colors"
          @click="goBack"
        >
          {{ t('import.retry') }}
        </button>
      </div>

      <!-- Step 2: Confirm Game Info -->
      <div v-if="step === 'confirm'" class="px-8 py-8 space-y-6">
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('import.gameName') }}</label>
          <input
            v-model="gameName"
            class="w-full px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors"
            :placeholder="t('import.gameNamePlaceholder')"
          />
        </div>

        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('import.exeFile') }}</label>
          <input
            v-model="exePath"
            class="w-full px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors"
            :placeholder="t('import.exePlaceholder')"
          />
        </div>

        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('import.coverPath') }}</label>
          <div class="flex gap-2">
            <input
              v-model="coverPath"
              class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
              :placeholder="t('import.coverAuto')"
            />
            <button
              class="px-4 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors shrink-0"
              :disabled="coverCopying"
              @click="selectCoverForImport"
            >
              {{ coverCopying ? t('import.copying') : t('import.browse') }}
            </button>
          </div>
        </div>

        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">
            {{ t('import.scriptPath') }} <span class="text-text-sub font-normal">({{ t('import.scriptOptional') }})</span>
          </label>
          <div class="flex gap-2">
            <input
              v-model="scriptPath"
              class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
              :placeholder="t('import.scriptPlaceholder')"
            />
            <button
              class="px-4 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 transition-colors shrink-0"
              @click="selectScript"
            >
              {{ t('import.browse') }}
            </button>
          </div>
        </div>

        <div v-if="scriptPath">
          <label class="block text-sm font-medium text-text-main mb-1.5">
            {{ t('import.scriptArgs') }} <span class="text-text-sub font-normal">({{ t('import.scriptOptional') }})</span>
          </label>
          <input
            v-model="scriptArgs"
            class="w-full px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors font-mono"
            :placeholder="t('import.scriptArgsPlaceholder')"
          />
        </div>

        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">
            {{ t('import.installDir') }} <span class="text-text-sub font-normal">({{ t('import.installDirModifiable') }})</span>
          </label>
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
              {{ t('import.browse') }}
            </button>
          </div>
        </div>

        <div class="p-3 rounded-xl bg-primary-50 text-xs text-text-sub space-y-1">
          <p v-if="savePath">{{ t('import.saveLocation', { path: savePath }) }}</p>
        </div>

        <div class="flex gap-3 pt-2">
          <button
            class="flex-1 py-2.5 rounded-xl border border-primary-200 text-sm text-text-sub hover:bg-primary-50 transition-colors"
            @click="goBack"
          >
            {{ t('import.back') }}
          </button>
          <button
            class="flex-1 py-2.5 rounded-xl bg-gradient-to-r from-primary-500 to-primary-600 text-white text-sm font-medium hover:from-primary-600 hover:to-primary-700 transition-all shadow-sm"
            @click="confirmAdd"
          >
            {{ t('import.confirmAdd') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
