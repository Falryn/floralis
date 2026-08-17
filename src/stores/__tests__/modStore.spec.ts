import { describe, it, expect, beforeEach, vi } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { useModStore } from "../modStore";
import type { Mod } from "../../types";

// Mock Tauri API
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

function makeMod(overrides: Partial<Mod> = {}): Mod {
  return {
    id: 1,
    name: "Test Mod",
    description: "",
    mod_path: "",
    install_path: "",
    game_id: null,
    game_dir: "",
    version: "",
    author: "",
    is_enabled: false,
    sort_order: 0,
    category: "",
    source_url: "",
    cover_path: "",
    mod_type: "file",
    original_name: "",
    created_at: "",
    updated_at: "",
    ...overrides,
  };
}

describe("modStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  describe("filteredMods", () => {
    it("filters by search keyword across name/author/description", () => {
      const store = useModStore();
      store.mods = [
        makeMod({ id: 1, name: "HD Textures" }),
        makeMod({ id: 2, author: "Alice" }),
        makeMod({ id: 3, description: "Adds HD lighting" }),
        makeMod({ id: 4, name: "Unrelated" }),
      ];
      store.modSearchKeyword = "hd";

      expect(store.filteredMods.map((m) => m.id)).toEqual([1, 3]);
    });

    it("filters by game id and independent mods (-1)", () => {
      const store = useModStore();
      store.mods = [
        makeMod({ id: 1, game_id: 10 }),
        makeMod({ id: 2, game_id: 20 }),
        makeMod({ id: 3, game_id: null }),
      ];

      store.modFilterGameId = 10;
      expect(store.filteredMods.map((m) => m.id)).toEqual([1]);

      store.modFilterGameId = -1;
      expect(store.filteredMods.map((m) => m.id)).toEqual([3]);
    });

    it("filters by enabled state", () => {
      const store = useModStore();
      store.mods = [
        makeMod({ id: 1, is_enabled: true }),
        makeMod({ id: 2, is_enabled: false }),
      ];

      store.modFilterEnabled = "enabled";
      expect(store.filteredMods.map((m) => m.id)).toEqual([1]);

      store.modFilterEnabled = "disabled";
      expect(store.filteredMods.map((m) => m.id)).toEqual([2]);
    });

    it("sorts by name ascending and descending", () => {
      const store = useModStore();
      store.mods = [
        makeMod({ id: 1, name: "Zebra" }),
        makeMod({ id: 2, name: "Apple" }),
        makeMod({ id: 3, name: "Mango" }),
      ];

      store.modSortType = "name_asc";
      expect(store.filteredMods.map((m) => m.name)).toEqual(["Apple", "Mango", "Zebra"]);

      store.modSortType = "name_desc";
      expect(store.filteredMods.map((m) => m.name)).toEqual(["Zebra", "Mango", "Apple"]);
    });
  });

  describe("modsByGame", () => {
    it("groups mods by game_id and excludes independent mods", () => {
      const store = useModStore();
      store.mods = [
        makeMod({ id: 1, game_id: 10 }),
        makeMod({ id: 2, game_id: 10 }),
        makeMod({ id: 3, game_id: null }),
      ];

      expect(store.modsByGame.get(10)?.map((m) => m.id)).toEqual([1, 2]);
      expect(store.modsByGame.size).toBe(1);
    });
  });

  describe("selection operations", () => {
    it("toggleSelectMod adds and removes from selection", () => {
      const store = useModStore();

      store.toggleSelectMod(1);
      expect(store.selectedModIds.has(1)).toBe(true);
      expect(store.isModSelectMode).toBe(true);

      store.toggleSelectMod(1);
      expect(store.selectedModIds.has(1)).toBe(false);
      expect(store.isModSelectMode).toBe(false);
    });

    it("selectAllMods selects all filtered mods", () => {
      const store = useModStore();
      store.mods = [makeMod({ id: 1 }), makeMod({ id: 2 })];

      store.selectAllMods();
      expect(store.selectedModIds.size).toBe(2);
      expect(store.isModSelectMode).toBe(true);

      store.clearModSelection();
      expect(store.selectedModIds.size).toBe(0);
      expect(store.isModSelectMode).toBe(false);
    });
  });

  describe("optimistic updates", () => {
    it("deleteMod removes the mod and clears selection", async () => {
      const store = useModStore();
      store.mods = [makeMod({ id: 1 }), makeMod({ id: 2 })];
      store.selectedModId = 1;
      vi.mocked(invoke).mockResolvedValue(undefined);

      await store.deleteMod(1);

      expect(invoke).toHaveBeenCalledWith("delete_mod", { id: 1 });
      expect(store.mods.map((m) => m.id)).toEqual([2]);
      expect(store.selectedModId).toBeNull();
    });

    it("toggleModEnabled flips is_enabled", async () => {
      const store = useModStore();
      store.mods = [makeMod({ id: 1, is_enabled: false })];
      vi.mocked(invoke).mockResolvedValue(undefined);

      await store.toggleModEnabled(1);

      expect(invoke).toHaveBeenCalledWith("toggle_mod_enabled", { id: 1 });
      expect(store.mods[0].is_enabled).toBe(true);
    });
  });
});
