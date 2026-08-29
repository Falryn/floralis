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

// 各批量操作下拉菜单的展开状态
const showBatchMoveMenu = ref(false);
const showBatchStatusMenu = ref(false);
const showBatchRatingMenu = ref(false);

// 批量扫描封面进行中
const batchScanning = ref(false);

const statusOptions = [
  { labelKey: 'game.notPlayed', value: 'not_played' },
  { labelKey: 'game.playing', value: 'playing' },
  { labelKey: 'game.completed', value: 'completed' },
  { labelKey: 'game.shelved', value: 'shelved' },
];

async function doBatchMove(groupId: number | null) {
  await store.batchMoveGames(groupId);
  showBatchMoveMenu.value = false;
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
  }
}

async function doBatchSetStatus(status: string) {
  await store.batchSetStatus(status);
  showBatchStatusMenu.value = false;
}

async function doBatchSetRating(rating: number) {
  await store.batchSetRating(rating);
  showBatchRatingMenu.value = false;
}

function exitSelectMode() {
  store.clearSelection();
}

// 点击批量菜单以外区域时收起所有下拉
function handleClickOutside(e: MouseEvent) {
  const el = e.target as HTMLElement;
  if (
    el.closest(".batch-move-menu") ||
    el.closest(".batch-status-menu") ||
    el.closest(".batch-rating-menu")
  ) {
    return;
  }
  showBatchMoveMenu.value = false;
  showBatchStatusMenu.value = false;
  showBatchRatingMenu.value = false;
}
onMounted(() => document.addEventListener("click", handleClickOutside));
onUnmounted(() => document.removeEventListener("click", handleClickOutside));
</script>

<template>
  <button
    class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-main hover:bg-primary-50 transition-colors"
    @click="store.selectAll()"
  >
    {{ t('app.selectAll') }}
  </button>
  <div class="relative batch-move-menu">
    <button
      class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-main hover:bg-primary-50 transition-colors"
      @click="showBatchMoveMenu = !showBatchMoveMenu"
    >
      {{ t('batch.moveToGroup') }} ▾
    </button>
    <div
      v-if="showBatchMoveMenu"
      class="absolute top-full mt-1 right-0 z-[80] bg-modal-bg border border-border-light rounded-xl shadow-2xl py-2 min-w-[140px]"
    >
      <button
        class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 transition-colors"
        @click="doBatchMove(null)"
      >
        {{ t('batch.ungrouped') }}
      </button>
      <button
        v-for="g in store.groups"
        :key="g.id"
        class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 transition-colors"
        @click="doBatchMove(g.id)"
      >
        {{ g.name }}
      </button>
    </div>
  </div>
  <button
    class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-main hover:bg-primary-50 transition-colors"
    :disabled="batchScanning"
    @click="doBatchScanCovers"
  >
    {{ batchScanning ? t('batch.scanning') : t('batch.scanCovers') }}
  </button>
  <div class="relative batch-status-menu">
    <button
      class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-main hover:bg-primary-50 transition-colors"
      @click="showBatchStatusMenu = !showBatchStatusMenu"
    >
      {{ t('batch.setStatus') }} ▾
    </button>
    <div
      v-if="showBatchStatusMenu"
      class="absolute top-full mt-1 right-0 z-[80] bg-modal-bg border border-border-light rounded-xl shadow-2xl py-2 min-w-[140px]"
    >
      <button
        v-for="opt in statusOptions"
        :key="opt.value"
        class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 transition-colors"
        @click="doBatchSetStatus(opt.value)"
      >
        {{ t(opt.labelKey) }}
      </button>
    </div>
  </div>
  <div class="relative batch-rating-menu">
    <button
      class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-main hover:bg-primary-50 transition-colors"
      @click="showBatchRatingMenu = !showBatchRatingMenu"
    >
      {{ t('batch.setRating') }} ▾
    </button>
    <div
      v-if="showBatchRatingMenu"
      class="absolute top-full mt-1 right-0 z-[80] bg-modal-bg border border-border-light rounded-xl shadow-2xl py-2 min-w-[100px]"
    >
      <button
        v-for="r in [1, 2, 3, 4, 5]"
        :key="r"
        class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 transition-colors"
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
