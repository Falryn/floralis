<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useModStore } from "../stores/modStore";
import { useGameStore } from "../stores/gameStore";
import { useI18n } from "vue-i18n";
import { addToast } from "../composables/useToast";
import CustomSelect from "./CustomSelect.vue";
import type { ScannedMod, ModScanProgress } from "../types";

const { t } = useI18n();

const emit = defineEmits<{
  close: [];
  imported: [];
}>();

const modStore = useModStore();
const gameStore = useGameStore();

const targetPath = ref("");
const scanning = ref(false);
const scanProgress = ref<ModScanProgress | null>(null);
const foundMods = ref<ScannedMod[]>([]);
const checkedIndices = ref<Set<number>>(new Set());
const importing = ref(false);
const selectedGameId = ref<number | null>(null);

const gameOptions = computed(() => [
  { label: t("mod.noGame"), value: null as number | null },
  ...gameStore.games.map((g) => ({ label: g.name, value: g.id as number | null })),
]);

// Auto-fill target path when game is selected
watch(selectedGameId, (gameId) => {
  if (gameId) {
    const game = gameStore.games.find((g) => g.id === gameId);
    if (game) {
      targetPath.value = game.default_mod_dir || game.install_path || "";
    }
  }
});

async function selectDirectory() {
  const path = await open({ directory: true, multiple: false });
  if (path) targetPath.value = path as string;
}

async function startScan() {
  if (!targetPath.value) return;

  scanning.value = true;
  scanProgress.value = null;
  foundMods.value = [];
  checkedIndices.value = new Set();

  const unlisten = await listen<ModScanProgress>("mod-scan-progress", (event) => {
    scanProgress.value = event.payload;
  });

  try {
    const discoveredMods = await invoke<ScannedMod[]>("scan_mod_directory", { dirPath: targetPath.value });
    foundMods.value = discoveredMods;
    checkedIndices.value = new Set(discoveredMods.map((_, i) => i));
  } catch (e: any) {
    addToast(typeof e === "string" ? e : String(e), "error");
  } finally {
    unlisten();
    scanning.value = false;
  }
}

function toggleCheck(index: number) {
  const s = new Set(checkedIndices.value);
  if (s.has(index)) s.delete(index);
  else s.add(index);
  checkedIndices.value = s;
}

function toggleSelectAll() {
  if (checkedIndices.value.size === foundMods.value.length) {
    checkedIndices.value = new Set();
  } else {
    checkedIndices.value = new Set(foundMods.value.map((_, i) => i));
  }
}

const isAllSelected = () => foundMods.value.length > 0 && checkedIndices.value.size === foundMods.value.length;

async function importSelected() {
  const selected = foundMods.value.filter((_, i) => checkedIndices.value.has(i));
  importing.value = true;

  const gameId = selectedGameId.value;
  const game = gameId ? gameStore.games.find((g) => g.id === gameId) : null;
  const gameDir = game?.install_path ?? "";

  try {
    for (const mod of selected) {
      await modStore.addMod({
        name: mod.name,
        description: "",
        mod_path: mod.path,
        install_path: targetPath.value,
        game_id: gameId,
        game_dir: gameDir,
        version: "",
        author: "",
        is_enabled: true,
        sort_order: 0,
        category: "",
        source_url: "",
        cover_path: "",
        mod_type: mod.mod_type ?? "file",
        original_name: mod.path.split(/[\\/]/).pop() ?? mod.name,
      });
    }
    addToast(t("mod.importComplete", { count: selected.length }), "success");
    emit("imported");
    emit("close");
  } catch (e: any) {
    addToast(typeof e === "string" ? e : String(e), "error");
  } finally {
    importing.value = false;
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
        <h2 class="text-lg font-bold text-text-main">{{ t("mod.scanTitle") }}</h2>
        <button
          class="p-1.5 rounded-lg hover:bg-primary-50 dark:hover:bg-primary-900/30 text-text-sub transition-colors"
          @click="emit('close')"
        >
          ✕
        </button>
      </div>

      <div class="px-8 py-8 space-y-6">
        <!-- Linked Game (select first, then scan) -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t("mod.linkedGame") }}</label>
          <CustomSelect
            v-model="selectedGameId"
            :options="gameOptions"
            searchable
            :placeholder="t('mod.noGame')"
          />
          <p class="mt-1 text-xs text-text-sub">{{ t("mod.scanGameHint") }}</p>
        </div>

        <!-- Directory Selection -->
        <div>
          <label class="block text-sm font-medium text-text-main mb-1.5">{{ t("mod.scanDirLabel") }}</label>
          <div class="flex gap-2">
            <input
              v-model="targetPath"
              readonly
              class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 bg-input-bg outline-none truncate cursor-pointer hover:border-primary-300 transition-colors"
              :placeholder="t('mod.scanDirPlaceholder')"
              @click="selectDirectory"
            />
            <button
              class="px-4 py-2.5 text-sm rounded-xl bg-primary-500 text-white hover:bg-primary-600 transition-colors shrink-0"
              @click="selectDirectory"
            >
              {{ t("mod.browse") }}
            </button>
          </div>
          <p class="mt-1 text-xs text-text-sub">{{ t("mod.scanDirHint") }}</p>
        </div>

        <!-- Scan Progress -->
        <div v-if="scanning" class="flex items-center gap-3">
          <div class="w-4 h-4 border-2 border-primary-500 border-t-transparent rounded-full animate-spin"></div>
          <span class="text-sm text-text-sub">
            {{ scanProgress ? scanProgress.name : t("mod.scanning") }}
          </span>
        </div>

        <!-- Found Mods List -->
        <div v-if="foundMods.length > 0 && !scanning" class="space-y-3">
          <div class="flex items-center justify-between">
            <h3 class="text-sm font-medium text-text-main">{{ t("mod.foundMods") }} ({{ foundMods.length }})</h3>
            <button
              class="text-xs text-primary-600 hover:text-primary-700 transition-colors"
              @click="toggleSelectAll"
            >
              {{ isAllSelected() ? t("mod.deselectAll") : t("mod.selectAll") }}
            </button>
          </div>
          <div class="space-y-1.5 max-h-64 overflow-auto">
            <label
              v-for="(mod, index) in foundMods"
              :key="index"
              class="flex items-center gap-3 px-3 py-2.5 rounded-xl hover:bg-primary-50 dark:hover:bg-primary-900/30 transition-colors cursor-pointer"
            >
              <input
                type="checkbox"
                :checked="checkedIndices.has(index)"
                class="rounded accent-primary-500"
                @change="toggleCheck(index)"
              />
              <div class="flex-1 min-w-0">
                <div class="text-sm text-text-main font-medium truncate">{{ mod.name }}</div>
                <div class="text-xs text-text-sub truncate">{{ mod.path }}</div>
              </div>
            </label>
          </div>
        </div>

        <!-- No results -->
        <div v-if="foundMods.length === 0 && !scanning && targetPath" class="text-center py-4">
          <p class="text-sm text-text-sub">{{ t("mod.scanNoResult") }}</p>
        </div>

        <!-- Actions -->
        <div class="flex items-center gap-3 pt-2">
          <button
            v-if="!scanning && !importing"
            :disabled="!targetPath"
            class="px-4 py-2.5 rounded-xl border border-primary-200 text-sm text-text-sub hover:bg-primary-50 dark:hover:bg-primary-900/30 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            @click="startScan"
          >
            {{ t("mod.startScan") }}
          </button>
          <div class="flex-1"></div>
          <button
            class="px-4 py-2.5 rounded-xl border border-primary-200 text-sm text-text-sub hover:bg-primary-50 dark:hover:bg-primary-900/30 transition-colors"
            @click="emit('close')"
          >
            {{ t("common.cancel") }}
          </button>
          <button
            v-if="foundMods.length > 0 && !scanning && !importing"
            :disabled="checkedIndices.size === 0"
            class="px-6 py-2.5 rounded-xl bg-gradient-to-r from-primary-500 to-primary-600 text-white text-sm font-medium hover:from-primary-600 hover:to-primary-700 transition-all shadow-sm disabled:opacity-50 disabled:cursor-not-allowed"
            @click="importSelected"
          >
            {{ t("mod.importSelected") }} ({{ checkedIndices.size }})
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
