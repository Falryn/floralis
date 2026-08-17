<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useI18n } from "vue-i18n";
import type { IntegrityReport, IntegrityIssue, RelocateReport } from "../types";
import { addToast } from "../composables/useToast";
import { useGameStore } from "../stores/gameStore";

const { t } = useI18n();
const store = useGameStore();
const emit = defineEmits<{ close: [] }>();

const checking = ref(false);
const report = ref<IntegrityReport | null>(null);
const rescanningId = ref<number | null>(null);
const cleaning = ref(false);
// 库路径重定位：单个进行中 id / 批量进行中
const relocatingId = ref<number | null>(null);
const batchRelocating = ref(false);

// 问题类型的展示元数据
const typeMeta: Record<string, { icon: string; labelKey: string }> = {
  missing_cover: { icon: "🖼️", labelKey: "integrity.missingCover" },
  missing_exe: { icon: "⚙️", labelKey: "integrity.missingExe" },
  missing_install: { icon: "📁", labelKey: "integrity.missingInstall" },
  missing_save: { icon: "💾", labelKey: "integrity.missingSave" },
  duplicate: { icon: "👥", labelKey: "integrity.duplicate" },
};

const groupedIssues = computed(() => {
  if (!report.value) return [];
  const order = ["missing_cover", "missing_exe", "missing_install", "missing_save", "duplicate"];
  return order
    .map((type) => ({
      type,
      meta: typeMeta[type],
      items: report.value!.issues.filter((i) => i.issue_type === type),
    }))
    .filter((g) => g.items.length > 0);
});

const issueCount = computed(() => report.value?.issues.length ?? 0);
const orphanCount = computed(() => report.value?.orphan_covers.length ?? 0);

async function runCheckup() {
  checking.value = true;
  report.value = null;
  try {
    report.value = await invoke<IntegrityReport>("run_integrity_checkup");
  } catch (e) {
    addToast(t("integrity.checkFail") + ": " + (e as string), "error");
  } finally {
    checking.value = false;
  }
}

async function rescanCover(issue: IntegrityIssue) {
  rescanningId.value = issue.game_id;
  try {
    await invoke("rescan_game_cover", { gameId: issue.game_id });
    await store.loadGames();
    report.value = await invoke<IntegrityReport>("run_integrity_checkup");
    addToast(t("integrity.coverFixed", { name: issue.game_name }), "success");
  } catch (e) {
    addToast(t("integrity.fixFail") + ": " + (e as string), "error");
  } finally {
    rescanningId.value = null;
  }
}

async function cleanupOrphans() {
  if (!report.value || report.value.orphan_covers.length === 0) return;
  cleaning.value = true;
  try {
    const removed = await invoke<number>("cleanup_orphan_files", { paths: report.value.orphan_covers });
    addToast(t("integrity.cleaned", { count: removed }), "success");
    report.value = await invoke<IntegrityReport>("run_integrity_checkup");
  } catch (e) {
    addToast(t("integrity.fixFail") + ": " + (e as string), "error");
  } finally {
    cleaning.value = false;
  }
}

/** 单个重定位：选目录后修复该游戏的安装/主程序/存档路径 */
async function relocateOne(issue: IntegrityIssue) {
  const dir = await open({ directory: true, multiple: false, title: t("integrity.chooseNewInstallDir") });
  if (!dir) return;
  relocatingId.value = issue.game_id;
  try {
    await invoke("relocate_game", { gameId: issue.game_id, newInstallPath: dir as string });
    await store.loadGames();
    report.value = await invoke<IntegrityReport>("run_integrity_checkup");
    addToast(t("integrity.relocated", { name: issue.game_name }), "success");
  } catch (e) {
    addToast(t("integrity.fixFail") + ": " + (e as string), "error");
  } finally {
    relocatingId.value = null;
  }
}

/** 批量重定位：选新库根目录，按子文件夹同名匹配修复全部失效游戏 */
async function relocateBatch() {
  const dir = await open({ directory: true, multiple: false, title: t("integrity.chooseNewRoot") });
  if (!dir) return;
  batchRelocating.value = true;
  try {
    const result = await invoke<RelocateReport>("relocate_games_by_root", { rootPath: dir as string });
    await store.loadGames();
    report.value = await invoke<IntegrityReport>("run_integrity_checkup");
    if (result.fixed > 0) {
      addToast(t("integrity.batchRelocated", { fixed: result.fixed, count: result.unmatched.length }), "success");
    } else {
      addToast(t("integrity.batchNoMatch"), "error");
    }
  } catch (e) {
    addToast(t("integrity.fixFail") + ": " + (e as string), "error");
  } finally {
    batchRelocating.value = false;
  }
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
        <h2 class="text-lg font-bold text-text-main">🩺 {{ t("integrity.title") }}</h2>
        <button class="p-1.5 rounded-lg hover:bg-primary-50 text-text-sub transition-colors" @click="emit('close')">✕</button>
      </div>

      <div class="px-8 py-6 overflow-auto flex-1 space-y-5">
        <!-- Run button -->
        <div class="flex items-center justify-between">
          <p class="text-sm text-text-sub">{{ t("integrity.desc") }}</p>
          <button
            class="px-4 py-2 rounded-xl bg-gradient-to-r from-primary-500 to-primary-600 text-white text-sm font-medium hover:from-primary-600 hover:to-primary-700 transition-all shrink-0 ml-4"
            :disabled="checking"
            @click="runCheckup"
          >
            {{ checking ? t("integrity.checking") : t("integrity.run") }}
          </button>
        </div>

        <!-- Empty state -->
        <div v-if="!report && !checking" class="text-center py-10 text-text-sub text-sm">
          {{ t("integrity.empty") }}
        </div>

        <!-- Results -->
        <template v-if="report">
          <!-- Summary cards -->
          <div class="grid grid-cols-3 gap-3">
            <div class="p-3 rounded-2xl bg-code-bg border border-border-light text-center">
              <p class="text-xl font-bold text-text-main">{{ report.total_games }}</p>
              <p class="text-xs text-text-sub mt-1">{{ t("integrity.totalGames") }}</p>
            </div>
            <div class="p-3 rounded-2xl bg-code-bg border border-border-light text-center">
              <p class="text-xl font-bold" :class="issueCount > 0 ? 'text-orange-500' : 'text-emerald-500'">{{ issueCount }}</p>
              <p class="text-xs text-text-sub mt-1">{{ t("integrity.issues") }}</p>
            </div>
            <div class="p-3 rounded-2xl bg-code-bg border border-border-light text-center">
              <p class="text-xl font-bold" :class="orphanCount > 0 ? 'text-orange-500' : 'text-emerald-500'">{{ orphanCount }}</p>
              <p class="text-xs text-text-sub mt-1">{{ t("integrity.orphans") }}</p>
            </div>
          </div>

          <!-- All clear -->
          <div
            v-if="issueCount === 0 && orphanCount === 0"
            class="p-4 rounded-2xl bg-emerald-50 border border-emerald-200 text-emerald-600 text-sm text-center"
          >
            ✅ {{ t("integrity.allClear") }}
          </div>

          <!-- Grouped issues -->
          <div v-for="group in groupedIssues" :key="group.type" class="space-y-2">
            <p class="text-sm font-medium text-text-main flex items-center gap-2">
              <span>{{ group.meta.icon }}</span>
              {{ t(group.meta.labelKey) }}
              <span class="text-xs text-text-sub font-normal">({{ group.items.length }})</span>
              <button
                v-if="group.type === 'missing_install' && group.items.length >= 2"
                class="ml-auto px-3 py-1 rounded-lg text-xs border border-primary-300 text-primary-600 hover:bg-primary-50 transition-colors"
                :disabled="batchRelocating"
                @click="relocateBatch"
              >
                {{ batchRelocating ? t("integrity.fixing") : t("integrity.batchRelocate") }}
              </button>
            </p>
            <div class="space-y-1.5">
              <div
                v-for="issue in group.items"
                :key="issue.game_id + issue.issue_type"
                class="flex items-center justify-between gap-2 px-3 py-2 rounded-xl bg-input-bg border border-border-light"
              >
                <div class="min-w-0 flex-1">
                  <p class="text-sm text-text-main truncate">{{ issue.game_name }}</p>
                  <p class="text-xs text-text-sub truncate">{{ issue.path || t("integrity.noPath") }}</p>
                </div>
                <button
                  v-if="issue.issue_type === 'missing_cover'"
                  class="px-3 py-1.5 rounded-lg text-xs border border-primary-300 text-primary-600 hover:bg-primary-50 transition-colors shrink-0"
                  :disabled="rescanningId === issue.game_id"
                  @click="rescanCover(issue)"
                >
                  {{ rescanningId === issue.game_id ? t("integrity.fixing") : t("integrity.rescan") }}
                </button>
                <button
                  v-else-if="issue.issue_type === 'missing_install'"
                  class="px-3 py-1.5 rounded-lg text-xs border border-primary-300 text-primary-600 hover:bg-primary-50 transition-colors shrink-0"
                  :disabled="relocatingId === issue.game_id || batchRelocating"
                  @click="relocateOne(issue)"
                >
                  {{ relocatingId === issue.game_id ? t("integrity.fixing") : t("integrity.relocate") }}
                </button>
              </div>
            </div>
          </div>

          <!-- Orphan covers -->
          <div v-if="orphanCount > 0" class="space-y-2">
            <p class="text-sm font-medium text-text-main flex items-center gap-2">
              <span>🧹</span> {{ t("integrity.orphanTitle") }}
              <span class="text-xs text-text-sub font-normal">({{ orphanCount }})</span>
            </p>
            <p class="text-xs text-text-sub">{{ t("integrity.orphanDesc") }}</p>
            <button
              class="px-4 py-2 rounded-xl bg-red-500 text-white text-sm hover:bg-red-600 transition-colors"
              :disabled="cleaning"
              @click="cleanupOrphans"
            >
              {{ cleaning ? t("integrity.cleaning") : t("integrity.cleanOrphans", { count: orphanCount }) }}
            </button>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>
