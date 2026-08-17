<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const props = defineProps<{
  modelValue: string[];
  options: { label: string; value: string }[];
  placeholder?: string;
  allLabel?: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string[]];
}>();

const isOpen = ref(false);
const selectRef = ref<HTMLElement | null>(null);

const displayLabel = computed(() => {
  if (props.modelValue.length === 0) return props.allLabel ?? props.placeholder ?? "全部";
  const labels = props.modelValue.map(
    (v) => props.options.find((o) => o.value === v)?.label ?? v
  );
  if (labels.length <= 2) return labels.join(", ");
  return `${labels[0]} 等 ${labels.length} 项`;
});

const hasSelection = computed(() => props.modelValue.length > 0);

function toggle() {
  isOpen.value = !isOpen.value;
}

function isSelected(value: string): boolean {
  return props.modelValue.includes(value);
}

function toggleOption(value: string) {
  const current = [...props.modelValue];
  const idx = current.indexOf(value);
  if (idx >= 0) {
    current.splice(idx, 1);
  } else {
    current.push(value);
  }
  emit("update:modelValue", current);
}

function clearAll() {
  emit("update:modelValue", []);
}

function selectAllOptions() {
  emit("update:modelValue", props.options.map((o) => o.value));
}

function onClickOutside(e: MouseEvent) {
  if (selectRef.value && !selectRef.value.contains(e.target as Node)) {
    isOpen.value = false;
  }
}

onMounted(() => {
  document.addEventListener("click", onClickOutside);
});

onBeforeUnmount(() => {
  document.removeEventListener("click", onClickOutside);
});
</script>

<template>
  <div ref="selectRef" class="relative">
    <button
      type="button"
      class="w-full px-3 py-2 text-sm rounded-xl border border-border-medium bg-input-bg text-left flex items-center justify-between outline-none focus:border-primary-400 transition-colors cursor-pointer"
      :class="{ 'text-primary-600 dark:text-primary-400 border-primary-300/50': hasSelection }"
      @click="toggle"
    >
      <span class="truncate" :class="hasSelection ? '' : 'text-text-sub'">{{ displayLabel }}</span>
      <svg
        class="w-4 h-4 text-text-sub shrink-0 ml-2 transition-transform"
        :class="{ 'rotate-180': isOpen }"
        viewBox="0 0 20 20"
        fill="currentColor"
      >
        <path
          fill-rule="evenodd"
          d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z"
          clip-rule="evenodd"
        />
      </svg>
    </button>
    <transition name="dropdown">
      <div
        v-if="isOpen"
        class="absolute z-50 mt-1 w-full min-w-[180px] bg-input-bg rounded-xl border border-border-medium shadow-lg overflow-hidden"
      >
        <div class="max-h-60 overflow-auto py-1">
          <div
            v-for="opt in options"
            :key="opt.value"
            class="px-3 py-2 text-sm cursor-pointer transition-colors flex items-center gap-2"
            :class="isSelected(opt.value) ? 'bg-primary-50/70 text-primary-600 font-medium' : 'text-text-main hover:bg-primary-50/50'"
            @click="toggleOption(opt.value)"
          >
            <span
              class="w-4 h-4 rounded border flex items-center justify-center shrink-0 transition-colors"
              :class="isSelected(opt.value) ? 'bg-primary-500 border-primary-500' : 'border-border-medium bg-input-bg'"
            >
              <svg v-if="isSelected(opt.value)" class="w-3 h-3 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                <polyline points="20 6 9 17 4 12"/>
              </svg>
            </span>
            {{ opt.label }}
          </div>
        </div>
        <div class="border-t border-border-light px-3 py-1.5 flex items-center justify-between">
          <button
            class="text-xs text-text-sub hover:text-primary-500 transition-colors"
            @click="selectAllOptions"
          >
            ✓ {{ t('common.selectAll') }}
          </button>
          <button
            class="text-xs text-text-sub hover:text-red-400 transition-colors"
            @click="clearAll"
          >
            ✕ {{ t('common.clearAll') }}
          </button>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.dropdown-enter-active,
.dropdown-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
