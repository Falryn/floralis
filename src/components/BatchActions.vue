<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useGameStore } from "../stores/gameStore";
import { addToast } from "../composables/useToast";

const { t } = useI18n();
const store = useGameStore();

const emit = defineEmits<{
  // 请求批量删除：由 App.vue 弹出确认对话框
  requestDelete: [];
}>();

// 同一时间只允许一个批量下拉展开：打开任一下拉时自动收起其他
type MenuName = "move" | "status" | "rating" | "favorite" | "more";
const openMenu = ref<MenuName | null>(null);

function toggleMenu(name: MenuName) {
  openMenu.value = openMenu.value === name ? null : name;
}

// 批量扫描封面进行中
const batchScanning = ref(false);

const statusOptions = [
  { labelKey: 'game.notPlayed', value: 'not_played' },
  { labelKey: 'game.playing', value: 'playing' },
  { labelKey: 'game.completed', value: 'completed' },
  { labelKey: 'game.shelved', value: 'shelved' },
];

async function doBatchMove(groupId: number | null) {
  openMenu.value = null;
  await store.batchMoveGames(groupId);
}

async function doBatchScanCovers() {
  batchScanning.value = true;
  try {
    const ids = store.filteredGames.map((g) => g.id);
    const count = await store.batchScanCovers(ids);
    await store.loadGames();
    addToast(t('batch.scanComplete', { count }), "success");
  } catch (e) {
    console.error("批量扫描封面失败:", e);
  } finally {
    batchScanning.value = false;
    openMenu.value = null;
  }
}

async function doBatchSetStatus(status: string) {
  openMenu.value = null;
  await store.batchSetStatus(status);
}

async function doBatchSetRating(rating: number) {
  openMenu.value = null;
  await store.batchSetRating(rating);
}

async function doBatchSetFavorite(favorite: boolean) {
  openMenu.value = null;
  await store.batchSetFavorite(favorite);
}

function exitSelectMode() {
  store.clearSelection();
}

// 点击所有批量菜单以外区域时收起下拉
function handleClickOutside(e: MouseEvent) {
  const el = e.target as HTMLElement;
  if (
    el.closest(".batch-move-menu") ||
    el.closest(".batch-status-menu") ||
    el.closest(".batch-rating-menu") ||
    el.closest(".batch-favorite-menu") ||
    el.closest(".batch-more-menu")
  ) {
    return;
  }
  openMenu.value = null;
}
onMounted(() => document.addEventListener("click", handleClickOutside));
onUnmounted(() => document.removeEventListener("click", handleClickOutside));

const menuPanelClass =
  "absolute top-full mt-1 right-0 z-[80] bg-modal-bg border border-border-light rounded-xl shadow-2xl py-2 min-w-[140px]";
const menuItemClass =
  "w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 transition-colors";
const triggerClass =
  "px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-main hover:bg-primary-50 transition-colors";
</script>

<template>
  <div class="relative batch-move-menu">
    <button :class="triggerClass" @click="toggleMenu('move')">
      {{ t('batch.moveToGroup') }} ▾
    </button>
    <div v-if="openMenu === 'move'" :class="menuPanelClass">
      <button :class="menuItemClass" @click="doBatchMove(null)">
        {{ t('batch.ungrouped') }}
      </button>
      <button
        v-for="g in store.groups"
        :key="g.id"
        :class="menuItemClass"
        @click="doBatchMove(g.id)"
      >
        {{ g.name }}
      </button>
    </div>
  </div>
  <div class="relative batch-status-menu">
    <button :class="triggerClass" @click="toggleMenu('status')">
      {{ t('batch.setStatus') }} ▾
    </button>
    <div v-if="openMenu === 'status'" :class="menuPanelClass">
      <button
        v-for="opt in statusOptions"
        :key="opt.value"
        :class="menuItemClass"
        @click="doBatchSetStatus(opt.value)"
      >
        {{ t(opt.labelKey) }}
      </button>
    </div>
  </div>
  <div class="relative batch-rating-menu">
    <button :class="triggerClass" @click="toggleMenu('rating')">
      {{ t('batch.setRating') }} ▾
    </button>
    <div v-if="openMenu === 'rating'" :class="menuPanelClass">
      <button
        v-for="r in [1, 2, 3, 4, 5]"
        :key="r"
        :class="menuItemClass"
        @click="doBatchSetRating(r)"
      >
        {{ r }} {{ t('batch.star') }}
      </button>
      <button
        class="w-full px-4 py-2 text-sm text-left text-text-sub hover:bg-primary-50 transition-colors"
        @click="doBatchSetRating(0)"
      >
        {{ t('batch.clearRating') }}
      </button>
    </div>
  </div>
  <div class="relative batch-favorite-menu">
    <button :class="triggerClass" @click="toggleMenu('favorite')">
      {{ t('batch.favorite') }} ▾
    </button>
    <div v-if="openMenu === 'favorite'" :class="menuPanelClass">
      <button :class="menuItemClass" @click="doBatchSetFavorite(true)">
        {{ t('game.favorite') }}
      </button>
      <button :class="menuItemClass" @click="doBatchSetFavorite(false)">
        {{ t('game.unfavorite') }}
      </button>
    </div>
  </div>
  <div class="relative batch-more-menu">
    <button :class="triggerClass" @click="toggleMenu('more')">
      {{ t('batch.more') }} ▾
    </button>
    <div v-if="openMenu === 'more'" :class="menuPanelClass">
      <button :class="menuItemClass" @click="openMenu = null; store.selectAll()">
        {{ t('app.selectAll') }}
      </button>
      <button
        :class="menuItemClass"
        :disabled="batchScanning"
        @click="doBatchScanCovers"
      >
        {{ batchScanning ? t('batch.scanning') : t('batch.scanCovers') }}
      </button>
    </div>
  </div>
  <button
    class="px-3 py-2 text-sm rounded-xl bg-red-500 text-white hover:bg-red-600 transition-colors"
    @click="emit('requestDelete')"
  >
    {{ t('batch.delete') }}
  </button>
  <button
    class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-sub hover:bg-code-bg transition-colors"
    @click="exitSelectMode"
  >
    {{ t('app.cancel') }}
  </button>
</template>
