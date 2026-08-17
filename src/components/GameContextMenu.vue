<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { useGameStore } from "../stores/gameStore";
import { useI18n } from "vue-i18n";
import { openInExplorer } from "../utils/format";
import type { LaunchAction } from "../types";

const props = defineProps<{
  gameId: number;
  x: number;
  y: number;
}>();

const emit = defineEmits<{
  close: [];
  launch: [id: number, actionId?: number];
  edit: [id: number];
  delete: [id: number];
  moveToGroup: [gameId: number, groupId: number | null];
}>();

const { t } = useI18n();
const store = useGameStore();

const menuRef = ref<HTMLElement | null>(null);

const currentGame = computed(() => store.games.find((g) => g.id === props.gameId));

// 附加启动入口：菜单打开时加载，有则平铺展示在“启动”下方
const launchActions = ref<LaunchAction[]>([]);

const groupOptions = computed(() => [
  { label: t('game.ungrouped'), value: null as number | null },
  ...store.groups.map((g) => ({ label: g.name, value: g.id as number | null })),
]);

function handleClickOutside(e: MouseEvent) {
  if (menuRef.value && !menuRef.value.contains(e.target as Node)) {
    emit("close");
  }
}

onMounted(async () => {
  document.addEventListener("mousedown", handleClickOutside);
  try {
    launchActions.value = await store.loadLaunchActions(props.gameId);
  } catch {
    // 加载失败时退化为仅默认启动，不打断菜单
  }
});

onUnmounted(() => {
  document.removeEventListener("mousedown", handleClickOutside);
});
</script>

<template>
  <div
    ref="menuRef"
    class="fixed z-[70] bg-modal-bg border border-border-light rounded-xl shadow-2xl py-2 min-w-[160px]"
    :style="{ left: x + 'px', top: y + 'px' }"
  >
    <button
      class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 transition-colors"
      @click="emit('launch', gameId)"
    >
      ▶ {{ t('game.launch') }}
    </button>
    <template v-if="launchActions.length > 0">
      <button
        v-for="action in launchActions"
        :key="action.id"
        class="w-full pl-8 pr-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 transition-colors"
        :title="action.program_path"
        @click="emit('launch', gameId, action.id)"
      >
        ▶ {{ action.name }}
      </button>
    </template>
    <button
      class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 transition-colors"
      @click="emit('edit', gameId)"
    >
      ✏️ {{ t('game.edit') }}
    </button>
    <button
      v-if="currentGame?.install_path"
      class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 transition-colors"
      @click="openInExplorer(currentGame!.install_path); emit('close')"
    >
      📂 {{ t('game.openInstallDir') }}
    </button>
    <div class="border-t border-border-light my-1" />
    <div class="px-4 py-1.5 text-xs text-text-sub font-medium">{{ t('batch.moveToGroup') }}</div>
    <button
      v-for="opt in groupOptions"
      :key="opt.label"
      class="w-full px-4 py-1.5 text-sm text-left text-text-main hover:bg-primary-50 transition-colors"
      :class="{ 'text-primary-600 font-medium': currentGame?.group_id === opt.value }"
      @click="emit('moveToGroup', gameId, opt.value)"
    >
      {{ opt.label }}
    </button>
    <div class="border-t border-border-light my-1" />
    <button
      class="w-full px-4 py-2 text-sm text-left text-red-500 hover:bg-red-50 transition-colors"
      @click="emit('delete', gameId)"
    >
      🗑️ {{ t('common.delete') }}
    </button>
  </div>
</template>
