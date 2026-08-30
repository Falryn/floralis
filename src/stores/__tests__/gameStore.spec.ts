import { describe, it, expect, beforeEach, vi } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { useGameStore } from "../gameStore";
import type { Game } from "../../types";

// Mock Tauri API
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((p: string) => `asset://localhost/${p}`),
}));

function makeGame(overrides: Partial<Game> = {}): Game {
  return {
    id: 1,
    name: "Test Game",
    group_id: null,
    install_path: "",
    exe_path: "",
    launch_args: "",
    cover_path: "",
    save_path: "",
    notes: "",
    script_path: "",
    script_args: "",
    total_play_time: 0,
    last_played_at: null,
    status: "not_played",
    rating: 0,
    sort_order: 0,
    default_mod_dir: "",
    mod_naming_pattern: "",
    mod_uses_load_order: false,
    tracked_process_name: "",
    ...overrides,
  };
}

describe("gameStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  describe("filteredGames", () => {
    it("returns all games sorted by id desc by default", () => {
      const store = useGameStore();
      store.games = [
        makeGame({ id: 1, name: "Alpha" }),
        makeGame({ id: 3, name: "Charlie" }),
        makeGame({ id: 2, name: "Bravo" }),
      ];

      const result = store.filteredGames;
      expect(result.map((g) => g.id)).toEqual([3, 2, 1]);
    });

    it("filters by group", () => {
      const store = useGameStore();
      store.games = [
        makeGame({ id: 1, group_id: 10 }),
        makeGame({ id: 2, group_id: 20 }),
        makeGame({ id: 3, group_id: 10 }),
      ];
      store.selectedGroupId = 10;

      const result = store.filteredGames;
      expect(result).toHaveLength(2);
      expect(result.every((g) => g.group_id === 10)).toBe(true);
    });

    it("filters by search keyword", () => {
      const store = useGameStore();
      store.games = [
        makeGame({ id: 1, name: "The Witcher 3" }),
        makeGame({ id: 2, name: "Cyberpunk 2077" }),
        makeGame({ id: 3, name: "Witcher Tales" }),
      ];
      store.searchKeyword = "witcher";

      const result = store.filteredGames;
      expect(result).toHaveLength(2);
      expect(result.map((g) => g.name)).toContain("The Witcher 3");
      expect(result.map((g) => g.name)).toContain("Witcher Tales");
    });

    it("sorts by name ascending", () => {
      const store = useGameStore();
      store.games = [
        makeGame({ id: 1, name: "Zelda" }),
        makeGame({ id: 2, name: "Ark" }),
        makeGame({ id: 3, name: "Metro" }),
      ];
      store.sortType = "name_asc";

      const result = store.filteredGames;
      expect(result.map((g) => g.name)).toEqual(["Ark", "Metro", "Zelda"]);
    });

    it("filters by status", () => {
      const store = useGameStore();
      store.games = [
        makeGame({ id: 1, status: "completed" }),
        makeGame({ id: 2, status: "playing" }),
        makeGame({ id: 3, status: "completed" }),
      ];
      store.selectedStatus = "completed";

      const result = store.filteredGames;
      expect(result).toHaveLength(2);
      expect(result.every((g) => g.status === "completed")).toBe(true);
    });

    it("sorts by rating descending", () => {
      const store = useGameStore();
      store.games = [
        makeGame({ id: 1, rating: 3 }),
        makeGame({ id: 2, rating: 5 }),
        makeGame({ id: 3, rating: 0 }),
      ];
      store.sortType = "rating_desc";

      expect(store.filteredGames.map((g) => g.id)).toEqual([2, 1, 3]);
    });

    it("sorts by last played with never-played games last", () => {
      const store = useGameStore();
      store.games = [
        makeGame({ id: 1, last_played_at: null }),
        makeGame({ id: 2, last_played_at: "2026-01-01T00:00:00" }),
        makeGame({ id: 3, last_played_at: "2026-03-01T00:00:00" }),
      ];
      store.sortType = "last_played";

      expect(store.filteredGames.map((g) => g.id)).toEqual([3, 2, 1]);
    });

    it("filters by tag", () => {
      const store = useGameStore();
      store.games = [makeGame({ id: 1 }), makeGame({ id: 2 }), makeGame({ id: 3 })];
      store.gameTags = new Map([
        [1, [{ id: 10, name: "VN" }]],
        [3, [{ id: 10, name: "VN" }, { id: 11, name: "RPG" }]],
      ]);
      store.selectedTagId = 10;

      expect(store.filteredGames.map((g) => g.id).sort()).toEqual([1, 3]);
    });

    it("combines group and search filters", () => {
      const store = useGameStore();
      store.games = [
        makeGame({ id: 1, group_id: 10, name: "Fate Stay Night" }),
        makeGame({ id: 2, group_id: 10, name: "Steins;Gate" }),
        makeGame({ id: 3, group_id: 20, name: "Fate Hollow" }),
      ];
      store.selectedGroupId = 10;
      store.searchKeyword = "fate";

      expect(store.filteredGames.map((g) => g.id)).toEqual([1]);
    });
  });

  describe("selection operations", () => {
    it("toggleSelectGame adds and removes from selection", () => {
      const store = useGameStore();
      store.games = [makeGame({ id: 1 }), makeGame({ id: 2 })];

      store.toggleSelectGame(1);
      expect(store.selectedGameIds.has(1)).toBe(true);
      expect(store.isSelectMode).toBe(true);

      store.toggleSelectGame(1);
      expect(store.selectedGameIds.has(1)).toBe(false);
      expect(store.isSelectMode).toBe(false);
    });

    it("selectAll selects all filtered games", () => {
      const store = useGameStore();
      store.games = [makeGame({ id: 1 }), makeGame({ id: 2 }), makeGame({ id: 3 })];

      store.selectAll();
      expect(store.selectedGameIds.size).toBe(3);
      expect(store.isSelectMode).toBe(true);
    });

    it("clearSelection resets selection state", () => {
      const store = useGameStore();
      store.games = [makeGame({ id: 1 })];
      store.toggleSelectGame(1);

      store.clearSelection();
      expect(store.selectedGameIds.size).toBe(0);
      expect(store.isSelectMode).toBe(false);
    });
  });

  describe("selectedGame", () => {
    it("returns null when no game selected", () => {
      const store = useGameStore();
      store.games = [makeGame({ id: 1 })];
      expect(store.selectedGame).toBeNull();
    });

    it("returns the selected game", () => {
      const store = useGameStore();
      store.games = [makeGame({ id: 1, name: "Found" }), makeGame({ id: 2 })];
      store.selectedGameId = 1;
      expect(store.selectedGame?.name).toBe("Found");
    });
  });

  describe("reorderGames", () => {
    it("reorders locally in the given order", async () => {
      const store = useGameStore();
      store.games = [makeGame({ id: 1 }), makeGame({ id: 2 }), makeGame({ id: 3 })];

      await store.reorderGames([3, 1, 2]);
      expect(store.games.map((g) => g.id)).toEqual([3, 1, 2]);
    });
  });
});
