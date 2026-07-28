<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from "vue";

const props = defineProps<{
  modelValue: string | number | null;
  options: { label: string; value: string | number | null }[];
  placeholder?: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string | number | null];
}>();

const isOpen = ref(false);
const selectRef = ref<HTMLElement | null>(null);

const selectedLabel = computed(() => {
  const opt = props.options.find((o) => o.value === props.modelValue);
  return opt?.label ?? props.placeholder ?? "请选择";
});

function toggle() {
  isOpen.value = !isOpen.value;
}

function select(value: string | number | null) {
  emit("update:modelValue", value);
  isOpen.value = false;
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
      @click="toggle"
    >
      <span class="truncate text-text-main">{{ selectedLabel }}</span>
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
        class="absolute z-50 mt-1 w-full bg-input-bg rounded-xl border border-border-medium shadow-lg overflow-hidden max-h-60 overflow-auto"
      >
        <div
          v-for="opt in options"
          :key="String(opt.value)"
          class="px-3 py-2.5 text-sm cursor-pointer transition-colors"
          :class="
            opt.value === modelValue
              ? 'bg-primary-50 text-primary-600 font-medium'
              : 'text-text-main hover:bg-primary-50/50'
          "
          @click="select(opt.value)"
        >
          {{ opt.label }}
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
