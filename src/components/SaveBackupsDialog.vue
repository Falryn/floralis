<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { useI18n } from "vue-i18n";
import type { SaveBackupInfo } from "../types";
import { addToast } from "../composables/useToast";
import { useGameStore } from "../stores/gameStore";
import { formatBytes } from "../utils/format";
import ConfirmDialog from "./ConfirmDialog.vue";

const props = defineProps<{ gameId: number }>();
const emit = defineEmits<{ close: [] }>();

const { t } = useI18n();
const store = useGameStore();

const game = computed(() => store.games.find((g) => g.id === props.gameId) ?? null);
const hasSavePath = computed(() => !!game.value?.save_path);

const backups = ref<SaveBackupInfo[]>([]);
const loading = ref(false);
const backingUp = ref(false);
const busyId = ref<string | null>(null);
const note = ref("");
const restoreTarget = ref<SaveBackupInfo | null>(null);
const deleteTarget = ref<SaveBackupInfo | null>(null);

async function loadList() {
  loading.value = true;
  try {
    backups.value = await store.listSaveBackups(props.gameId);
  } catch (e) {
    addToast(t("saveBackup.loadFail") + ": " + (e as Error).message, "error");
  } finally {
    loading.value = false;
  }
}

onMounted(loadList);

async function doBackup() {
  if (!game.value) return;
  if (!hasSavePath.value) {
    addToast(t("saveBackup.noSavePath"), "error");
    return;
  }
  backingUp.value = true;
  try {
    await store.backupGameSave(props.gameId, note.value.trim());
    note.value = "";
    await loadList();
    addToast(t("saveBackup.backupSuccess"), "success");
  } catch (e) {
    addToast(t("saveBackup.backupFail") + ": " + (e as Error).message, "error");
  } finally {
    backingUp.value = false;
  }
}

async function doRestore() {
  const target = restoreTarget.value;
  restoreTarget.value = null;
  if (!target) return;
  busyId.value = target.id;
  try {
    await store.restoreGameSave(props.gameId, target.id);
    await loadList();
    addToast(t("saveBackup.restoreSuccess"), "success");
  } catch (e) {
    addToast(t("saveBackup.restoreFail") + ": " + (e as Error).message, "error");
  } finally {
    busyId.value = null;
  }
}

async function doDelete() {
  const target = deleteTarget.value;
  deleteTarget.value = null;
  if (!target) return;
  busyId.value = target.id;
  try {
    await store.deleteSaveBackup(props.gameId, target.id);
    await loadList();
    addToast(t("saveBackup.deleteSuccess"), "success");
  } catch (e) {
    addToast(t("saveBackup.deleteFail") + ": " + (e as Error).message, "error");
  } finally {
    busyId.value = null;
  }
}

async function changeBackupDir() {
  const dir = await open({ directory: true, multiple: false, title: t("saveBackup.chooseDir") });
  if (!dir) return;
  try {
    await store.saveSaveBackupDir(dir as string);
    addToast(t("saveBackup.dirUpdated"), "success");
  } catch (e) {
    addToast(t("saveBackup.dirUpdateFail") + ": " + (e as Error).message, "error");
  }
}

function formatTime(ts: number): string {
  const d = new Date(ts * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}
</script>

<template>
  <div
    class="fixed inset-0 z-[70] flex items-center justify-center bg-black/30 backdrop-blur-sm"
    @click.self="emit('close')"
  >
    <div class="modal-panel bg-modal-bg rounded-3xl shadow-2xl w-[560px] max-h-[80vh] overflow-hidden flex flex-col">
      <!-- Header -->
      <div class="flex items-center justify-between px-8 py-6 border-b border-border-light shrink-0">
        <div class="min-w-0">
          <h2 class="text-lg font-bold text-text-main">💾 {{ t("saveBackup.title") }}</h2>
          <p class="text-xs text-text-sub mt-0.5 truncate">{{ game?.name }}</p>
        </div>
        <button class="p-1.5 rounded-lg hover:bg-primary-50 text-text-sub transition-colors" @click="emit('close')">✕</button>
      </div>

      <div class="px-8 py-6 overflow-auto flex-1 space-y-5">
        <!-- Backup location -->
        <div class="flex items-center justify-between gap-3 text-xs">
          <div class="min-w-0 flex-1">
            <p class="text-text-sub font-medium mb-0.5">{{ t("saveBackup.backupDir") }}</p>
            <p class="text-text-sub/80 truncate" :title="store.settings.save_backup_dir || t('saveBackup.backupDirDefault')">
              {{ store.settings.save_backup_dir || t("saveBackup.backupDirDefault") }}
            </p>
          </div>
          <button
            class="px-3 py-1.5 rounded-lg border border-border-medium text-text-sub hover:bg-code-bg transition-colors shrink-0"
            @click="changeBackupDir"
          >
            {{ t("saveBackup.change") }}
          </button>
        </div>

        <!-- No save path hint -->
        <div
          v-if="!hasSavePath"
          class="p-3 rounded-2xl bg-yellow-50 border border-yellow-200 text-yellow-700 text-xs leading-relaxed"
        >
          ⚠️ {{ t("saveBackup.noSavePath") }}
        </div>

        <!-- Backup action -->
        <div class="flex items-center gap-3">
          <input
            v-model="note"
            type="text"
            class="flex-1 px-4 py-2.5 rounded-xl bg-input-bg border border-border-light text-sm text-text-main placeholder:text-text-sub/50 focus:outline-none focus:border-primary-400"
            :placeholder="t('saveBackup.notePlaceholder')"
            :disabled="backingUp"
            @keyup.enter="doBackup"
          />
          <button
            class="px-5 py-2.5 rounded-xl bg-gradient-to-r from-primary-500 to-primary-600 text-white text-sm font-medium hover:from-primary-600 hover:to-primary-700 transition-all shrink-0 disabled:opacity-50 disabled:cursor-not-allowed"
            :disabled="backingUp || !hasSavePath"
            @click="doBackup"
          >
            {{ backingUp ? t("saveBackup.backingUp") : t("saveBackup.backupNow") }}
          </button>
        </div>

        <!-- Backup list -->
        <div v-if="loading" class="text-center py-8 text-text-sub text-sm">
          {{ t("common.loading") }}
        </div>
        <div v-else-if="backups.length === 0" class="text-center py-8 text-text-sub text-sm">
          {{ t("saveBackup.empty") }}
        </div>
        <div v-else class="space-y-2">
          <div
            v-for="b in backups"
            :key="b.id"
            class="flex items-center gap-3 px-4 py-3 rounded-2xl bg-input-bg border border-border-light"
          >
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <p class="text-sm text-text-main font-medium">{{ formatTime(b.created_at) }}</p>
                <span
                  v-if="b.is_auto"
                  class="px-1.5 py-0.5 rounded-md bg-primary-50 text-primary-500 text-[10px] font-medium"
                >{{ t("saveBackup.auto") }}</span>
              </div>
              <p class="text-xs text-text-sub mt-0.5 truncate">
                {{ formatBytes(b.size_bytes) }} · {{ t("saveBackup.fileCount", { n: b.file_count }) }}
                <template v-if="b.note"> · {{ b.note }}</template>
              </p>
            </div>
            <button
              class="px-3 py-1.5 rounded-lg border border-primary-300 text-primary-600 text-xs hover:bg-primary-50 transition-colors shrink-0 disabled:opacity-50"
              :disabled="busyId === b.id"
              @click="restoreTarget = b"
            >
              {{ t("saveBackup.restore") }}
            </button>
            <button
              class="px-3 py-1.5 rounded-lg border border-border-medium text-text-sub text-xs hover:bg-red-50 hover:text-red-500 hover:border-red-200 transition-colors shrink-0 disabled:opacity-50"
              :disabled="busyId === b.id"
              @click="deleteTarget = b"
            >
              {{ t("saveBackup.delete") }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Restore confirm -->
    <ConfirmDialog
      v-if="restoreTarget"
      :title="t('saveBackup.confirmRestoreTitle')"
      :message="t('saveBackup.confirmRestoreMsg')"
      :confirm-text="t('saveBackup.restore')"
      @confirm="doRestore"
      @cancel="restoreTarget = null"
    />

    <!-- Delete confirm -->
    <ConfirmDialog
      v-if="deleteTarget"
      :title="t('saveBackup.confirmDeleteTitle')"
      :message="t('saveBackup.confirmDeleteMsg')"
      :confirm-text="t('saveBackup.delete')"
      danger
      @confirm="doDelete"
      @cancel="deleteTarget = null"
    />
  </div>
</template>
