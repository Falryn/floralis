<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { open } from "@tauri-apps/plugin-shell";
import { useGameStore } from "../stores/gameStore";

const props = defineProps<{
  gameId: number;
  x: number;
  y: number;
}>();

const emit = defineEmits<{
  close: [];
  launch: [id: number];
  edit: [id: number];
  delete: [id: number];
  moveToGroup: [gameId: number, groupId: number | null];
}>();

const store = useGameStore();

const menuRef = ref<HTMLElement | null>(null);

const currentGame = computed(() => store.games.find((g) => g.id === props.gameId));

const groupOptions = computed(() => [
  { label: "未分组", value: null as number | null },
  ...store.groups.map((g) => ({ label: g.name, value: g.id as number | null })),
]);

function handleClickOutside(e: MouseEvent) {
  if (menuRef.value && !menuRef.value.contains(e.target as Node)) {
    emit("close");
  }
}

onMounted(() => {
  document.addEventListener("mousedown", handleClickOutside);
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
      ▶ 启动游戏
    </button>
    <button
      class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 transition-colors"
      @click="emit('edit', gameId)"
    >
      ✏️ 编辑
    </button>
    <button
      v-if="currentGame?.install_path"
      class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 transition-colors"
      @click="open(currentGame!.install_path).catch(() => {})"
    >
      📂 打开安装目录
    </button>
    <div class="border-t border-border-light my-1" />
    <div class="px-4 py-1.5 text-xs text-text-sub font-medium">移动到分组</div>
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
      🗑️ 删除
    </button>
  </div>
</template>
