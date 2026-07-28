<script setup lang="ts">
import { useToast } from "../composables/useToast";

const { toasts, removeToast } = useToast();

const iconMap: Record<string, string> = {
  success: "\u2713",
  error: "\u2717",
  info: "\u24D8",
};
</script>

<template>
  <Teleport to="body">
    <div class="fixed top-6 right-6 z-[200] flex flex-col gap-2 pointer-events-none">
      <TransitionGroup name="toast">
        <div
          v-for="toast in toasts"
          :key="toast.id"
          class="pointer-events-auto flex items-center gap-2.5 px-4 py-3 rounded-xl shadow-lg backdrop-blur-md border min-w-[240px] max-w-[360px]"
          :class="{
            'bg-green-50/95 border-green-200 text-green-700': toast.type === 'success',
            'bg-red-50/95 border-red-200 text-red-700': toast.type === 'error',
            'bg-primary-50/95 border-primary-200 text-primary-700': toast.type === 'info',
          }"
        >
          <span class="text-base shrink-0">{{ iconMap[toast.type] }}</span>
          <span class="text-sm flex-1">{{ toast.message }}</span>
          <button
            class="shrink-0 text-current opacity-50 hover:opacity-100 transition-opacity text-xs"
            @click="removeToast(toast.id)"
          >
            ✕
          </button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-enter-active {
  transition: all 0.3s ease;
}
.toast-leave-active {
  transition: all 0.2s ease;
}
.toast-enter-from {
  opacity: 0;
  transform: translateX(30px);
}
.toast-leave-to {
  opacity: 0;
  transform: translateX(30px);
}
</style>
