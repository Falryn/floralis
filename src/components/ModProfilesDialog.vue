<script setup lang="ts">
/**
 * Mod 配置文件管理对话框
 *
 * 按游戏维护多套 Mod 启用组合（Profile），支持应用、新建（保存当前启用组合）、
 * 重命名、删除与成员编辑。应用 = 将该游戏的 Mod 启用状态调整为组合记录的状态。
 */
import { ref, computed, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useModStore } from "../stores/modStore";
import { useGameStore } from "../stores/gameStore";
import { addToast } from "../composables/useToast";
import CustomSelect from "./CustomSelect.vue";
import ConfirmDialog from "./ConfirmDialog.vue";
import type { ModProfile } from "../types";

const emit = defineEmits<{
  close: [];
}>();

const { t } = useI18n();
const modStore = useModStore();
const gameStore = useGameStore();

// 游戏选择：默认当前筛选的游戏，否则第一个拥有 Mod 的游戏
const firstGameWithMods = gameStore.games.find(g => modStore.modsByGame.has(g.id));
const selectedGameId = ref<number | null>(
  modStore.modFilterGameId !== null && modStore.modFilterGameId > 0
    ? modStore.modFilterGameId
    : (firstGameWithMods?.id ?? gameStore.games[0]?.id ?? null)
);

const gameOptions = computed(() =>
  gameStore.games.map(g => ({ label: g.name, value: g.id as number | null }))
);

const gameMods = computed(() =>
  selectedGameId.value !== null ? (modStore.modsByGame.get(selectedGameId.value) ?? []) : []
);

watch(selectedGameId, async (gameId) => {
  if (gameId !== null) await modStore.loadProfiles(gameId);
  else modStore.modProfiles = [];
  editingProfileId.value = null;
  renamingId.value = null;
}, { immediate: true });

// ===== Apply =====

const applyingId = ref<number | null>(null);

async function applyProfile(profile: ModProfile) {
  applyingId.value = profile.id;
  try {
    const result = await modStore.applyProfile(profile.id);
    if (result.failures.length > 0) {
      addToast(t("mod.profileAppliedWithErrors", { count: result.failures.length }), "error");
    } else {
      addToast(t("mod.profileApplied", { count: result.changed }), "success");
    }
  } catch (e) {
    addToast(t("mod.applyFailed") + ": " + (e as string), "error");
  } finally {
    applyingId.value = null;
  }
}

// ===== Create from current enabled set =====

const newProfileName = ref("");

async function createFromCurrent() {
  if (selectedGameId.value === null) return;
  const name = newProfileName.value.trim();
  if (!name) return;
  const enabledIds = gameMods.value.filter(m => m.is_enabled).map(m => m.id);
  try {
    await modStore.createProfile(selectedGameId.value, name, enabledIds);
    addToast(t("mod.profileCreated"), "success");
    newProfileName.value = "";
  } catch (e) {
    addToast(t("mod.profileCreateFailed") + ": " + (e as string), "error");
  }
}

// ===== Rename =====

const renamingId = ref<number | null>(null);
const renameText = ref("");

function startRename(profile: ModProfile) {
  renamingId.value = profile.id;
  renameText.value = profile.name;
}

async function confirmRename(profile: ModProfile) {
  const name = renameText.value.trim();
  if (selectedGameId.value === null || !name) {
    renamingId.value = null;
    return;
  }
  try {
    await modStore.renameProfile(profile.id, selectedGameId.value, name);
  } catch (e) {
    addToast(String(e), "error");
  }
  renamingId.value = null;
}

// ===== Delete =====

const deletingProfile = ref<ModProfile | null>(null);

async function confirmDelete() {
  const p = deletingProfile.value;
  deletingProfile.value = null;
  if (!p || selectedGameId.value === null) return;
  try {
    await modStore.deleteProfile(p.id, selectedGameId.value);
  } catch (e) {
    addToast(String(e), "error");
  }
}

// ===== Edit members =====

const editingProfileId = ref<number | null>(null);
const editingSet = ref<Set<number>>(new Set());

function toggleEdit(profile: ModProfile) {
  if (editingProfileId.value === profile.id) {
    editingProfileId.value = null;
    return;
  }
  editingProfileId.value = profile.id;
  editingSet.value = new Set(profile.mod_ids);
}

function toggleMember(modId: number) {
  const s = new Set(editingSet.value);
  if (s.has(modId)) s.delete(modId);
  else s.add(modId);
  editingSet.value = s;
}

async function saveMembers(profile: ModProfile) {
  if (selectedGameId.value === null) return;
  // 保留原顺序，追加新增项
  const ordered = profile.mod_ids.filter(id => editingSet.value.has(id));
  for (const m of gameMods.value) {
    if (editingSet.value.has(m.id) && !profile.mod_ids.includes(m.id)) ordered.push(m.id);
  }
  try {
    await modStore.setProfileMods(profile.id, selectedGameId.value, ordered);
    addToast(t("mod.profileMembersSaved"), "success");
  } catch (e) {
    addToast(String(e), "error");
  }
  editingProfileId.value = null;
}
</script>

<template>
  <div
    class="fixed inset-0 z-[60] flex items-center justify-center bg-black/30 backdrop-blur-sm"
    @click.self="emit('close')"
  >
    <div class="bg-modal-bg rounded-2xl shadow-2xl w-[560px] max-h-[80vh] flex flex-col">
      <!-- Header -->
      <div class="shrink-0 flex items-center justify-between px-6 pt-6 pb-4">
        <div>
          <h3 class="text-lg font-bold text-text-main">{{ t('mod.profilesTitle') }}</h3>
          <p class="text-xs text-text-sub mt-1">{{ t('mod.profilesHint') }}</p>
        </div>
        <button class="w-8 h-8 rounded-lg flex items-center justify-center text-text-sub hover:bg-code-bg transition-colors" @click="emit('close')">
          <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6L6 18M6 6l12 12"/></svg>
        </button>
      </div>

      <!-- Game selector -->
      <div class="shrink-0 px-6 pb-4">
        <CustomSelect
          v-model="selectedGameId"
          :options="gameOptions"
          :placeholder="t('mod.selectGame')"
          searchable
        />
      </div>

      <!-- Profile list -->
      <div class="flex-1 overflow-auto px-6 pb-4 space-y-2">
        <div v-if="selectedGameId === null || modStore.modProfiles.length === 0" class="py-10 text-center text-sm text-text-sub">
          {{ t('mod.noProfiles') }}
        </div>
        <div
          v-for="profile in modStore.modProfiles"
          :key="profile.id"
          class="rounded-xl border p-3"
          :class="editingProfileId === profile.id ? 'border-primary-300 bg-primary-50/50 dark:bg-primary-900/10' : 'border-border-light bg-card'"
        >
          <div class="flex items-center gap-2">
            <!-- Name / rename input -->
            <input
              v-if="renamingId === profile.id"
              v-model="renameText"
              class="flex-1 min-w-0 px-2 py-1 text-sm rounded-lg border border-border-medium bg-input-bg text-text-main focus:outline-none focus:border-primary-400"
              @keyup.enter="confirmRename(profile)"
              @blur="confirmRename(profile)"
            />
            <div v-else class="flex-1 min-w-0">
              <span class="text-sm font-medium text-text-main truncate block">{{ profile.name }}</span>
              <span class="text-[11px] text-text-sub">{{ t('mod.profileModCount', { count: profile.mod_ids.length }) }}</span>
            </div>
            <!-- Actions -->
            <template v-if="renamingId !== profile.id">
              <button
                class="shrink-0 px-2.5 py-1.5 rounded-lg text-xs font-medium bg-primary-500 text-white hover:bg-primary-600 transition-colors disabled:opacity-50"
                :disabled="applyingId !== null"
                @click="applyProfile(profile)"
              >
                {{ applyingId === profile.id ? t('mod.applying') : t('mod.apply') }}
              </button>
              <button
                class="shrink-0 px-2.5 py-1.5 rounded-lg text-xs border border-border-medium text-text-sub hover:bg-code-bg transition-colors"
                :class="editingProfileId === profile.id ? 'text-primary-600 dark:text-primary-400 border-primary-300/60' : ''"
                @click="toggleEdit(profile)"
              >
                {{ t('mod.editMembers') }}
              </button>
              <button
                class="shrink-0 px-2.5 py-1.5 rounded-lg text-xs border border-border-medium text-text-sub hover:bg-code-bg transition-colors"
                @click="startRename(profile)"
              >
                {{ t('mod.rename') }}
              </button>
              <button
                class="shrink-0 px-2.5 py-1.5 rounded-lg text-xs border border-red-200 dark:border-red-900/50 text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
                @click="deletingProfile = profile"
              >
                {{ t('common.delete') }}
              </button>
            </template>
            <template v-else>
              <button class="shrink-0 px-2.5 py-1.5 rounded-lg text-xs bg-primary-500 text-white" @click="confirmRename(profile)">{{ t('common.save') }}</button>
              <button class="shrink-0 px-2.5 py-1.5 rounded-lg text-xs border border-border-medium text-text-sub" @click="renamingId = null">{{ t('app.cancel') }}</button>
            </template>
          </div>

          <!-- Member editor -->
          <div v-if="editingProfileId === profile.id" class="mt-3 border-t border-border-light pt-3">
            <div v-if="gameMods.length === 0" class="text-xs text-text-sub py-2">{{ t('mod.noMods') }}</div>
            <div v-else class="max-h-48 overflow-auto space-y-1">
              <label
                v-for="mod in gameMods"
                :key="mod.id"
                class="flex items-center gap-2 px-2 py-1.5 rounded-lg cursor-pointer hover:bg-primary-50 dark:hover:bg-primary-900/20 transition-colors"
              >
                <input
                  type="checkbox"
                  class="w-4 h-4 accent-primary-500"
                  :checked="editingSet.has(mod.id)"
                  @change="toggleMember(mod.id)"
                />
                <span class="flex-1 text-sm text-text-main truncate">{{ mod.name }}</span>
                <span
                  class="shrink-0 text-[10px] px-1.5 py-0.5 rounded-md font-medium"
                  :class="mod.is_enabled ? 'bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400' : 'bg-red-100 dark:bg-red-900/30 text-red-500 dark:text-red-400'"
                >
                  {{ mod.is_enabled ? t('mod.enabled') : t('mod.disabled') }}
                </span>
              </label>
            </div>
            <div class="flex justify-end gap-2 mt-2">
              <button class="px-3 py-1.5 rounded-lg text-xs border border-border-medium text-text-sub hover:bg-code-bg transition-colors" @click="editingProfileId = null">
                {{ t('app.cancel') }}
              </button>
              <button class="px-3 py-1.5 rounded-lg text-xs font-medium bg-primary-500 text-white hover:bg-primary-600 transition-colors" @click="saveMembers(profile)">
                {{ t('common.save') }}
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Create from current -->
      <div class="shrink-0 px-6 py-4 border-t border-border-light">
        <div class="flex items-center gap-2">
          <input
            v-model="newProfileName"
            class="flex-1 px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-main placeholder-text-sub/50 focus:outline-none focus:border-primary-400"
            :placeholder="t('mod.profileNamePlaceholder')"
            @keyup.enter="createFromCurrent"
          />
          <button
            class="shrink-0 px-4 py-2 rounded-xl text-sm font-medium bg-primary-500 text-white hover:bg-primary-600 transition-colors disabled:opacity-50"
            :disabled="!newProfileName.trim() || selectedGameId === null"
            @click="createFromCurrent"
          >
            {{ t('mod.saveCurrentAsProfile') }}
          </button>
        </div>
        <p class="text-[11px] text-text-sub mt-2">{{ t('mod.saveCurrentHint') }}</p>
      </div>
    </div>

    <!-- Delete confirm -->
    <ConfirmDialog
      v-if="deletingProfile"
      :title="t('mod.deleteProfile')"
      :message="t('mod.deleteProfileConfirm', { name: deletingProfile.name })"
      :confirm-text="t('common.delete')"
      :danger="true"
      @confirm="confirmDelete"
      @cancel="deletingProfile = null"
    />
  </div>
</template>
