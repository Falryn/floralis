<script setup lang="ts">
import { ref, computed, onMounted, watch, watchEffect } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "../utils/invoke";
import { useModStore } from "../stores/modStore";
import { useGameStore, loadImage } from "../stores/gameStore";
import { useI18n } from "vue-i18n";
import { addToast } from "../composables/useToast";
import type { Mod, Tag } from "../types";
import CustomSelect from "./CustomSelect.vue";
import { MOD_CATEGORIES } from "../utils/mod";

const { t } = useI18n();

const props = defineProps<{
  mod?: Mod | null;
  initialTab?: 'basic' | 'assoc';
}>();

const emit = defineEmits<{
  close: [];
  saved: [];
}>();

const modStore = useModStore();
const gameStore = useGameStore();

const isEdit = computed(() => !!props.mod);

// Tab state: basic + assoc (merged path & association)
const activeTab = ref<'basic' | 'assoc'>(props.initialTab ?? 'basic');

// Form fields
const name = ref(props.mod?.name ?? "");
const nameError = ref("");
const version = ref(props.mod?.version ?? "");
const author = ref(props.mod?.author ?? "");
const description = ref(props.mod?.description ?? "");
const modPath = ref(props.mod?.mod_path ?? "");
const gameDir = ref(props.mod?.game_dir ?? "");
const selectedGameId = ref<number | null>(props.mod?.game_id ?? null);
const category = ref(props.mod?.category ?? "");
const sourceUrl = ref(props.mod?.source_url ?? "");
const coverPath = ref(props.mod?.cover_path ?? "");

// Cover preview
const coverPreviewUrl = ref("");
watchEffect(async () => {
  if (coverPath.value) {
    coverPreviewUrl.value = await loadImage(coverPath.value) || "";
  } else {
    coverPreviewUrl.value = "";
  }
});

const categoryOptions = computed(() => [
  { label: t('mod.noCategory'), value: '' },
  ...MOD_CATEGORIES.map(c => ({ label: t(`mod.cat.${c}`), value: c })),
]);

// Tags
const modTags = ref<Tag[]>([]);
const newTagName = ref("");
const showTagPicker = ref(false);

const gameOptions = computed(() => [
  { label: t("mod.noGame"), value: null as number | null },
  ...gameStore.games.map((g) => ({ label: g.name, value: g.id as number | null })),
]);

const availableTags = computed(() => {
  const current = new Set(modTags.value.map((t) => t.id));
  return gameStore.tags.filter((t) => !current.has(t.id));
});

onMounted(async () => {
  if (props.mod) {
    modTags.value = await modStore.getModTags(props.mod.id);
  }
});

async function selectModPath() {
  const path = await open({ directory: true, multiple: false });
  if (path) modPath.value = path as string;
}

async function selectGameDir() {
  const path = await open({ directory: true, multiple: false });
  if (path) gameDir.value = path as string;
}

async function selectCover() {
  const path = await open({
    filters: [{ name: t('mod.coverImage'), extensions: ["jpg", "jpeg", "png", "webp"] }],
    multiple: false,
    directory: false,
  });
  if (path) {
    try {
      const stored = await invoke<string>("copy_mod_cover_to_storage", {
        sourcePath: path as string,
        modId: props.mod?.id ?? null,
      });
      coverPath.value = stored;
    } catch (e) {
      console.error("Mod cover copy failed:", e);
      coverPath.value = path as string;
    }
  }
}

function removeCover() {
  coverPath.value = "";
}

// Auto-fill game_dir & mod_path when game is selected
watch(selectedGameId, (newGameId) => {
  if (newGameId !== null) {
    const game = gameStore.games.find((g) => g.id === newGameId);
    if (game && game.install_path) {
      gameDir.value = game.install_path;
      // Auto-fill mod path from game's default_mod_dir
      const defaultDir = game.default_mod_dir?.trim();
      if (defaultDir) {
        // Absolute path → use directly; relative → join with install_path
        const isAbsolute = /^([A-Za-z]:[\\/]|\\\\|\/)/.test(defaultDir);
        modPath.value = isAbsolute
          ? defaultDir
          : `${game.install_path.replace(/[\\/]+$/, "")}\\${defaultDir}`;
      } else {
        modPath.value = `${game.install_path.replace(/[\\/]+$/, "")}\\mods`;
      }
    }
  }
});

async function handleAddTag() {
  const tagName = newTagName.value.trim();
  if (!tagName) return;
  const existing = gameStore.tags.find((t) => t.name === tagName);
  let tagId: number;
  if (existing) {
    tagId = existing.id;
  } else {
    tagId = await gameStore.createTag(tagName);
  }
  if (!modTags.value.find((t) => t.id === tagId)) {
    modTags.value.push({ id: tagId, name: tagName });
  }
  newTagName.value = "";
  showTagPicker.value = false;
}

function handleRemoveTag(tagId: number) {
  modTags.value = modTags.value.filter((t) => t.id !== tagId);
}

function handlePickTag(tag: Tag) {
  if (!modTags.value.find((t) => t.id === tag.id)) {
    modTags.value.push(tag);
  }
  showTagPicker.value = false;
}

async function save() {
  if (!name.value.trim()) {
    nameError.value = t('mod.nameRequired');
    return;
  }
  nameError.value = '';

  const modData = {
    name: name.value.trim(),
    version: version.value.trim(),
    author: author.value.trim(),
    description: description.value.trim(),
    mod_path: modPath.value,
    install_path: props.mod?.install_path ?? "",
    game_dir: gameDir.value,
    game_id: selectedGameId.value,
    is_enabled: props.mod?.is_enabled ?? true,
    sort_order: props.mod?.sort_order ?? 0,
    category: category.value,
    source_url: sourceUrl.value,
    cover_path: coverPath.value,
    mod_type: props.mod?.mod_type ?? "file",
    original_name: props.mod?.original_name ?? "",
  };

  try {
    let modId: number;
    if (isEdit.value && props.mod) {
      await modStore.updateMod(props.mod.id, modData);
      modId = props.mod.id;
    } else {
      modId = await modStore.addMod(modData);
    }

    // Save tags
    const tagIds = modTags.value.map((t) => t.id);
    await modStore.setModTags(modId, tagIds);

    emit("saved");
  } catch (e) {
    console.error('Failed to save mod:', e);
    addToast(t('mod.saveFailed'), 'error');
  }
}
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm"
    @click.self="emit('close')"
  >
    <div class="modal-panel bg-modal-bg rounded-3xl shadow-2xl w-[900px] max-h-[96vh] flex flex-col">
      <!-- Header -->
      <div class="flex items-center justify-between px-8 py-5 border-b border-border-light">
        <h2 class="text-lg font-bold text-text-main">
          {{ isEdit ? t("mod.editMod") : t("mod.addMod") }}
        </h2>
        <button
          class="p-1.5 rounded-lg hover:bg-primary-50 dark:hover:bg-primary-900/30 text-text-sub transition-colors"
          @click="emit('close')"
        >
          ✕
        </button>
      </div>

      <div class="flex-1 overflow-auto px-8 py-5 space-y-5">
        <!-- Tab Bar -->
        <div class="flex gap-1 p-1 rounded-xl bg-input-bg/50 border border-border-light">
          <button
            class="flex-1 px-3 py-2 rounded-lg text-sm font-medium transition-all"
            :class="activeTab === 'basic' ? 'bg-primary-500 text-white shadow-sm' : 'text-text-sub hover:text-text-main hover:bg-overlay-white'"
            @click="activeTab = 'basic'"
          >
            {{ t('mod.basicInfo') }}
          </button>
          <button
            class="flex-1 px-3 py-2 rounded-lg text-sm font-medium transition-all"
            :class="activeTab === 'assoc' ? 'bg-primary-500 text-white shadow-sm' : 'text-text-sub hover:text-text-main hover:bg-overlay-white'"
            @click="activeTab = 'assoc'"
          >
            {{ t('mod.assocAndPath') }}
          </button>
        </div>

        <!-- Tab: Basic Info -->
        <div v-show="activeTab === 'basic'" class="space-y-4">
          <!-- Name + Version + Author -->
          <div class="flex gap-4">
            <div class="flex-1 min-w-0">
              <label class="block text-sm font-medium text-text-main mb-1.5">{{ t("mod.name") }} *</label>
              <input
                v-model="name"
                class="w-full px-3 py-2.5 text-sm rounded-xl border outline-none focus:border-primary-400 transition-colors"
                :class="nameError ? 'border-red-400' : 'border-primary-200'"
                :placeholder="t('mod.name')"
                @input="nameError = ''"
                @keyup.enter="save"
              />
              <p v-if="nameError" class="text-red-500 dark:text-red-400 text-xs mt-1">{{ nameError }}</p>
            </div>
            <div class="w-28 shrink-0">
              <label class="block text-sm font-medium text-text-main mb-1.5">{{ t("mod.version") }}</label>
              <input
                v-model="version"
                class="w-full px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors"
                :placeholder="t('mod.version')"
              />
            </div>
            <div class="w-44 shrink-0">
              <label class="block text-sm font-medium text-text-main mb-1.5">{{ t("mod.author") }}</label>
              <input
                v-model="author"
                class="w-full px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors"
                :placeholder="t('mod.author')"
              />
            </div>
          </div>

          <!-- Category + Source URL -->
          <div class="flex gap-4">
            <div class="w-44 shrink-0">
              <label class="block text-sm font-medium text-text-main mb-1.5">{{ t("mod.category") }}</label>
              <CustomSelect
                v-model="category"
                :options="categoryOptions"
              />
            </div>
            <div class="flex-1 min-w-0">
              <label class="block text-sm font-medium text-text-main mb-1.5">
                <span class="inline-flex items-center gap-1">
                  {{ t("mod.sourceUrl") }}
                  <svg class="w-3 h-3 text-text-sub/40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M10 13a5 5 0 007.54.54l3-3a5 5 0 00-7.07-7.07l-1.72 1.71"/>
                    <path d="M14 11a5 5 0 00-7.54-.54l-3 3a5 5 0 007.07 7.07l1.71-1.71"/>
                  </svg>
                </span>
              </label>
              <input
                v-model="sourceUrl"
                class="w-full px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors"
                :placeholder="t('mod.sourceUrlPlaceholder')"
              />
            </div>
          </div>

          <!-- Cover + Description side by side -->
          <div class="flex gap-4 items-start">
            <div class="w-52 shrink-0">
              <label class="block text-sm font-medium text-text-main mb-1.5">{{ t("mod.coverImage") }}</label>
              <div class="h-[74px] rounded-xl border border-dashed border-border-medium overflow-hidden flex items-center justify-center bg-input-bg/50">
                <img v-if="coverPreviewUrl" :src="coverPreviewUrl" class="w-full h-full object-cover" />
                <span v-else class="text-2xl opacity-30">🖼️</span>
              </div>
              <div class="flex gap-2 mt-2">
                <button
                  class="flex-1 px-2 py-1.5 text-xs rounded-lg border border-primary-200 text-text-sub hover:bg-primary-50 dark:hover:bg-primary-900/30 transition-colors"
                  @click="selectCover"
                >
                  {{ t("mod.selectCover") }}
                </button>
                <button
                  v-if="coverPath"
                  class="flex-1 px-2 py-1.5 text-xs rounded-lg border border-red-200 dark:border-red-800 text-red-400 hover:bg-red-50 dark:hover:bg-red-900/30 transition-colors"
                  @click="removeCover"
                >
                  {{ t("mod.removeCover") }}
                </button>
              </div>
            </div>
            <div class="flex-1 min-w-0">
              <label class="block text-sm font-medium text-text-main mb-1.5">{{ t("mod.description") }}</label>
              <textarea
                v-model="description"
                rows="4"
                class="w-full px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors resize-none"
                :placeholder="t('mod.descriptionPlaceholder')"
              ></textarea>
            </div>
          </div>

          <!-- Tags -->
          <div>
            <label class="block text-sm font-medium text-text-main mb-1.5">{{ t("mod.tags") }}</label>
            <div class="flex items-center gap-2">
              <div class="flex flex-wrap gap-1.5 flex-1 min-w-0">
                <span
                  v-for="tag in modTags"
                  :key="tag.id"
                  class="inline-flex items-center gap-1 px-2.5 py-1 rounded-lg bg-primary-50 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 text-xs font-medium"
                >
                  {{ tag.name }}
                  <button
                    class="hover:text-red-500 dark:hover:text-red-400 transition-colors leading-none"
                    @click="handleRemoveTag(tag.id)"
                  >
                    ×
                  </button>
                </span>
                <span
                  v-if="modTags.length === 0"
                  class="text-xs text-text-sub italic leading-7"
                >
                  {{ t("mod.noTags") }}
                </span>
              </div>
              <input
                v-model="newTagName"
                :placeholder="t('mod.newTag')"
                class="w-36 px-3 py-1.5 text-xs rounded-lg border border-primary-200 bg-input-bg text-text-main placeholder-text-sub/50 outline-none focus:border-primary-400 transition-colors shrink-0"
                @keyup.enter="handleAddTag"
              />
              <button
                class="px-3 py-1.5 text-xs rounded-lg bg-primary-500 text-white hover:bg-primary-600 transition-colors shrink-0"
                @click="handleAddTag"
              >
                {{ t("mod.addTag") }}
              </button>
              <button
                class="px-3 py-1.5 text-xs rounded-lg border border-primary-200 text-text-sub hover:bg-primary-50 dark:hover:bg-primary-900/30 transition-colors shrink-0"
                @click="showTagPicker = !showTagPicker"
              >
                {{ t("mod.selectTag") }}
              </button>
            </div>
            <div
              v-if="showTagPicker && availableTags.length > 0"
              class="mt-2 flex flex-wrap gap-1.5"
            >
              <button
                v-for="tag in availableTags"
                :key="tag.id"
                class="px-2.5 py-1 rounded-lg border border-primary-200 text-xs text-text-sub hover:bg-primary-50 dark:hover:bg-primary-900/30 hover:text-primary-600 hover:border-primary-300 transition-colors"
                @click="handlePickTag(tag)"
              >
                + {{ tag.name }}
              </button>
            </div>
          </div>
        </div>

        <!-- Tab: Association & Path -->
        <div v-show="activeTab === 'assoc'" class="space-y-6">
          <!-- Linked Game (first) -->
          <div>
            <label class="block text-sm font-medium text-text-main mb-1.5">{{ t("mod.linkedGame") }}</label>
            <CustomSelect
              v-model="selectedGameId"
              :options="gameOptions"
              searchable
            />
          </div>

          <!-- Game Directory (auto-filled) -->
          <div>
            <label class="block text-sm font-medium text-text-main mb-1.5">{{ t("mod.gameDir") }}</label>
            <div class="flex gap-2">
              <input
                v-model="gameDir"
                class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
                :placeholder="t('mod.gameDir')"
              />
              <button
                class="px-4 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 dark:hover:bg-primary-900/30 transition-colors shrink-0"
                @click="selectGameDir"
              >
                {{ t("mod.browse") }}
              </button>
            </div>
          </div>

          <!-- Mod Path (auto-filled) -->
          <div>
            <label class="block text-sm font-medium text-text-main mb-1.5">{{ t("mod.modPath") }}</label>
            <div class="flex gap-2">
              <input
                v-model="modPath"
                class="flex-1 px-3 py-2.5 text-sm rounded-xl border border-primary-200 outline-none focus:border-primary-400 transition-colors min-w-0"
                :placeholder="t('mod.modPath')"
              />
              <button
                class="px-4 py-2.5 text-sm rounded-xl border border-primary-200 text-text-sub hover:bg-primary-50 dark:hover:bg-primary-900/30 transition-colors shrink-0"
                @click="selectModPath"
              >
                {{ t("mod.browse") }}
              </button>
            </div>
          </div>
        </div>

        <!-- Actions -->
        <div class="flex gap-2 pt-2">
          <div class="flex-1"></div>
          <button
            class="px-4 py-2.5 rounded-xl border border-primary-200 text-sm text-text-sub hover:bg-primary-50 dark:hover:bg-primary-900/30 transition-colors"
            @click="emit('close')"
          >
            {{ t("common.cancel") }}
          </button>
          <button
            class="px-6 py-2.5 rounded-xl bg-gradient-to-r from-primary-500 to-primary-600 text-white text-sm font-medium hover:from-primary-600 hover:to-primary-700 transition-all shadow-sm"
            :disabled="!name.trim()"
            @click="save"
          >
            {{ t("common.confirm") }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
