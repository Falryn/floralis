import { computed, ref } from "vue";
import type { Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import { useGameStore } from "../stores/gameStore";
import type { LaunchAction } from "../types";

/** 元数据数据源标识 */
export type MetadataSource = "vndb" | "igdb" | "bangumi" | "steam";

/** VNDB 搜索结果项 */
export interface VndbItem {
  id: string;
  title: string;
  image?: { url?: string } | null;
  description?: string | null;
}

/** IGDB 搜索结果项 */
export interface IgdbItem {
  id: number;
  name: string;
  cover?: { url?: string } | null;
  summary?: string | null;
}

/** Bangumi 搜索结果项 */
export interface BangumiItem {
  id: number;
  nameCn?: string | null;
  name: string;
  images?: { grid?: string; large?: string; common?: string; medium?: string; small?: string } | null;
  summary?: string | null;
}

/** Steam 搜索结果项 */
export interface SteamItem {
  id: number;
  name: string;
  tiny_image?: string | null;
}

export interface MetadataSearchOptions {
  /** 搜索关键词（游戏名输入框） */
  name: Ref<string>;
  /** 是否创建模式：创建模式下封面延迟到游戏落库后再下载 */
  isCreateMode: Ref<boolean>;
  /** 编辑模式下的游戏 ID */
  gameId: () => number | undefined;
  /** 应用搜索结果时回写表单字段 */
  handlers: {
    setName: (name: string) => void;
    setNotes: (notes: string) => void;
    setCover: (path: string) => void;
  };
}

/**
 * 四数据源（Bangumi / Steam / VNDB / IGDB）元数据搜索：
 * 搜索、结果展示状态、应用结果（名称/简介/封面）。
 * 创建模式下封面 URL 记入 pendingCover*，由保存流程落库后统一下载。
 */
export function useGameMetadataSearch(options: MetadataSearchOptions) {
  const { name, isCreateMode, gameId, handlers } = options;
  const store = useGameStore();
  const { t } = useI18n();

  const dataSource = ref<string>('bangumi');

  const vndbResults = ref<VndbItem[]>([]);
  const vndbSearching = ref(false);
  const vndbError = ref("");
  const showVndbResults = ref(false);

  const igdbResults = ref<IgdbItem[]>([]);
  const igdbSearching = ref(false);
  const igdbError = ref("");
  const showIgdbResults = ref(false);

  const bangumiResults = ref<BangumiItem[]>([]);
  const bangumiSearching = ref(false);
  const bangumiError = ref("");
  const showBangumiResults = ref(false);

  const steamResults = ref<SteamItem[]>([]);
  const steamSearching = ref(false);
  const steamError = ref("");
  const showSteamResults = ref(false);

  // 创建模式：封面延迟到游戏创建后再下载
  const pendingCoverUrl = ref<string | null>(null);
  const pendingCoverSource = ref<MetadataSource | null>(null);
  const pendingSteamAppId = ref<number | null>(null);

  const searching = computed(
    () => vndbSearching.value || igdbSearching.value || bangumiSearching.value || steamSearching.value,
  );

  /** 编辑模式：直接下载封面到游戏存储 */
  async function downloadCover(cmd: string, args: Record<string, unknown>) {
    try {
      const localPath = await invoke<string>(cmd, { gameId: gameId(), ...args });
      handlers.setCover(localPath);
    } catch (e) {
      console.warn("封面下载失败:", e);
    }
  }

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

  async function searchBangumi() {
    bangumiSearching.value = true;
    bangumiError.value = "";
    bangumiResults.value = [];
    try {
      bangumiResults.value = await invoke<BangumiItem[]>("search_bangumi", { query: name.value });
      showBangumiResults.value = true;
    } catch (e) {
      bangumiError.value = e as string;
    } finally {
      bangumiSearching.value = false;
    }
  }

  async function searchSteam() {
    steamSearching.value = true;
    steamError.value = "";
    steamResults.value = [];
    try {
      steamResults.value = await invoke<SteamItem[]>("search_steam", { query: name.value });
      showSteamResults.value = true;
    } catch (e) {
      steamError.value = e as string;
    } finally {
      steamSearching.value = false;
    }
  }

  async function doSearch() {
    if (dataSource.value === 'vndb') await searchVndb();
    else if (dataSource.value === 'igdb') await searchIgdb();
    else if (dataSource.value === 'steam') await searchSteam();
    else await searchBangumi();
  }

  async function applyVndbResult(item: VndbItem) {
    handlers.setName(item.title);
    if (item.description) {
      handlers.setNotes(item.description.slice(0, 500));
    }
    const imgUrl = item.image?.url;
    if (imgUrl) {
      if (isCreateMode.value) {
        pendingCoverUrl.value = imgUrl;
        pendingCoverSource.value = 'vndb';
        handlers.setCover(imgUrl); // URL 作为占位预览
      } else {
        await downloadCover("download_vndb_cover", { url: imgUrl });
      }
    }
    showVndbResults.value = false;
  }

  async function applyIgdbResult(item: IgdbItem) {
    handlers.setName(item.name);
    if (item.summary) handlers.setNotes(item.summary.slice(0, 500));
    const imgUrl = item.cover?.url;
    if (imgUrl) {
      if (isCreateMode.value) {
        pendingCoverUrl.value = imgUrl;
        pendingCoverSource.value = 'igdb';
        handlers.setCover('https:' + imgUrl);
      } else {
        await downloadCover("download_igdb_cover", { url: imgUrl });
      }
    }
    showIgdbResults.value = false;
  }

  async function applyBangumiResult(item: BangumiItem) {
    handlers.setName(item.nameCn || item.name);
    if (item.summary) handlers.setNotes(item.summary.slice(0, 500));
    const imgUrl = item.images?.large || item.images?.common || item.images?.medium;
    if (imgUrl) {
      if (isCreateMode.value) {
        pendingCoverUrl.value = imgUrl;
        pendingCoverSource.value = 'bangumi';
        handlers.setCover(imgUrl);
      } else {
        await downloadCover("download_bangumi_cover", { url: imgUrl });
      }
    }
    showBangumiResults.value = false;
  }

  async function applySteamResult(item: SteamItem) {
    handlers.setName(item.name);
    // 构造 header 图 URL 作为预览（比 tiny_image 胶囊图更适合竖版容器）
    const headerUrl = `https://cdn.akamai.steamstatic.com/steam/apps/${item.id}/header.jpg`;
    if (isCreateMode.value) {
      pendingCoverSource.value = 'steam';
      pendingSteamAppId.value = item.id;
      handlers.setCover(headerUrl);
    } else {
      await downloadCover("download_steam_cover", { appId: item.id });
    }
    showSteamResults.value = false;
  }

  return {
    dataSource,
    searching,
    vndbResults, vndbSearching, vndbError, showVndbResults,
    igdbResults, igdbSearching, igdbError, showIgdbResults,
    bangumiResults, bangumiSearching, bangumiError, showBangumiResults,
    steamResults, steamSearching, steamError, showSteamResults,
    pendingCoverUrl, pendingCoverSource, pendingSteamAppId,
    doSearch, searchBangumi,
    applyVndbResult, applyIgdbResult, applyBangumiResult, applySteamResult,
  };
}

// ===== 附加启动入口草稿 =====

/** 附加启动入口草稿：保存时整体提交 */
export interface ActionDraft {
  name: string;
  program_path: string;
  args: string;
}

/**
 * 附加启动入口草稿管理：从库中加载、编辑、脏检测与整体持久化。
 */
export function useLaunchActions() {
  const store = useGameStore();

  const actionDrafts = ref<ActionDraft[]>([]);
  let actionSnapshot = "[]";

  function loadActionDrafts(list: LaunchAction[]) {
    actionDrafts.value = list.map((a) => ({ name: a.name, program_path: a.program_path, args: a.args }));
    actionSnapshot = JSON.stringify(actionDrafts.value);
  }

  const actionsDirty = () => JSON.stringify(actionDrafts.value) !== actionSnapshot;

  const configuredActionCount = computed(
    () => actionDrafts.value.filter((d) => d.program_path.trim()).length,
  );

  async function loadActionsFor(gameId: number) {
    loadActionDrafts(await store.loadLaunchActions(gameId));
  }

  function addActionDraft() {
    actionDrafts.value.push({ name: "", program_path: "", args: "" });
  }

  function removeActionDraft(index: number) {
    actionDrafts.value.splice(index, 1);
  }

  /** 将草稿转为可提交的入口列表（丢弃未选程序的空行） */
  function draftsToActions(gameId: number): LaunchAction[] {
    return actionDrafts.value
      .filter((d) => d.program_path.trim())
      .map((d, i) => ({
        id: 0,
        game_id: gameId,
        name: d.name.trim() || d.program_path.split("\\").pop() || "",
        program_path: d.program_path.trim(),
        args: d.args.trim(),
        sort_order: i,
      }));
  }

  async function persistActions(gameId: number) {
    const saved = await store.saveLaunchActions(gameId, draftsToActions(gameId));
    loadActionDrafts(saved);
  }

  return {
    actionDrafts, configuredActionCount,
    loadActionsFor, addActionDraft, removeActionDraft,
    actionsDirty, persistActions,
  };
}
