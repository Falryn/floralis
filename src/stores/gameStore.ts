/**
 * 游戏状态管理
 * 
 * 使用 Pinia 管理游戏、分组、标签、设置等全局状态
 * 提供与 Rust 后端通信的所有操作方法
 */

import { defineStore } from "pinia";
import { ref, computed, watch } from "vue";
import { invoke } from "../utils/invoke";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Game, Group, AppSettings, PlaySession, Tag, TagUsage, UpdateInfo, LaunchAction } from "../types";

// 图片URL缓存：路径 -> asset协议URL
const imageCache = new Map<string, string>();

/**
 * 加载本地图片并转换为 asset 协议 URL
 * 
 * 使用 Tauri 的 convertFileSrc 安全地访问本地文件
 * 结果会被缓存，避免重复转换
 */
export function loadImage(path: string): string {
  if (!path) return "";
  if (imageCache.has(path)) return imageCache.get(path)!;
  try {
    const url = convertFileSrc(path);
    imageCache.set(path, url);
    return url;
  } catch (e) {
    console.error("[loadImage] failed:", path, e);
    return "";
  }
}

/**
 * 游戏状态 Store
 * 
 * 管理所有游戏相关的全局状态和操作方法
 */
export const useGameStore = defineStore("game", () => {
  const games = ref<Game[]>([]);
  const groups = ref<Group[]>([]);
  const settings = ref<AppSettings>({
    seven_zip_path: "",
    default_extract_path: "",
    custom_banner: "",
    custom_sidebar_bg: "",
    custom_empty_illustration: "",
    theme: "light",
    update_repo: "",
    close_behavior: "ask",
    igdb_client_id: "",
    igdb_client_secret: "",
    image_blur: "0",
    banner_blur: "0",
    banner_brightness: "100",
    sidebar_blur: "0",
    sidebar_brightness: "100",
    auto_backup: "true",
  });
  const passwords = ref<string[]>([]);
  const tags = ref<Tag[]>([]);
  const gameTags = ref<Map<number, Tag[]>>(new Map());

  const selectedGroupId = ref<number | null>(null);
  const selectedGameId = ref<number | null>(null);
  const searchInput = ref(""); // Raw input bound to search field
  const searchKeyword = ref(""); // Debounced value used for filtering
  const sortType = ref<"created_desc" | "created_asc" | "name_asc" | "name_desc" | "last_played" | "rating_desc">("created_desc");

  // Debounce search input → searchKeyword (300ms)
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  watch(searchInput, (val) => {
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      searchKeyword.value = val.trim();
    }, 300);
  });
  const selectedGameIds = ref<Set<number>>(new Set());
  const isSelectMode = ref(false);
  const selectedTagId = ref<number | null>(null);
  const selectedStatus = ref<string | null>(null);

  const filteredGames = computed(() => {
    let list = games.value;
    // Group filter
    if (selectedGroupId.value !== null) {
      list = list.filter((g) => g.group_id === selectedGroupId.value);
    }
    // Tag filter
    if (selectedTagId.value !== null) {
      const tagGameIds = new Set<number>();
      const tags = gameTags.value;
      for (const [gid, tlist] of tags) {
        if (tlist.some((t) => t.id === selectedTagId.value)) {
          tagGameIds.add(gid);
        }
      }
      list = list.filter((g) => tagGameIds.has(g.id));
    }
    // Status filter
    if (selectedStatus.value !== null) {
      list = list.filter((g) => g.status === selectedStatus.value);
    }
    // Search filter（匹配名称/备注/安装路径/标签）
    if (searchKeyword.value.trim()) {
      const kw = searchKeyword.value.trim().toLowerCase();
      list = list.filter((g) => {
        if (g.name.toLowerCase().includes(kw)) return true;
        if (g.notes.toLowerCase().includes(kw)) return true;
        if (g.install_path.toLowerCase().includes(kw)) return true;
        const gtags = gameTags.value.get(g.id);
        return gtags?.some((tag) => tag.name.toLowerCase().includes(kw)) ?? false;
      });
    }
    // Sort
    const sorted = [...list];
    switch (sortType.value) {
      case "name_asc":
        sorted.sort((a, b) => a.name.localeCompare(b.name));
        break;
      case "name_desc":
        sorted.sort((a, b) => b.name.localeCompare(a.name));
        break;
      case "created_asc":
        sorted.sort((a, b) => a.id - b.id);
        break;
      case "last_played":
        sorted.sort((a, b) => {
          const ta = a.last_played_at ?? "";
          const tb = b.last_played_at ?? "";
          return tb.localeCompare(ta);
        });
        break;
      case "rating_desc":
        sorted.sort((a, b) => b.rating - a.rating);
        break;
      case "created_desc":
      default:
        sorted.sort((a, b) => b.id - a.id);
        break;
    }
    return sorted;
  });

  const selectedGame = computed(() => {
    if (selectedGameId.value === null) return null;
    return games.value.find((g) => g.id === selectedGameId.value) ?? null;
  });

  async function loadGames() {
    games.value = await invoke<Game[]>("get_all_games");
  }
  async function loadGroups() {
    groups.value = await invoke<Group[]>("get_all_groups");
  }
  async function loadSettings() {
    settings.value = await invoke<AppSettings>("get_settings");
  }
  async function loadPasswords() {
    passwords.value = await invoke<string[]>("get_passwords");
  }
  async function loadTags() {
    tags.value = await invoke<Tag[]>("get_all_tags");
  }
  async function loadAllGameTags() {
    const result = await invoke<Record<number, Tag[]>>("get_all_game_tags");
    const m = new Map<number, Tag[]>();
    for (const [key, val] of Object.entries(result)) {
      m.set(Number(key), val);
    }
    gameTags.value = m;
  }
  async function loadGameTags(gameId: number) {
    const result = await invoke<Tag[]>("get_game_tags", { gameId });
    const m = new Map(gameTags.value);
    m.set(gameId, result);
    gameTags.value = m;
  }

  async function addGame(game: Omit<Game, "id" | "total_play_time" | "last_played_at">) {
    const id = await invoke<number>("add_game", {
      name: game.name,
      groupId: game.group_id,
      installPath: game.install_path,
      exePath: game.exe_path,
      launchArgs: game.launch_args,
      coverPath: game.cover_path,
      savePath: game.save_path,
      notes: game.notes,
      scriptPath: game.script_path,
      scriptArgs: game.script_args,
    });
    // Optimistic update: append new game to list
    games.value.unshift({ ...game, id, total_play_time: 0, last_played_at: null } as Game);
    return id;
  }

  async function updateGame(game: Game) {
    await invoke("update_game", {
      id: game.id,
      name: game.name,
      groupId: game.group_id,
      installPath: game.install_path,
      exePath: game.exe_path,
      launchArgs: game.launch_args,
      coverPath: game.cover_path,
      savePath: game.save_path,
      notes: game.notes,
      scriptPath: game.script_path,
      scriptArgs: game.script_args,
      defaultModDir: game.default_mod_dir ?? "",
      modNamingPattern: game.mod_naming_pattern ?? "",
      modUsesLoadOrder: game.mod_uses_load_order ?? false,
      trackedProcessName: game.tracked_process_name ?? "",
    });
    // Optimistic update: modify in place
    const idx = games.value.findIndex((g) => g.id === game.id);
    if (idx !== -1) {
      games.value[idx] = { ...game };
    }
  }

  async function deleteGame(id: number) {
    await invoke("delete_game", { id });
    if (selectedGameId.value === id) selectedGameId.value = null;
    // Optimistic update: remove from list
    games.value = games.value.filter((g) => g.id !== id);
  }

  async function setGameGroup(gameId: number, groupId: number | null) {
    await invoke("set_game_group", { gameId, groupId });
    // Optimistic update
    const game = games.value.find((g) => g.id === gameId);
    if (game) game.group_id = groupId;
  }

  async function setGameStatus(gameId: number, status: string) {
    await invoke("set_game_status", { gameId, status });
    // Optimistic update
    const game = games.value.find((g) => g.id === gameId);
    if (game) game.status = status;
  }

  async function setGameRating(gameId: number, rating: number) {
    await invoke("set_game_rating", { gameId, rating });
    // Optimistic update
    const game = games.value.find((g) => g.id === gameId);
    if (game) game.rating = rating;
  }

  async function setGamePlayTime(gameId: number, seconds: number) {
    await invoke("set_game_play_time", { gameId, seconds });
    // Optimistic update
    const game = games.value.find((g) => g.id === gameId);
    if (game) game.total_play_time = seconds;
  }

  /** 设置游戏封面（拖拽图片/本地文件），复制入内部存储并更新记录 */
  async function setGameCover(gameId: number, sourcePath: string) {
    const stored = await invoke<string>("set_game_cover", { gameId, sourcePath });
    // Optimistic update（后端返回带时间戳的新路径，触发各视图刷新）
    const game = games.value.find((g) => g.id === gameId);
    if (game) game.cover_path = stored;
    return stored;
  }

  async function launchGame(id: number, actionId?: number) {
    await invoke("launch_game", { id, actionId: actionId ?? null });
  }

  // 附加启动入口缓存：gameId -> 入口列表（右键菜单/详情页/编辑对话框共用，避免重复查询）
  const launchActionsCache = new Map<number, LaunchAction[]>();

  async function loadLaunchActions(gameId: number): Promise<LaunchAction[]> {
    if (launchActionsCache.has(gameId)) return launchActionsCache.get(gameId)!;
    const list = await invoke<LaunchAction[]>("get_launch_actions", { gameId });
    launchActionsCache.set(gameId, list);
    return list;
  }

  async function saveLaunchActions(gameId: number, actions: LaunchAction[]) {
    const saved = await invoke<LaunchAction[]>("save_launch_actions", { gameId, actions });
    launchActionsCache.set(gameId, saved);
    return saved;
  }

  async function addGroup(name: string) {
    await invoke("add_group", { name });
    await loadGroups();
  }

  async function renameGroup(id: number, name: string) {
    await invoke("rename_group", { id, name });
    // Optimistic update
    const group = groups.value.find((g) => g.id === id);
    if (group) group.name = name;
  }

  async function deleteGroup(id: number) {
    await invoke("delete_group", { id });
    if (selectedGroupId.value === id) selectedGroupId.value = null;
    // Optimistic update: remove group, set games in that group to ungrouped
    groups.value = groups.value.filter((g) => g.id !== id);
    for (const g of games.value) {
      if (g.group_id === id) g.group_id = null;
    }
  }

  async function reorderGroups(orderedIds: number[]) {
    await invoke("reorder_groups", { orderedIds });
    await loadGroups();
  }

  async function exportData(): Promise<string> {
    return await invoke<string>("export_data");
  }

  async function importData(json: string) {
    await invoke("import_data", { json });
    await Promise.all([loadGames(), loadGroups(), loadSettings(), loadPasswords()]);
  }

  // ===== Batch Operations =====

  function toggleSelectGame(id: number) {
    const s = new Set(selectedGameIds.value);
    if (s.has(id)) s.delete(id);
    else s.add(id);
    selectedGameIds.value = s;
    if (s.size > 0) isSelectMode.value = true;
    else isSelectMode.value = false;
  }

  function selectAll() {
    const ids = new Set(filteredGames.value.map((g) => g.id));
    selectedGameIds.value = ids;
    isSelectMode.value = true;
  }

  function clearSelection() {
    selectedGameIds.value = new Set();
    isSelectMode.value = false;
  }

  async function batchDeleteGames() {
    const ids = Array.from(selectedGameIds.value);
    if (ids.length === 0) return;
    await invoke("batch_delete_games", { ids });
    selectedGameIds.value = new Set();
    isSelectMode.value = false;
    if (selectedGameId.value !== null && ids.includes(selectedGameId.value)) {
      selectedGameId.value = null;
    }
    // Optimistic update
    const idSet = new Set(ids);
    games.value = games.value.filter((g) => !idSet.has(g.id));
  }

  async function batchMoveGames(groupId: number | null) {
    const ids = Array.from(selectedGameIds.value);
    if (ids.length === 0) return;
    await invoke("batch_set_game_group", { gameIds: ids, groupId });
    selectedGameIds.value = new Set();
    isSelectMode.value = false;
    // Optimistic update
    const idSet = new Set(ids);
    for (const g of games.value) {
      if (idSet.has(g.id)) g.group_id = groupId;
    }
  }

  async function batchScanCovers(gameIds: number[]): Promise<number> {
    return await invoke<number>("batch_scan_covers", { gameIds }, { taskKey: "scan-covers" });
  }

  async function batchSetStatus(status: string) {
    const ids = Array.from(selectedGameIds.value);
    if (ids.length === 0) return;
    await invoke("batch_set_game_status", { gameIds: ids, status });
    // Optimistic update
    const idSet = new Set(ids);
    for (const g of games.value) {
      if (idSet.has(g.id)) g.status = status;
    }
  }

  async function batchSetRating(rating: number) {
    const ids = Array.from(selectedGameIds.value);
    if (ids.length === 0) return;
    await invoke("batch_set_game_rating", { gameIds: ids, rating });
    // Optimistic update
    const idSet = new Set(ids);
    for (const g of games.value) {
      if (idSet.has(g.id)) g.rating = rating;
    }
  }

  async function getPlaySessions(gameId: number, limit = 10): Promise<PlaySession[]> {
    return await invoke<PlaySession[]>("get_play_sessions", { gameId, limit });
  }

  async function getPlayCalendar(year: number, month: number): Promise<{ date: string; duration: number }[]> {
    return await invoke<{ date: string; duration: number }[]>("get_play_calendar", { year, month });
  }

  async function saveSettings(sevenZipPath: string, defaultExtractPath: string) {
    await invoke("save_settings", {
      sevenZipPath,
      defaultExtractPath,
    });
    settings.value.seven_zip_path = sevenZipPath;
    settings.value.default_extract_path = defaultExtractPath;
  }

  async function saveCustomImage(key: string, path: string): Promise<string> {
    // 后端会将图片复制到数据目录（asset scope 内）并返回内部路径
    const internal = await invoke<string>("save_custom_image", { key, path });
    const imageKeys = ["custom_banner", "custom_sidebar_bg", "custom_empty_illustration"] as const;
    if (imageKeys.includes(key as typeof imageKeys[number])) {
      (settings.value as Record<string, string>)[key] = internal;
    }
    return internal;
  }

  async function saveTheme(theme: string) {
    await invoke("save_theme", { theme });
    settings.value.theme = theme;
  }

  async function saveCloseBehavior(closeBehavior: string) {
    await invoke("save_close_behavior", { closeBehavior });
    settings.value.close_behavior = closeBehavior;
  }

  async function addPassword(password: string) {
    await invoke("add_password", { password });
    await loadPasswords();
  }

  async function removePassword(password: string) {
    await invoke("remove_password", { password });
    await loadPasswords();
  }

  // ===== Tags =====

  async function createTag(name: string): Promise<number> {
    const id = await invoke<number>("add_tag", { name });
    await loadTags();
    return id;
  }

  async function deleteTag(id: number) {
    await invoke("delete_tag", { id });
    await loadTags();
  }

  async function renameTag(id: number, name: string) {
    await invoke("rename_tag", { id, name });
    await loadTags();
  }

  async function getTagUsage(): Promise<TagUsage[]> {
    return await invoke<TagUsage[]>("get_tag_usage");
  }

  async function addGameTag(gameId: number, tagId: number) {
    await invoke("add_game_tag", { gameId, tagId });
    await loadGameTags(gameId);
  }

  async function removeGameTag(gameId: number, tagId: number) {
    await invoke("remove_game_tag", { gameId, tagId });
    await loadGameTags(gameId);
  }

  // ===== Screenshots =====

  async function getGameScreenshots(gameId: number): Promise<{ id: number; path: string }[]> {
    return await invoke<{ id: number; path: string }[]>("get_game_screenshots", { gameId });
  }

  async function addGameScreenshot(gameId: number, path: string): Promise<number> {
    return await invoke<number>("add_game_screenshot", { gameId, path });
  }

  async function deleteGameScreenshot(screenshotId: number): Promise<void> {
    await invoke("delete_game_screenshot", { screenshotId });
  }

  // ===== Update =====

  async function checkForUpdate(): Promise<UpdateInfo> {
    return await invoke<UpdateInfo>("check_for_update");
  }

  async function saveUpdateRepo(updateRepo: string) {
    await invoke("save_update_repo", { updateRepo });
    settings.value.update_repo = updateRepo;
  }

  async function saveIgdbSettings(clientId: string, clientSecret: string) {
    await invoke("save_setting", { key: "igdb_client_id", value: clientId });
    await invoke("save_setting", { key: "igdb_client_secret", value: clientSecret });
    settings.value.igdb_client_id = clientId;
    settings.value.igdb_client_secret = clientSecret;
  }

  async function backupDatabase(): Promise<string> {
    return await invoke<string>("backup_database");
  }

  // ===== Reorder Games =====

  async function reorderGames(gameIds: number[]) {
    await invoke("reorder_games", { gameIds });
    await loadGames();
  }

  return {
    games, groups, settings, passwords, tags, gameTags,
    selectedGroupId, selectedGameId, searchInput, searchKeyword, sortType, selectedTagId, selectedStatus,
    selectedGameIds, isSelectMode,
    filteredGames, selectedGame,
    loadGames, loadGroups, loadSettings, loadPasswords, loadTags, loadGameTags, loadAllGameTags,
    addGame, updateGame, deleteGame, setGameGroup, setGameStatus, setGameRating, setGamePlayTime, setGameCover, launchGame,
    loadLaunchActions, saveLaunchActions,
    addGroup, renameGroup, deleteGroup, reorderGroups,
    saveSettings, saveCustomImage, saveTheme, saveCloseBehavior,
    addPassword, removePassword,
    exportData, importData,
    toggleSelectGame, selectAll, clearSelection,
    batchDeleteGames, batchMoveGames, batchScanCovers, batchSetStatus, batchSetRating,
    getPlaySessions, getPlayCalendar,
    createTag, deleteTag, renameTag, getTagUsage, addGameTag, removeGameTag,
    getGameScreenshots, addGameScreenshot, deleteGameScreenshot,
    checkForUpdate, saveUpdateRepo, saveIgdbSettings, backupDatabase, reorderGames,
  };
});
