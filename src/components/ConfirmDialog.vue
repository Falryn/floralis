<script setup lang="ts">
defineProps<{
  title: string;
  message: string;
  confirmText?: string;
  danger?: boolean;
}>();

const emit = defineEmits<{
  confirm: [];
  cancel: [];
}>();
</script>

<template>
  <div
    class="fixed inset-0 z-[60] flex items-center justify-center bg-black/30 backdrop-blur-sm"
    @click.self="emit('cancel')"
  >
    <div class="bg-modal-bg rounded-2xl shadow-2xl w-[400px] p-8 space-y-6">
      <h3 class="text-lg font-bold text-text-main">{{ title }}</h3>
      <p class="text-sm text-text-sub leading-relaxed">{{ message }}</p>
      <div class="flex justify-end gap-3 pt-2">
        <button
          class="px-4 py-2 rounded-xl border border-border-medium text-sm text-text-sub hover:bg-code-bg transition-colors"
          @click="emit('cancel')"
        >
          取消
        </button>
        <button
          :class="[
            'px-4 py-2 rounded-xl text-sm font-medium transition-all shadow-sm',
            danger
              ? 'bg-red-500 text-white hover:bg-red-600'
              : 'bg-primary-500 text-white hover:bg-primary-600'
          ]"
          @click="emit('confirm')"
        >
          {{ confirmText ?? '确认' }}
        </button>
      </div>
    </div>
  </div>
</template>
