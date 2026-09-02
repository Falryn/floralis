<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { invoke } from "../utils/invoke";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useGameStore } from "../stores/gameStore";
import { failTask } from "../composables/useTaskCenter";
import { addToast } from "../composables/useToast";
import { useI18n } from "vue-i18n";
import type { ExtractResult, SteamLibraryItem, EpicLibraryItem, MatchCandidate } from "../types";
import MultiSelect from "./MultiSelect.vue";

const { t } = useI18n();

export interface ExtractedGameData {
  name: string;
  install_path: string;
  exe_path: string;
  cover_path: string;
  save_path: string;
  script_path: string;
  script_args: string;
}

interface BatchItem {
  name: string;
  install_path: string;
  exe_path: string;
  cover_path: string;
  save_path: string;
  included: boolean;
  matched: boolean;
  /** 安装目录已在库中（默认不勾选） */
  duplicate: boolean;
  /** Steam appId（用于 CDN 封面下载兜底） */
  appId?: number;
  /** 多名称候选（引擎标题/exe 名/目录名），匹配时逐个尝试 */
  nameCandidates?: string[];
  /** 多源匹配状态 */
  matchState?: "idle" | "matching" | "done" | "none";
  /** 匹配候选列表（中/低置信度时供用户选择） */
  candidates?: MatchCandidate[];
  /** 候选面板展开 */
  candidatesOpen?: boolean;
  /** 用户选定（或高置信自动选定）的候选，导入时按其数据源下载封面 */
  selectedCandidate?: MatchCandidate | null;
}

const emit = defineEmits<{
  close: [];
  extracted: [data: ExtractedGameData];
  batchImported: [];
}>();

const props = defineProps<{
  initialPaths?: string[];
  /** 由库监视横幅打开时，直接进入“库目录扫描”并自动扫描该根目录 */
  initialLibraryRoot?: string;
}>();

const store = useGameStore();

type ImportMode = "archive" | "local" | "library" | "steam" | "epic";
type Phase = "setup" | "extracting" | "results";

const importMode = ref<ImportMode>("local");
const phase = ref<Phase>("setup");
/** 当前视图：入口卡片页 或 具体导入模式子流程 */
const view = ref<"entry" | ImportMode>("entry");

/** 从入口页进入某个导入模式子流程 */
function enterMode(m: ImportMode) {
  importMode.value = m;
  view.value = m;
}

// 子流程标题（字面量映射，避免动态拼接键导致 i18n 检查遗漏）
const modeTitleKeys: Record<ImportMode, string> = {
  local: "import.mode.local",
  archive: "import.mode.archive",
  library: "import.mode.library",
  steam: "import.mode.steam",
  epic: "import.mode.epic",
};
const modeTitle = computed(() => t(modeTitleKeys[importMode.value]));

// Archive mode
const archivePaths = ref<string[]>([]);
/** 勾选的已保存密码（默认全选，解压时按勾选顺序尝试） */
const selectedPasswords = ref<string[]>([]);
const customPassword = ref("");
const customDestPath = ref("");
const extracting = ref(false);

// Batch extraction progress
const batchProgress = ref({ current: 0, total: 0, name: "" });
const batchResults = ref<BatchItem[]>([]);
const matching = ref(false);
const importing = ref(false);
const matchProgress = ref({ current: 0, total: 0 });

// Local mode
const localDirPath = ref("");
const scanning = ref(false);

// Library scan mode（Playnite 式批量扫描：根目录下每个子文件夹识别为一个游戏）
const libraryRootPath = ref("");
const libraryScanning = ref(false);
const libraryEmpty = ref(false);

// Steam library mode
const steamRootPath = ref("");
const steamScanning = ref(false);
const steamEmpty = ref(false);

// Epic library mode
const epicManifestsPath = ref("");
const epicScanning = ref(false);
const epicEmpty = ref(false);

// 拖入的多路径（压缩包/文件夹混合时仅处理压缩包）
const initialPaths = props.initialPaths ?? [];
const ARCHIVE_EXTS = ["zip", "rar", "7z", "tar", "gz", "bz2", "xz"];
function isArchivePath(p: string): boolean {
  const ext = p.split(".").pop()?.toLowerCase() ?? "";
  return ARCHIVE_EXTS.includes(ext);
}

// 查重：已导入游戏的安装目录集合（归一化比较）
function normalizePath(p: string): string {
  return p.replace(/\//g, "\\").toLowerCase();
}
const existingPaths = computed(
  () => new Set(store.games.filter((g) => g.install_path).map((g) => normalizePath(g.install_path)))
);
/** 本批扫描内已出现过的路径（防同批重复） */
const seenInBatch = new Set<string>();

/** 统一的扫描结果 → 批量清单条目转换（含查重标记） */
function toBatchItem(r: ExtractResult, includedBase: boolean, appId?: number): BatchItem {
  const norm = normalizePath(r.extract_dir);
  const duplicate = existingPaths.value.has(norm) || seenInBatch.has(norm);
  seenInBatch.add(norm);
  return {
    name: r.detected_name,
    install_path: r.extract_dir,
    exe_path: r.exe_path,
    cover_path: r.cover_path,
    save_path: r.save_path,
    included: includedBase && !duplicate,
    matched: false,
    duplicate,
    appId,
    nameCandidates:
      r.name_candidates && r.name_candidates.length > 0 ? r.name_candidates : [r.detected_name],
  };
}

let unlisten: (() => void) | null = null;

onMounted(async () => {
  // 默认勾选全部已保存密码（对应旧版"自动尝试"默认开启的行为）
  selectedPasswords.value = [...store.passwords];
  unlisten = await listen<{ current: number; total: number; name: string }>("extract-progress", (e) => {
    batchProgress.value = e.payload;
  });

  // 拖拽进入：根据文件类型自动分流并启动导入（跳过入口卡片页，直接进对应子流程）
  if (initialPaths.length > 0) {
    const archives = initialPaths.filter(isArchivePath);
    const dirs = initialPaths.filter((p) => !isArchivePath(p));
    if (archives.length > 0) {
      importMode.value = "archive";
      view.value = "archive";
      archivePaths.value = archives;
      await doExtract();
    } else if (dirs.length === 1) {
      importMode.value = "local";
      view.value = "local";
      localDirPath.value = dirs[0];
      if (existingPaths.value.has(normalizePath(dirs[0]))) {
        addToast(t('import.duplicateWarn'), "info");
      }
      await scanLocalGame();
    } else if (dirs.length > 1) {
      view.value = "local";
      await scanDroppedDirs(dirs);
    }
  } else if (props.initialLibraryRoot) {
    // 库监视触发：直达“库目录扫描”并自动扫描
    importMode.value = "library";
    view.value = "library";
    libraryRootPath.value = props.initialLibraryRoot;
    await scanLibraryRoot();
  }
});

onUnmounted(() => {
  unlisten?.();
});

const passwordOptions = computed(() =>
  store.passwords.map((pwd) => ({ label: pwd, value: pwd }))
);

const includedCount = computed(() => batchResults.value.filter(r => r.included).length);

async function selectArchive() {
  const paths = await open({
    filters: [
      {
        name: t('import.title'),
        extensions: ["zip", "rar", "7z", "tar", "gz", "bz2", "xz"],
      },
    ],
    multiple: true,
    directory: false,
  });
  if (paths) {
    archivePaths.value = paths as string[];
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
    emit('extracted', {
      name: result.detected_name,
      install_path: result.extract_dir,
      exe_path: result.exe_path,
      cover_path: result.cover_path,
      save_path: result.save_path || "",
      script_path: "",
      script_args: "",
    });
    emit('close');
  } catch (e) {
    console.error(e);
  } finally {
    scanning.value = false;
  }
}

async function selectLibraryRoot() {
  const path = await open({
    directory: true,
    multiple: false,
  });
  if (path) {
    libraryRootPath.value = path as string;
  }
}

async function scanLibraryRoot() {
  if (!libraryRootPath.value) return;
  libraryScanning.value = true;
  libraryEmpty.value = false;
  try {
    const results = await invoke<ExtractResult[]>("scan_library_root", {
      dirPath: libraryRootPath.value,
    });
    if (results.length === 0) {
      libraryEmpty.value = true;
      return;
    }
    seenInBatch.clear();
    batchResults.value = results.map(r => toBatchItem(r, !!r.exe_path));
    phase.value = 'results';
  } catch (e) {
    console.error(e);
  } finally {
    libraryScanning.value = false;
  }
}

// ==================== Steam 库扫描 ====================

async function selectSteamRoot() {
  const path = await open({
    directory: true,
    multiple: false,
  });
  if (path) {
    steamRootPath.value = path as string;
  }
}

async function detectSteam() {
  try {
    steamRootPath.value = await invoke<string>("detect_steam_root");
  } catch (e) {
    console.warn("Steam 自动检测失败:", e);
    addToast(t('import.steamDetectFail'), "error");
  }
}

async function scanSteamLibrary() {
  if (!steamRootPath.value) return;
  steamScanning.value = true;
  steamEmpty.value = false;
  try {
    const items = await invoke<SteamLibraryItem[]>("scan_steam_library", {
      path: steamRootPath.value,
    });
    if (items.length === 0) {
      steamEmpty.value = true;
      return;
    }
    seenInBatch.clear();
    batchResults.value = items.map((it) => {
      const norm = normalizePath(it.install_path);
      const duplicate = existingPaths.value.has(norm) || seenInBatch.has(norm);
      seenInBatch.add(norm);
      return {
        name: it.name,
        install_path: it.install_path,
        exe_path: it.exe_path,
        cover_path: it.cover_path,
        save_path: "",
        included: !duplicate,
        matched: false,
        duplicate,
        appId: it.app_id,
        nameCandidates: [it.name],
      } as BatchItem;
    });
    phase.value = 'results';
  } catch (e) {
    console.error(e);
    addToast(String(e), "error");
  } finally {
    steamScanning.value = false;
  }
}

// ==================== Epic 库扫描 ====================

async function selectEpicManifests() {
  const path = await open({
    directory: true,
    multiple: false,
  });
  if (path) {
    epicManifestsPath.value = path as string;
  }
}

async function detectEpic() {
  try {
    epicManifestsPath.value = await invoke<string>("detect_epic_manifests_dir");
  } catch (e) {
    console.warn("Epic 自动检测失败:", e);
    addToast(t('import.epicDetectFail'), "error");
  }
}

async function scanEpicLibrary() {
  if (!epicManifestsPath.value) return;
  epicScanning.value = true;
  epicEmpty.value = false;
  try {
    const items = await invoke<EpicLibraryItem[]>("scan_epic_library", {
      path: epicManifestsPath.value,
    });
    if (items.length === 0) {
      epicEmpty.value = true;
      return;
    }
    seenInBatch.clear();
    batchResults.value = items.map((it) => {
      const norm = normalizePath(it.install_path);
      const duplicate = existingPaths.value.has(norm) || seenInBatch.has(norm);
      seenInBatch.add(norm);
      return {
        name: it.name,
        install_path: it.install_path,
        exe_path: it.exe_path,
        cover_path: "",
        save_path: "",
        included: !duplicate,
        matched: false,
        duplicate,
        nameCandidates: [it.name],
      } as BatchItem;
    });
    phase.value = 'results';
  } catch (e) {
    console.error(e);
    addToast(String(e), "error");
  } finally {
    epicScanning.value = false;
  }
}

/** 拖入多个文件夹：逐个扫描后进入批量结果清单 */
async function scanDroppedDirs(dirs: string[]) {
  seenInBatch.clear();
  const results: BatchItem[] = [];
  for (const dir of dirs) {
    try {
      const r = await invoke<ExtractResult>("scan_local_game", { dirPath: dir });
      results.push(toBatchItem(r, !!r.exe_path));
    } catch {
      // 单个目录扫描失败跳过
    }
  }
  if (results.length > 0) {
    batchResults.value = results;
    phase.value = 'results';
  }
}

async function doExtract() {
  // 单个压缩包：保持原有流程（解压 → 弹出编辑对话框）
  if (archivePaths.value.length === 1) {
    await doSingleExtract();
    return;
  }
  // 多个压缩包：批量解压 → 结果列表
  await doBatchExtract();
}

async function doSingleExtract() {
  extracting.value = true;
  const archivePath = archivePaths.value[0];

  // 密码尝试顺序：临时密码 → 勾选的已保存密码 → 无密码兑底
  const attempts: (string | null)[] = [];
  if (customPassword.value.trim()) attempts.push(customPassword.value.trim());
  attempts.push(...selectedPasswords.value);
  attempts.push(null);

  try {
    let result: ExtractResult | null = null;
    for (const pwd of attempts) {
      result = await invoke<ExtractResult>("extract_game", {
        archivePath,
        destPath: customDestPath.value || null,
        password: pwd,
      });
      if (result.success) break;
    }

    // 软失败（命令成功但解压失败）写入任务中心，保证后台任务失败可见
    if (result && !result.success) {
      failTask("extract", result.error || t("import.extractFailed"), "task.extracting");
      return;
    }

    if (result?.success) {
      emit('extracted', {
        name: result.detected_name,
        install_path: result.extract_dir,
        exe_path: result.exe_path,
        cover_path: result.cover_path,
        save_path: result.save_path || "",
        script_path: "",
        script_args: "",
      });
      emit('close');
    }
  } catch (e) {
    // 硬失败（command reject）同样写入任务中心
    failTask("extract", e instanceof Error ? e.message : String(e), "task.extracting");
  } finally {
    extracting.value = false;
  }
}

async function doBatchExtract() {
  phase.value = 'extracting';
  batchProgress.value = { current: 0, total: archivePaths.value.length, name: "" };

  const passwords: string[] = [];
  if (customPassword.value.trim()) {
    passwords.push(customPassword.value.trim());
  }
  passwords.push(...selectedPasswords.value);

  try {
    const results = await invoke<ExtractResult[]>("batch_extract_games", {
      archivePaths: archivePaths.value,
      destPath: customDestPath.value || null,
      passwords,
    }, { taskKey: "extract" });

    seenInBatch.clear();
    batchResults.value = results.map(r => toBatchItem(r, r.success));
    // 存在解压失败的压缩包时，将后台任务标记为 error
    const failed = results.find(r => !r.success);
    if (failed) {
      failTask("extract", failed.error || t("import.extractFailed"), "task.extracting");
    }
    phase.value = 'results';
  } catch {
    // 硬失败已由统一 invoke 包装写入任务中心（taskKey: extract）
    phase.value = 'setup';
  }
}

// ==================== 一键匹配（多源流水线） ====================

/** 匹配并发数：后端已对 Bangumi/VNDB 全局限流，这里控制整体推进速度 */
const MATCH_CONCURRENCY = 4;
const matchCancelled = ref(false);

/** 高置信候选直接采纳：回写名称并记录选定候选 */
function applyCandidate(item: BatchItem, cand: MatchCandidate) {
  item.name = cand.name;
  item.matched = true;
  item.selectedCandidate = cand;
  item.candidatesOpen = false;
}

/** 候选卡片预览图：bangumi/vndb 用直链，steam 用 CDN 胶囊图 */
function candidateThumb(cand: MatchCandidate): string {
  if (cand.cover_url) return cand.cover_url;
  if (cand.source === "steam" && cand.app_id) {
    return `https://cdn.akamai.steamstatic.com/steam/apps/${cand.app_id}/capsule_231x87.jpg`;
  }
  return "";
}

function selectCandidate(item: BatchItem, cand: MatchCandidate) {
  applyCandidate(item, cand);
}

/** 放弃候选，保留原名手动处理 */
function skipCandidates(item: BatchItem) {
  item.candidatesOpen = false;
  item.matchState = "none";
}

/**
 * 一键匹配：多源并行 + 游戏级流水线（4 路并发），结果流式回填。
 * 高置信自动采纳；中/低置信展开候选卡片供人工确认；无结果标记待手动。
 */
async function matchAll() {
  matching.value = true;
  matchCancelled.value = false;
  matchProgress.value = { current: 0, total: includedCount.value };

  const targets = batchResults.value.filter((i) => i.included);
  let cursor = 0;

  async function worker() {
    while (cursor < targets.length) {
      if (matchCancelled.value) return;
      const item = targets[cursor++];
      item.matchState = "matching";
      try {
        // 多个名称候选一起交给后端，首个已高置信命中时后端会自行收手
        const queries =
          item.nameCandidates && item.nameCandidates.length > 0
            ? item.nameCandidates
            : [item.name];
        const cands = await invoke<MatchCandidate[]>("match_game_metadata", {
          queries,
        });
        if (matchCancelled.value) return;
        item.candidates = cands;
        const top = cands[0];
        if (top && top.confidence === "high") {
          applyCandidate(item, top);
          item.matchState = "done";
        } else if (cands.length > 0) {
          item.matchState = "done";
          item.candidatesOpen = true;
        } else {
          item.matchState = "none";
        }
      } catch {
        item.matchState = "none";
      }
      matchProgress.value.current++;
    }
  }

  const workers = Array.from(
    { length: Math.min(MATCH_CONCURRENCY, targets.length) },
    () => worker()
  );
  await Promise.all(workers);
  matching.value = false;
}

function cancelMatch() {
  matchCancelled.value = true;
}

/// 一键导入：将所有包含的项目添加为游戏
async function importAll() {
  importing.value = true;
  const items = batchResults.value.filter(r => r.included);

  for (const item of items) {
    try {
      const gameId = await store.addGame({
        name: item.name,
        group_id: null,
        install_path: item.install_path,
        exe_path: item.exe_path,
        launch_args: "",
        cover_path: "",
        save_path: item.save_path,
        notes: item.selectedCandidate?.summary ?? "",
        script_path: "",
        script_args: "",
        status: "not_played",
        rating: 0,
        sort_order: 0,
        default_mod_dir: "",
        mod_naming_pattern: "",
        mod_uses_load_order: false,
        tracked_process_name: "",
        is_favorite: false,
      });

      // 封面优先级：本地源图（目录内/Steam 缓存） → 匹配选定候选（按数据源下载） → Steam CDN 兜底
      if (item.cover_path) {
        try {
          await invoke("set_game_cover", { gameId, sourcePath: item.cover_path });
        } catch {
          // 本地封面导入失败不影响游戏入库
        }
      }

      // 下载选定候选的封面（优先级高于本地扫描封面）
      const cand = item.selectedCandidate;
      if (cand) {
        try {
          let localPath = "";
          if (cand.source === "steam" && cand.app_id) {
            localPath = await invoke<string>("download_steam_cover", {
              appId: cand.app_id,
              gameId,
            });
          } else if (cand.cover_url) {
            localPath = await invoke<string>(
              cand.source === "vndb" ? "download_vndb_cover" : "download_bangumi_cover",
              { url: cand.cover_url, gameId }
            );
          }
          if (localPath) {
            await invoke("set_game_cover", { gameId, sourcePath: localPath });
          }
        } catch {
          // 封面下载失败不影响导入
        }
      } else if (!item.cover_path && item.appId) {
        try {
          const dl = await invoke<string>("download_steam_cover", {
            appId: item.appId,
            gameId,
          });
          await invoke("set_game_cover", { gameId, sourcePath: dl });
        } catch {
          // Steam CDN 封面失败不影响导入
        }
      }
    } catch (e) {
      console.warn('导入失败:', item.name, e);
    }
  }

  await store.loadGames();
  importing.value = false;
  emit('batchImported');
  emit('close');
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
        <div class="flex items-center gap-2.5">
          <button
            v-if="view !== 'entry' && phase === 'setup'"
            class="p-1.5 -ml-1.5 rounded-lg hover:bg-primary-50 text-text-sub transition-colors text-sm"
            :title="t('import.back')"
            @click="view = 'entry'"
          >
            ←
          </button>
          <h2 class="text-lg font-bold text-text-main">
            ✨ {{ view === 'entry' ? t('import.title') : modeTitle }}
          </h2>
        </div>
        <button
          class="p-1.5 rounded-lg hover:bg-primary-50 text-text-sub transition-colors"
          @click="emit('close')"
        >
          ✕
        </button>
      </div>

      <!-- Phase: Extracting Progress -->
      <template v-if="phase === 'extracting'">
        <div class="px-8 py-12 flex flex-col items-center gap-4">
          <div class="w-12 h-12 border-4 border-primary-200 border-t-primary-500 rounded-full animate-spin"></div>
          <p class="text-sm text-text-main font-medium">
            {{ t('import.extractingProgress', { current: batchProgress.current, total: batchProgress.total }) }}
          </p>
          <p class="text-xs text-text-sub truncate max-w-[400px]">{{ batchProgress.name }}</p>
          <div class="w-full max-w-[300px] h-2 bg-primary-100 rounded-full overflow-hidden">
            <div
              class="h-full bg-gradient-to-r from-sakura-400 to-sakura-500 rounded-full transition-all duration-300"
              :style="{ width: `${batchProgress.total ? (batchProgress.current / batchProgress.total) * 100 : 0}%` }"
            ></div>
          </div>
        </div>
      </template>

      <!-- Phase: Batch Results -->
      <template v-else-if="phase === 'results'">
        <div class="px-8 py-6 space-y-4">
          <div class="flex items-center justify-between">
            <p class="text-sm text-text-sub">
              {{ t('import.batchResultHint', { count: batchResults.length }) }}
            </p>
            <div class="flex gap-2">
              <button
                v-if="matching"
                class="px-3 py-1.5 text-xs rounded-lg border border-border-light text-text-sub hover:bg-primary-50 transition-colors"
                @click="cancelMatch"
              >
                {{ t('import.matchCancel') }}
              </button>
              <button
                :disabled="matching || importing || includedCount === 0"
                class="px-3 py-1.5 text-xs rounded-lg border border-primary-200 text-primary-600 hover:bg-primary-50 transition-colors disabled:opacity-50"
                @click="matchAll"
              >
                {{ matching ? t('import.matching', { current: matchProgress.current, total: matchProgress.total }) : t('import.matchAll') }}
              </button>
              <button
                :disabled="importing || matching || includedCount === 0"
                class="px-3 py-1.5 text-xs rounded-lg bg-gradient-to-r from-sakura-400 to-sakura-500 text-white hover:from-sakura-500 hover:to-sakura-500 transition-all disabled:opacity-50"
                @click="importAll"
              >
                {{ importing ? t('import.importing') : t('import.importAll', { count: includedCount }) }}
              </button>
            </div>
          </div>

          <div class="space-y-2 max-h-[400px] overflow-auto">
            <div v-for="(item, idx) in batchResults" :key="idx">
              <div
                class="flex items-center gap-3 p-3 rounded-xl border transition-colors"
                :class="item.included ? 'border-primary-200 bg-primary-50/30' : 'border-border-light opacity-50'"
              >
                <input
                  type="checkbox"
                  v-model="item.included"
                  class="rounded accent-primary-500 shrink-0"
                />
                <div class="flex-1 min-w-0">
                  <input
                    v-model="item.name"
                    class="w-full text-sm font-medium text-text-main bg-transparent border-b border-transparent hover:border-primary-200 focus:border-primary-400 outline-none transition-colors px-0 py-0.5"
                    :class="{ 'text-green-600': item.matched }"
                  />
                  <p class="text-xs text-text-sub truncate mt-0.5">{{ item.install_path }}</p>
                </div>
                <span v-if="item.duplicate" class="text-xs text-text-sub shrink-0 px-2 py-0.5 rounded-md bg-code-bg">{{ t('import.duplicate') }}</span>
                <span v-else-if="item.matchState === 'matching'" class="text-xs text-primary-500 shrink-0 animate-pulse">{{ t('import.matchingItem') }}</span>
                <button
                  v-else-if="item.candidates?.length"
                  class="text-xs shrink-0 px-2 py-0.5 rounded-md transition-colors"
                  :class="
                    item.matched
                      ? 'text-green-600 bg-green-500/10 hover:bg-green-500/20'
                      : 'text-amber-600 bg-amber-500/10 hover:bg-amber-500/20'
                  "
                  @click="item.candidatesOpen = !item.candidatesOpen"
                >
                  {{
                    item.candidatesOpen
                      ? t('import.collapseCandidates')
                      : item.matched
                        ? t('import.reviewCandidates', { count: item.candidates.length })
                        : t('import.awaitingConfirm', { count: item.candidates.length })
                  }}
                </button>
                <span v-else-if="item.matched" class="text-xs text-green-500 shrink-0">✓</span>
                <span v-else-if="item.matchState === 'none'" class="text-xs text-text-sub shrink-0">{{ t('import.noMatch') }}</span>
                <span v-else-if="!item.exe_path" class="text-xs text-amber-500 shrink-0">⚠</span>
              </div>

              <!-- 候选卡片：中/低置信度时供人工选择 -->
              <div
                v-if="item.candidatesOpen && item.candidates?.length"
                class="mt-1 mb-2 ml-8 p-2.5 rounded-xl border border-border-light bg-modal-bg/60"
              >
                <div class="flex gap-2 overflow-x-auto pb-1">
                  <button
                    v-for="(cand, ci) in item.candidates.slice(0, 5)"
                    :key="ci"
                    class="shrink-0 w-[124px] rounded-lg border border-border-light hover:border-primary-400 hover:bg-primary-50/50 transition-colors overflow-hidden text-left group"
                    :title="cand.summary || cand.original_name"
                    @click="selectCandidate(item, cand)"
                  >
                    <div class="h-[70px] bg-code-bg flex items-center justify-center overflow-hidden">
                      <img
                        v-if="candidateThumb(cand)"
                        :src="candidateThumb(cand)"
                        class="w-full h-full object-cover"
                        loading="lazy"
                        referrerpolicy="no-referrer"
                      />
                      <span v-else class="text-2xl opacity-40">🎮</span>
                    </div>
                    <div class="px-1.5 py-1">
                      <p class="text-[11px] font-medium text-text-main truncate">{{ cand.name }}</p>
                      <p class="text-[10px] text-text-sub truncate flex items-center gap-1">
                        <span class="uppercase opacity-70">{{ cand.source }}</span>
                        <span>{{ Math.round(cand.score * 100) }}%</span>
                      </p>
                    </div>
                  </button>
                  <button
                    class="shrink-0 self-stretch px-3 rounded-lg border border-dashed border-border-light text-xs text-text-sub hover:bg-primary-50/50 transition-colors"
                    @click="skipCandidates(item)"
                  >
                    {{ t('import.keepOriginal') }}
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </template>

      <!-- Phase: Setup -->
      <template v-else>
        <!-- 入口页：卡片式选择导入方式 -->
        <div v-if="view === 'entry'" class="px-8 py-8 space-y-6">
          <!-- 本地文件组 -->
          <div>
            <h3 class="text-xs font-medium text-text-sub mb-3">{{ t('import.localGroup') }}</h3>
            <div class="grid grid-cols-3 gap-3">
              <button
                class="import-card flex flex-col items-center gap-2.5 p-5 rounded-2xl border border-border-light bg-transparent hover:border-primary-300 hover:bg-primary-50/50 transition-all group"
                @click="enterMode('local')"
              >
                <span class="text-3xl group-hover:scale-110 transition-transform">📁</span>
                <span class="text-sm font-medium text-text-main">{{ t('import.localGame') }}</span>
                <span class="text-xs text-text-sub text-center leading-snug">{{ t('import.localDesc') }}</span>
              </button>
              <button
                class="import-card flex flex-col items-center gap-2.5 p-5 rounded-2xl border border-border-light bg-transparent hover:border-primary-300 hover:bg-primary-50/50 transition-all group"
                @click="enterMode('archive')"
              >
                <span class="text-3xl group-hover:scale-110 transition-transform">📦</span>
                <span class="text-sm font-medium text-text-main">{{ t('import.archive') }}</span>
                <span class="text-xs text-text-sub text-center leading-snug">{{ t('import.archiveDesc') }}</span>
              </button>
              <button
                class="import-card flex flex-col items-center gap-2.5 p-5 rounded-2xl border border-border-light bg-transparent hover:border-primary-300 hover:bg-primary-50/50 transition-all group"
                @click="enterMode('library')"
              >
                <span class="text-3xl group-hover:scale-110 transition-transform">🗂️</span>
                <span class="text-sm font-medium text-text-main">{{ t('import.libraryScan') }}</span>
                <span class="text-xs text-text-sub text-center leading-snug">{{ t('import.libraryDesc') }}</span>
              </button>
            </div>
          </div>

          <!-- 平台游戏库组（后续新增平台只需在此加卡片） -->
          <div>
            <h3 class="text-xs font-medium text-text-sub mb-3">{{ t('import.platformGroup') }}</h3>
            <div class="grid grid-cols-3 gap-3">
              <button
                class="import-card flex flex-col items-center gap-2.5 p-5 rounded-2xl border border-border-light bg-transparent hover:border-primary-300 hover:bg-primary-50/50 transition-all group"
                @click="enterMode('steam')"
              >
                <span class="text-3xl group-hover:scale-110 transition-transform">🎮</span>
                <span class="text-sm font-medium text-text-main">Steam</span>
                <span class="text-xs text-text-sub text-center leading-snug">{{ t('import.steamDesc') }}</span>
              </button>
              <button
                class="import-card flex flex-col items-center gap-2.5 p-5 rounded-2xl border border-border-light bg-transparent hover:border-primary-300 hover:bg-primary-50/50 transition-all group"
                @click="enterMode('epic')"
              >
                <span class="text-3xl group-hover:scale-110 transition-transform">🏪</span>
                <span class="text-sm font-medium text-text-main">Epic Games</span>
                <span class="text-xs text-text-sub text-center leading-snug">{{ t('import.epicDesc') }}</span>
              </button>
            </div>
          </div>
        </div>

        <!-- 具体模式子流程 -->
        <div v-else class="px-8 py-8 space-y-7">
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

          <!-- Library Scan Mode: Select Root Directory -->
          <template v-if="importMode === 'library'">
            <div>
              <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('import.libraryRoot') }}</label>
              <div class="flex gap-2">
                <input
                  :value="libraryRootPath"
                  readonly
                  :placeholder="t('import.libraryRoot')"
                  class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 bg-input-bg outline-none truncate cursor-pointer hover:border-primary-300 transition-colors"
                  @click="selectLibraryRoot"
                />
                <button
                  class="px-4 py-2.5 text-sm rounded-xl bg-primary-500 text-white hover:bg-primary-600 transition-colors shrink-0"
                  @click="selectLibraryRoot"
                >
                  {{ t('import.browse') }}
                </button>
              </div>
              <p class="text-xs text-text-sub mt-1.5">
                {{ t('import.libraryScanHint') }}
              </p>
              <p v-if="libraryEmpty" class="text-xs text-amber-500 mt-1.5">
                {{ t('import.libraryEmpty') }}
              </p>
            </div>

            <button
              :disabled="!libraryRootPath || libraryScanning"
              class="w-full py-3 rounded-xl bg-gradient-to-r from-sakura-400 to-sakura-500 text-white font-medium hover:from-sakura-500 hover:to-sakura-500 transition-all disabled:opacity-50 disabled:cursor-not-allowed shadow-sm"
              @click="scanLibraryRoot"
            >
              {{ libraryScanning ? t('import.scanning') : t('import.scanLibrary') }}
            </button>
          </template>

          <!-- Steam Library Mode -->
          <template v-if="importMode === 'steam'">
            <div>
              <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('import.steamRoot') }}</label>
              <div class="flex gap-2">
                <input
                  :value="steamRootPath"
                  readonly
                  :placeholder="t('import.steamRoot')"
                  class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 bg-input-bg outline-none truncate cursor-pointer hover:border-primary-300 transition-colors"
                  @click="selectSteamRoot"
                />
                <button
                  class="px-4 py-2.5 text-sm rounded-xl bg-primary-500 text-white hover:bg-primary-600 transition-colors shrink-0"
                  @click="selectSteamRoot"
                >
                  {{ t('import.browse') }}
                </button>
                <button
                  class="px-4 py-2.5 text-sm rounded-xl border border-primary-200 text-primary-600 hover:bg-primary-50 transition-colors shrink-0"
                  @click="detectSteam"
                >
                  {{ t('import.detectSteam') }}
                </button>
              </div>
              <p class="text-xs text-text-sub mt-1.5">
                {{ t('import.steamScanHint') }}
              </p>
              <p v-if="steamEmpty" class="text-xs text-amber-500 mt-1.5">
                {{ t('import.steamEmpty') }}
              </p>
            </div>

            <button
              :disabled="!steamRootPath || steamScanning"
              class="w-full py-3 rounded-xl bg-gradient-to-r from-sakura-400 to-sakura-500 text-white font-medium hover:from-sakura-500 hover:to-sakura-500 transition-all disabled:opacity-50 disabled:cursor-not-allowed shadow-sm"
              @click="scanSteamLibrary"
            >
              {{ steamScanning ? t('import.scanning') : t('import.scanSteam') }}
            </button>
          </template>

          <!-- Epic Library Mode -->
          <template v-if="importMode === 'epic'">
            <div>
              <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('import.epicManifests') }}</label>
              <div class="flex gap-2">
                <input
                  :value="epicManifestsPath"
                  readonly
                  :placeholder="t('import.epicManifests')"
                  class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 bg-input-bg outline-none truncate cursor-pointer hover:border-primary-300 transition-colors"
                  @click="selectEpicManifests"
                />
                <button
                  class="px-4 py-2.5 text-sm rounded-xl bg-primary-500 text-white hover:bg-primary-600 transition-colors shrink-0"
                  @click="selectEpicManifests"
                >
                  {{ t('import.browse') }}
                </button>
                <button
                  class="px-4 py-2.5 text-sm rounded-xl border border-primary-200 text-primary-600 hover:bg-primary-50 transition-colors shrink-0"
                  @click="detectEpic"
                >
                  {{ t('import.detectEpic') }}
                </button>
              </div>
              <p class="text-xs text-text-sub mt-1.5">
                {{ t('import.epicScanHint') }}
              </p>
              <p v-if="epicEmpty" class="text-xs text-amber-500 mt-1.5">
                {{ t('import.epicEmpty') }}
              </p>
            </div>

            <button
              :disabled="!epicManifestsPath || epicScanning"
              class="w-full py-3 rounded-xl bg-gradient-to-r from-sakura-400 to-sakura-500 text-white font-medium hover:from-sakura-500 hover:to-sakura-500 transition-all disabled:opacity-50 disabled:cursor-not-allowed shadow-sm"
              @click="scanEpicLibrary"
            >
              {{ epicScanning ? t('import.scanning') : t('import.scanEpic') }}
            </button>
          </template>

          <!-- Archive Mode -->
          <template v-if="importMode === 'archive'">
            <div>
              <label class="block text-sm font-medium text-text-main mb-1.5">{{ t('import.selectArchive') }}</label>
              <div class="flex gap-2">
                <div
                  class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 bg-input-bg outline-none cursor-pointer hover:border-primary-300 transition-colors min-h-[42px]"
                  @click="selectArchive"
                >
                  <template v-if="archivePaths.length > 0">
                    <p class="text-xs text-text-main truncate" v-for="(p, i) in archivePaths" :key="i">
                      {{ p.split('\\').pop()?.split('/').pop() }}
                    </p>
                  </template>
                  <span v-else class="text-text-sub/50">{{ t('import.selectArchiveHint') }}</span>
                </div>
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
                <MultiSelect
                  v-if="store.passwords.length > 0"
                  v-model="selectedPasswords"
                  :options="passwordOptions"
                  :placeholder="t('import.noPassword')"
                />
                <input
                  v-model="customPassword"
                  type="password"
                  class="w-full px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors"
                  :placeholder="t('import.tempPassword')"
                />
                <p class="text-xs text-text-sub">{{ t('import.passwordHint') }}</p>
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
              :disabled="archivePaths.length === 0 || extracting"
              class="w-full py-3 rounded-xl bg-gradient-to-r from-sakura-400 to-sakura-500 text-white font-medium hover:from-sakura-500 hover:to-sakura-500 transition-all disabled:opacity-50 disabled:cursor-not-allowed shadow-sm"
              @click="doExtract"
            >
              {{ extracting ? t('import.extracting') : t('import.startExtract') }}
              <span v-if="archivePaths.length > 1" class="text-xs opacity-80 ml-1">({{ archivePaths.length }})</span>
            </button>
          </template>
        </div>
      </template>
    </div>
  </div>
</template>
