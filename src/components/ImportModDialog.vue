<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useModStore } from "../stores/modStore";
import { useGameStore } from "../stores/gameStore";
import { useI18n } from "vue-i18n";
import { addToast } from "../composables/useToast";
import CustomSelect from "./CustomSelect.vue";
import type { ScannedMod } from "../types";

const { t } = useI18n();

const emit = defineEmits<{
  close: [];
  imported: [];
}>();

const modStore = useModStore();
const gameStore = useGameStore();

// Selected files
const selectedFiles = ref<string[]>([]);
// Linked game
const selectedGameId = ref<number | null>(null);
// Mod directory (target)
const modDir = ref("");
// Processing state
const processing = ref(false);
const processingText = ref("");

const gameOptions = computed(() => [
  { label: t("mod.noGame"), value: null as number | null },
  ...gameStore.games.map((g) => ({ label: g.name, value: g.id as number | null })),
]);

const fileNames = computed(() =>
  selectedFiles.value.map((f) => f.replace(/^.*[\\/]/, ""))
);

const canImport = computed(() =>
  selectedFiles.value.length > 0 && modDir.value.trim() !== "" && !processing.value
);

// Auto-fill mod directory when game is selected
watch(selectedGameId, (newGameId) => {
  if (newGameId !== null) {
    const game = gameStore.games.find((g) => g.id === newGameId);
    if (game && game.install_path) {
      const defaultDir = game.default_mod_dir;
      if (defaultDir) {
        const isAbsolute = /^([A-Za-z]:[\\/]|\\\\|\/)/.test(defaultDir);
        modDir.value = isAbsolute
          ? defaultDir
          : `${game.install_path.replace(/[\\/]+$/, "")}\\${defaultDir}`;
      } else {
        modDir.value = `${game.install_path.replace(/[\\/]+$/, "")}\\Mods`;
      }
    }
  }
});

async function selectFiles() {
  const paths = await open({
    multiple: true,
    directory: false,
    filters: [
      { name: "Mod Files", extensions: ["pak", "zip", "7z", "rar"] },
    ],
  });
  if (paths) {
    const arr = Array.isArray(paths) ? paths : [paths];
    selectedFiles.value = [...selectedFiles.value, ...(arr as string[])];
  }
}

function removeFile(index: number) {
  selectedFiles.value.splice(index, 1);
}

async function selectModDir() {
  const path = await open({ directory: true, multiple: false });
  if (path) modDir.value = path as string;
}

async function doImport() {
  if (!canImport.value) return;
  processing.value = true;

  try {
    let pakResults: ScannedMod[] = [];

    // Separate pak files and archives
    const pakFiles = selectedFiles.value.filter((f) => /\.pak$/i.test(f));
    const archives = selectedFiles.value.filter((f) => /\.(zip|7z|rar)$/i.test(f));

    // Copy .pak files
    if (pakFiles.length > 0) {
      processingText.value = t("mod.importCopying");
      const copied = await invoke<ScannedMod[]>("copy_mod_files", {
        filePaths: pakFiles,
        destDir: modDir.value,
      });
      pakResults.push(...copied);
    }

    // Extract archives
    if (archives.length > 0) {
      processingText.value = t("mod.importExtracting");
      const extracted = await invoke<ScannedMod[]>("extract_mod_files", {
        archivePaths: archives,
        destDir: modDir.value,
      });
      pakResults.push(...extracted);
    }

    // Register mods in DB
    processingText.value = t("mod.importRegistering");
    const gameId = selectedGameId.value;
    const game = gameId ? gameStore.games.find((g) => g.id === gameId) : null;
    const gameDir = game?.install_path ?? "";

    for (const pak of pakResults) {
      await modStore.addMod({
        name: pak.name,
        description: "",
        mod_path: pak.path,
        install_path: modDir.value,
        game_id: gameId,
        game_dir: gameDir,
        version: "",
        author: "",
        is_enabled: true,
        sort_order: 0,
        category: "",
        source_url: "",
        cover_path: "",
        mod_type: pak.mod_type ?? "file",
        original_name: pak.path.split(/[\\/]/).pop() ?? pak.name,
      });
    }

    addToast(t("mod.importComplete", { count: pakResults.length }), "success");
    emit("imported");
    emit("close");
  } catch (e: any) {
    addToast(typeof e === "string" ? e : e?.message ?? String(e), "error");
  } finally {
    processing.value = false;
    processingText.value = "";
  }
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
        <h2 class="text-lg font-bold text-text-main">{{ t("mod.importModTitle") }}</h2>
        <button
          class="p-1.5 rounded-lg hover:bg-primary-50 dark:hover:bg-primary-900/30 text-text-sub transition-colors"
          @click="emit('close')"
        >
          ✕
        </button>
      </div>

      <div class="px-8 py-8 space-y-6">
        <!-- File Selection -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t("mod.importFiles") }}</label>
          <div class="flex gap-2">
            <button
              class="flex-1 px-4 py-3 rounded-xl border-2 border-dashed border-primary-200 text-sm text-text-sub hover:border-primary-400 hover:bg-primary-50/50 dark:hover:bg-primary-900/20 transition-colors text-center"
              @click="selectFiles"
            >
              {{ t("mod.importFilesHint") }}
            </button>
          </div>
          <!-- Selected files list -->
          <div v-if="selectedFiles.length > 0" class="mt-3 space-y-1.5 max-h-40 overflow-auto">
            <div
              v-for="(file, index) in fileNames"
              :key="index"
              class="flex items-center gap-2 px-3 py-2 rounded-lg bg-input-bg text-sm"
            >
              <span class="text-primary-500">{{ /\.(zip|7z|rar)$/i.test(selectedFiles[index]) ? '📦' : '🎮' }}</span>
              <span class="flex-1 text-text-main truncate">{{ file }}</span>
              <button
                class="text-text-sub hover:text-red-500 transition-colors text-xs"
                @click="removeFile(index)"
              >
                ✕
              </button>
            </div>
          </div>
        </div>

        <!-- Linked Game -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t("mod.linkedGame") }}</label>
          <CustomSelect
            v-model="selectedGameId"
            :options="gameOptions"
            searchable
            :placeholder="t('mod.noGame')"
          />
        </div>

        <!-- Mod Directory -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t("mod.modDir") }}</label>
          <div class="flex gap-2">
            <input
              v-model="modDir"
              class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
              :placeholder="t('mod.modDirPlaceholder')"
            />
            <button
              class="px-4 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 dark:hover:bg-primary-900/30 transition-colors shrink-0"
              @click="selectModDir"
            >
              {{ t("mod.browse") }}
            </button>
          </div>
          <p class="mt-1 text-xs text-text-sub">{{ t("mod.modDirHint") }}</p>
        </div>

        <!-- Actions -->
        <div class="flex items-center gap-3 pt-2">
          <div v-if="processing" class="flex items-center gap-2 flex-1">
            <div class="w-4 h-4 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div>
            <span class="text-sm text-text-sub">{{ processingText }}</span>
          </div>
          <div class="flex-1" v-else></div>
          <button
            class="px-4 py-2.5 rounded-xl border border-primary-200 text-sm text-text-sub hover:bg-primary-50 dark:hover:bg-primary-900/30 transition-colors"
            @click="emit('close')"
          >
            {{ t("common.cancel") }}
          </button>
          <button
            :disabled="!canImport"
            class="px-6 py-2.5 rounded-xl bg-gradient-to-r from-primary-500 to-primary-600 text-white text-sm font-medium hover:from-primary-600 hover:to-primary-700 transition-all shadow-sm disabled:opacity-50 disabled:cursor-not-allowed"
            @click="doImport"
          >
            {{ t("mod.importConfirm") }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
