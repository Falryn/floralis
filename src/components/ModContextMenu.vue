<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { useModStore } from "../stores/modStore";
import { useI18n } from "vue-i18n";
import { openInExplorer } from "../utils/format";

const props = defineProps<{
  modId: number;
  x: number;
  y: number;
}>();

const emit = defineEmits<{
  close: [];
  edit: [id: number];
  delete: [id: number];
  linkGame: [id: number];
}>();

const { t } = useI18n();
const modStore = useModStore();
const menuRef = ref<HTMLElement | null>(null);

const currentMod = computed(() => modStore.mods.find((m) => m.id === props.modId));

function handleClickOutside(e: MouseEvent) {
  if (menuRef.value && !menuRef.value.contains(e.target as Node)) {
    emit("close");
  }
}

function openModDir() {
  if (currentMod.value?.mod_path) {
    openInExplorer(currentMod.value.mod_path);
  }
  emit("close");
}

function toggleEnabled() {
  if (currentMod.value) {
    modStore.toggleModEnabled(currentMod.value.id);
  }
  emit("close");
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
      class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 dark:hover:bg-primary-900/30 transition-colors"
      @click="emit('edit', modId)"
    >
      ✏️ {{ t('mod.contextEdit') }}
    </button>
    <button
      class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 dark:hover:bg-primary-900/30 transition-colors"
      @click="toggleEnabled"
    >
      {{ currentMod?.is_enabled ? '🔴' : '🟢' }} {{ t('mod.contextToggle') }}
    </button>
    <button
      v-if="currentMod?.mod_path"
      class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 dark:hover:bg-primary-900/30 transition-colors"
      @click="openModDir"
    >
      📂 {{ t('mod.contextOpenDir') }}
    </button>
    <div class="border-t border-border-light my-1" />
    <button
      class="w-full px-4 py-2 text-sm text-left text-text-main hover:bg-primary-50 dark:hover:bg-primary-900/30 transition-colors"
      @click="emit('linkGame', modId)"
    >
      🔗 {{ t('mod.contextLinkGame') }}
    </button>
    <div class="border-t border-border-light my-1" />
    <button
      class="w-full px-4 py-2 text-sm text-left text-red-500 hover:bg-red-50 dark:hover:bg-red-900/30 transition-colors"
      @click="emit('delete', modId)"
    >
      🗑️ {{ t('mod.contextDelete') }}
    </button>
  </div>
</template>
