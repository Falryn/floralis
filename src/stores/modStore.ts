/**
 * Mod 状态管理
 * 
 * 使用 Pinia 管理 Mod（模组）相关的全局状态和操作方法
 * 提供与 Rust 后端通信的所有 Mod 操作方法
 */

import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "../utils/invoke";
import type { Mod, ModProfile, ApplyProfileResult, Tag } from "../types";

/**
 * Mod 状态 Store
 * 
 * 管理所有 Mod 相关的全局状态和操作方法
 */
export const useModStore = defineStore("mod", () => {
  // 状态
  const mods = ref<Mod[]>([]);
  const selectedModId = ref<number | null>(null);
  const modSearchKeyword = ref("");
  const modFilterGameId = ref<number | null>(null);
  const modFilterTagId = ref<number | null>(null);
  const modFilterEnabled = ref<string | null>(null); // null=全部, 'enabled'=已启用, 'disabled'=已禁用
  const modFilterCategories = ref<string[]>([]); // 空数组=全部分类
  const modSortType = ref<string>('recent'); // recent, name_asc, name_desc, enabled
  const modTags = ref<Map<number, Tag[]>>(new Map());

  // Integrity: ids of mods whose file is missing on disk
  const modMissingIds = ref<Set<number>>(new Set());
  const modFilterMissing = ref(false);

  // Profiles
  const modProfiles = ref<ModProfile[]>([]);
  const activeProfileId = ref<number | null>(null);

  // Multi-select mode
  const isModSelectMode = ref(false);
  const selectedModIds = ref<Set<number>>(new Set());

  // 计算属性
  const filteredMods = computed(() => {
    let result = mods.value;
    if (modSearchKeyword.value) {
      const kw = modSearchKeyword.value.toLowerCase();
      result = result.filter(m =>
        m.name.toLowerCase().includes(kw) ||
        m.author.toLowerCase().includes(kw) ||
        m.description.toLowerCase().includes(kw)
      );
    }
    if (modFilterGameId.value !== null) {
      if (modFilterGameId.value === -1) {
        // -1 = independent mods (no game linked)
        result = result.filter(m => m.game_id === null);
      } else {
        result = result.filter(m => m.game_id === modFilterGameId.value);
      }
    }
    if (modFilterEnabled.value !== null && modFilterEnabled.value !== 'all') {
      const enabled = modFilterEnabled.value === 'enabled';
      result = result.filter(m => m.is_enabled === enabled);
    }
    if (modFilterMissing.value) {
      result = result.filter(m => modMissingIds.value.has(m.id));
    }
    if (modFilterCategories.value.length > 0) {
      result = result.filter(m => modFilterCategories.value.includes(m.category));
    }
    if (modFilterTagId.value !== null) {
      result = result.filter(m => {
        const tags = modTags.value.get(m.id);
        return tags?.some(t => t.id === modFilterTagId.value) ?? false;
      });
    }
    // Sort
    switch (modSortType.value) {
      case 'name_asc':
        result = [...result].sort((a, b) => a.name.localeCompare(b.name));
        break;
      case 'name_desc':
        result = [...result].sort((a, b) => b.name.localeCompare(a.name));
        break;
      case 'enabled':
        result = [...result].sort((a, b) => (b.is_enabled ? 1 : 0) - (a.is_enabled ? 1 : 0));
        break;
      case 'recent':
      default:
        // Keep original order (by id desc, newest first)
        break;
    }
    return result;
  });

  const selectedMod = computed(() =>
    mods.value.find(m => m.id === selectedModId.value) || null
  );

  const modsByGame = computed(() => {
    const map = new Map<number, Mod[]>();
    for (const mod of mods.value) {
      if (mod.game_id !== null) {
        const list = map.get(mod.game_id) || [];
        list.push(mod);
        map.set(mod.game_id, list);
      }
    }
    return map;
  });

  // Actions - 遵循 gameStore 的乐观更新模式
  async function loadMods() {
    mods.value = await invoke<Mod[]>("get_all_mods");
    await checkModIntegrity();
  }

  // ===== Integrity =====

  async function checkModIntegrity() {
    const missing = await invoke<number[]>("check_mods_integrity");
    modMissingIds.value = new Set(missing);
  }

  async function addMod(mod: Omit<Mod, "id" | "created_at" | "updated_at">) {
    const id = await invoke<number>("add_mod", {
      name: mod.name,
      description: mod.description,
      modPath: mod.mod_path,
      installPath: mod.install_path,
      gameId: mod.game_id,
      gameDir: mod.game_dir,
      version: mod.version,
      author: mod.author,
      isEnabled: mod.is_enabled,
      sortOrder: mod.sort_order,
      category: mod.category,
      sourceUrl: mod.source_url,
      coverPath: mod.cover_path ?? "",
      modType: mod.mod_type ?? "file",
      originalName: mod.original_name ?? "",
    });
    // Optimistic update: append new mod to list
    mods.value.unshift({ ...mod, id, sort_order: 0, created_at: "", updated_at: "" } as Mod);
    return id;
  }

  async function updateMod(id: number, mod: Omit<Mod, "id" | "created_at" | "updated_at">) {
    await invoke("update_mod", {
      id,
      name: mod.name,
      description: mod.description,
      modPath: mod.mod_path,
      installPath: mod.install_path,
      gameId: mod.game_id,
      gameDir: mod.game_dir,
      version: mod.version,
      author: mod.author,
      isEnabled: mod.is_enabled,
      sortOrder: mod.sort_order,
      category: mod.category,
      sourceUrl: mod.source_url,
      coverPath: mod.cover_path ?? "",
      modType: mod.mod_type ?? "file",
      originalName: mod.original_name ?? "",
    });
    // Optimistic update: modify in place
    const idx = mods.value.findIndex((m) => m.id === id);
    if (idx !== -1) {
      mods.value[idx] = { ...mods.value[idx], ...mod };
    }
  }

  async function deleteMod(id: number) {
    await invoke("delete_mod", { id });
    if (selectedModId.value === id) selectedModId.value = null;
    // Optimistic update: remove from list
    mods.value = mods.value.filter((m) => m.id !== id);
  }

  async function toggleModEnabled(id: number) {
    await invoke("toggle_mod_enabled", { id });
    // Optimistic update
    const mod = mods.value.find((m) => m.id === id);
    if (mod) mod.is_enabled = !mod.is_enabled;
    await checkModIntegrity();
  }

  async function reorderMods(modIds: number[]) {
    await invoke("reorder_mods", { modIds });
    await loadMods();
  }

  async function getModsByGame(gameId: number): Promise<Mod[]> {
    return await invoke<Mod[]>("get_mods_by_game", { gameId });
  }

  async function linkModToGame(modId: number, gameId: number) {
    await invoke("link_mod_to_game", { modId, gameId });
    // Optimistic update
    const mod = mods.value.find((m) => m.id === modId);
    if (mod) mod.game_id = gameId;
  }

  async function unlinkModFromGame(modId: number) {
    await invoke("unlink_mod_from_game", { modId });
    // Optimistic update
    const mod = mods.value.find((m) => m.id === modId);
    if (mod) mod.game_id = null;
  }

  async function getModTags(modId: number): Promise<Tag[]> {
    return await invoke<Tag[]>("get_mod_tags", { modId });
  }

  async function loadAllModTags() {
    const result = await invoke<Record<number, Tag[]>>("get_all_mod_tags");
    const m = new Map<number, Tag[]>();
    for (const [key, val] of Object.entries(result)) {
      m.set(Number(key), val);
    }
    modTags.value = m;
  }

  async function setModTags(modId: number, tagIds: number[]) {
    await invoke("set_mod_tags", { modId, tagIds });
    // 同步刷新全局 modTags 缓存
    const tags = await invoke<Tag[]>("get_mod_tags", { modId });
    const m = new Map(modTags.value);
    m.set(modId, tags);
    modTags.value = m;
  }

  // ===== Batch Operations =====

  function toggleSelectMod(id: number) {
    const s = new Set(selectedModIds.value);
    if (s.has(id)) s.delete(id);
    else s.add(id);
    selectedModIds.value = s;
    isModSelectMode.value = s.size > 0;
  }

  function selectAllMods() {
    const ids = new Set(filteredMods.value.map((m) => m.id));
    selectedModIds.value = ids;
    isModSelectMode.value = true;
  }

  function clearModSelection() {
    selectedModIds.value = new Set();
    isModSelectMode.value = false;
  }

  async function batchDeleteMods() {
    const ids = Array.from(selectedModIds.value);
    if (ids.length === 0) return;
    for (const id of ids) {
      await invoke("delete_mod", { id });
    }
    const idSet = new Set(ids);
    mods.value = mods.value.filter((m) => !idSet.has(m.id));
    if (selectedModId.value !== null && idSet.has(selectedModId.value)) {
      selectedModId.value = null;
    }
    clearModSelection();
  }

  async function batchToggleEnabled(enabled: boolean) {
    const ids = Array.from(selectedModIds.value);
    if (ids.length === 0) return;
    for (const id of ids) {
      const mod = mods.value.find((m) => m.id === id);
      if (mod && mod.is_enabled !== enabled) {
        await invoke("toggle_mod_enabled", { id });
        mod.is_enabled = enabled;
      }
    }
    clearModSelection();
    await checkModIntegrity();
  }

  async function batchSetModTags(tagIds: number[]) {
    const ids = Array.from(selectedModIds.value);
    if (ids.length === 0) return;
    for (const id of ids) {
      await invoke("set_mod_tags", { modId: id, tagIds });
      const tags = await invoke<Tag[]>("get_mod_tags", { modId: id });
      const m = new Map(modTags.value);
      m.set(id, tags);
      modTags.value = m;
    }
    clearModSelection();
  }

  // ===== Mod Profiles =====

  async function loadProfiles(gameId: number) {
    modProfiles.value = await invoke<ModProfile[]>("get_mod_profiles", { gameId });
  }

  async function createProfile(gameId: number, name: string, modIds: number[]) {
    await invoke<number>("create_mod_profile", { gameId, name, modIds });
    await loadProfiles(gameId);
  }

  async function renameProfile(id: number, gameId: number, name: string) {
    await invoke("rename_mod_profile", { id, name });
    await loadProfiles(gameId);
  }

  async function deleteProfile(id: number, gameId: number) {
    await invoke("delete_mod_profile", { id });
    if (activeProfileId.value === id) activeProfileId.value = null;
    await loadProfiles(gameId);
  }

  async function setProfileMods(profileId: number, gameId: number, modIds: number[]) {
    await invoke("set_mod_profile_mods", { profileId, modIds });
    await loadProfiles(gameId);
  }

  async function applyProfile(profileId: number): Promise<ApplyProfileResult> {
    const result = await invoke<ApplyProfileResult>("apply_mod_profile", { profileId });
    activeProfileId.value = profileId;
    await loadMods();
    return result;
  }

  return {
    mods, selectedModId, modSearchKeyword, modFilterGameId, modFilterTagId, modFilterEnabled, modFilterCategories, modSortType,
    modTags, isModSelectMode, selectedModIds,
    modMissingIds, modFilterMissing,
    modProfiles, activeProfileId,
    filteredMods, selectedMod, modsByGame,
    loadMods, checkModIntegrity,
    addMod, updateMod, deleteMod, toggleModEnabled, reorderMods,
    getModsByGame, linkModToGame, unlinkModFromGame,
    getModTags, loadAllModTags, setModTags,
    toggleSelectMod, selectAllMods, clearModSelection,
    batchDeleteMods, batchToggleEnabled, batchSetModTags,
    loadProfiles, createProfile, renameProfile, deleteProfile, setProfileMods, applyProfile,
  };
});
