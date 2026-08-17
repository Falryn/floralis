<script setup lang="ts">
import { ref, computed, nextTick, onMounted, onBeforeUnmount } from "vue";
import { useI18n } from "vue-i18n";

const props = defineProps<{
  modelValue: string | number | null;
  options: { label: string; value: string | number | null }[];
  placeholder?: string;
  searchable?: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string | number | null];
}>();

const { t } = useI18n();

const isOpen = ref(false);
const searchText = ref("");
const selectRef = ref<HTMLElement | null>(null);
const searchInputRef = ref<HTMLInputElement | null>(null);

const selectedLabel = computed(() => {
  const opt = props.options.find((o) => o.value === props.modelValue);
  return opt?.label ?? props.placeholder ?? "请选择";
});

const filteredOptions = computed(() => {
  if (!props.searchable || !searchText.value.trim()) return props.options;
  const kw = searchText.value.trim().toLowerCase();
  return props.options.filter((o) => o.label.toLowerCase().includes(kw));
});

async function toggle() {
  isOpen.value = !isOpen.value;
  if (isOpen.value && props.searchable) {
    searchText.value = "";
    await nextTick();
    searchInputRef.value?.focus();
  }
}

function select(value: string | number | null) {
  emit("update:modelValue", value);
  isOpen.value = false;
  searchText.value = "";
}

function onClickOutside(e: MouseEvent) {
  if (selectRef.value && !selectRef.value.contains(e.target as Node)) {
    isOpen.value = false;
    searchText.value = "";
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
        class="absolute z-50 mt-1 w-full bg-input-bg rounded-xl border border-border-medium shadow-lg overflow-hidden"
      >
        <!-- Search input -->
        <div v-if="searchable" class="px-2 pt-2 pb-1 border-b border-border-light">
          <input
            ref="searchInputRef"
            v-model="searchText"
            class="w-full px-2.5 py-1.5 text-sm rounded-lg border border-border-medium bg-input-bg text-text-main outline-none focus:border-primary-400 transition-colors"
            :placeholder="t('common.search')"
            @click.stop
            @keydown.escape="isOpen = false"
          />
        </div>
        <div class="max-h-60 overflow-auto">
          <div
            v-for="opt in filteredOptions"
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
          <div
            v-if="filteredOptions.length === 0"
            class="px-3 py-2.5 text-sm text-text-sub italic text-center"
          >
            {{ t('common.noResults') }}
          </div>
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
