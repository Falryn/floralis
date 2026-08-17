<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import CustomSelect from "./CustomSelect.vue";

const { t } = useI18n();

const props = withDefaults(defineProps<{
  title: string;
  subtitle: string;
  searchModelValue: string;
  searchPlaceholder?: string;
  sortModelValue: string;
  sortOptions: { label: string; value: string }[];
  sortClass?: string;
  showSelectMode?: boolean;
  isSelectMode?: boolean;
  viewMode?: 'grid' | 'list';
  showViewToggle?: boolean;
}>(), {
  searchPlaceholder: '',
  sortClass: 'w-32',
  showSelectMode: false,
  isSelectMode: false,
  viewMode: 'grid',
  showViewToggle: true,
});

const emit = defineEmits<{
  'update:searchModelValue': [value: string];
  'update:sortModelValue': [value: string];
  'update:viewMode': [value: 'grid' | 'list'];
  'enterSelectMode': [];
}>();

const searchInputRef = ref<HTMLInputElement | null>(null);

function focusSearch() {
  searchInputRef.value?.focus();
}

defineExpose({ focusSearch });
</script>

<template>
  <div class="flex items-end justify-between mb-10 gap-4">
    <div>
      <h2 class="text-2xl font-bold text-text-main">{{ title }}</h2>
      <p class="text-sm text-text-sub mt-1">
        {{ subtitle }}
        <slot name="subtitle-extra" />
      </p>
    </div>
    <div class="flex items-center gap-2 shrink-0">
      <!-- Batch actions (select mode) -->
      <template v-if="isSelectMode">
        <slot name="batch-actions" />
      </template>
      <!-- Normal mode controls -->
      <template v-else>
        <!-- Search -->
        <div class="relative">
          <input
            ref="searchInputRef"
            :value="searchModelValue"
            @input="emit('update:searchModelValue', ($event.target as HTMLInputElement).value)"
            type="text"
            :placeholder="searchPlaceholder || t('common.search')"
            class="w-48 pl-8 pr-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-main placeholder-text-sub/50 outline-none focus:border-primary-400 transition-colors"
          />
          <svg class="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-text-sub/50" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8"/>
            <line x1="21" y1="21" x2="16.65" y2="16.65"/>
          </svg>
        </div>
        <!-- Filters slot -->
        <slot name="filters" />
        <!-- Sort -->
        <CustomSelect
          :modelValue="sortModelValue"
          @update:modelValue="emit('update:sortModelValue', $event as string)"
          :options="sortOptions"
          :class="sortClass"
        />
        <!-- Multi-select button -->
        <button
          v-if="showSelectMode"
          class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-sub hover:bg-primary-50 hover:text-text-main transition-colors"
          @click="emit('enterSelectMode')"
        >
          {{ t('app.multiSelect') }}
        </button>
        <!-- View toggle -->
        <button
          v-if="showViewToggle"
          class="px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-text-sub hover:bg-primary-50 hover:text-text-main transition-colors"
          @click="emit('update:viewMode', viewMode === 'grid' ? 'list' : 'grid')"
          :title="viewMode === 'grid' ? t('app.switchToList') : t('app.switchToGrid')"
        >
          {{ viewMode === 'grid' ? '☰' : '⊞' }}
        </button>
      </template>
    </div>
  </div>
</template>
