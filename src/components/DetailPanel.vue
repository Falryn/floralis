<script setup lang="ts">
import { ref, watch } from "vue";

const props = withDefaults(defineProps<{
  coverUrl?: string;
  coverAspect?: 'square' | 'video';
  fallbackIcon?: string;
  showClose?: boolean;
}>(), {
  coverUrl: '',
  coverAspect: 'square',
  fallbackIcon: '🎮',
  showClose: true,
});

const emit = defineEmits<{
  close: [];
}>();

// 检测封面方向：横图用 object-contain 完整展示，竖图用 object-cover 填充
const isLandscape = ref(false);

watch(() => props.coverUrl, (url) => {
  isLandscape.value = false;
  if (!url) return;
  const img = new Image();
  img.onload = () => {
    isLandscape.value = img.naturalWidth > img.naturalHeight;
  };
  img.src = url;
}, { immediate: true });
</script>

<template>
  <div class="detail-panel w-[400px] h-full bg-detail-bg border-l border-border-light flex flex-col shadow-xl">
    <!-- Cover Image -->
    <div class="relative shrink-0">
      <div
        class="bg-gradient-to-br from-primary-50 to-sakura-50 overflow-hidden"
        :class="coverAspect === 'square' ? 'aspect-square' : 'aspect-video'"
      >
        <!-- Main cover image -->
        <img
          v-if="coverUrl"
          :src="coverUrl"
          class="relative w-full h-full"
          :class="isLandscape ? 'object-contain' : 'object-cover'"
        />
        <div
          v-else
          class="w-full h-full flex items-center justify-center text-7xl text-primary-200"
        >
          {{ fallbackIcon }}
        </div>
      </div>
      <!-- Gradient fade overlay -->
      <div 
        class="absolute inset-x-0 bottom-0 h-32 bg-gradient-to-t from-detail-bg via-detail-bg/80 to-transparent"
      />
    </div>

    <!-- Content -->
    <div class="flex-1 px-8 pb-8 -mt-4 relative space-y-8 overflow-auto">
      <slot />
      <button
        v-if="showClose"
        class="absolute top-4 right-4 w-10 h-10 flex items-center justify-center rounded-xl bg-overlay-white backdrop-blur-sm hover:bg-white/90 shadow-md transition-all text-sm z-10"
        @click="emit('close')"
      >
        ✕
      </button>
    </div>
  </div>
</template>
