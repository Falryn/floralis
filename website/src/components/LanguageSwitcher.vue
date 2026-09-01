<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { setLocale, type Locale } from '../i18n'

const { locale } = useI18n()

const locales: { value: Locale; label: string }[] = [
  { value: 'zh-CN', label: '简体中文' },
  { value: 'en-US', label: 'English' },
  { value: 'ja-JP', label: '日本語' }
]

const open = ref(false)
const root = ref<HTMLElement | null>(null)

const current = computed(
  () => locales.find(l => l.value === locale.value) ?? locales[1]
)

function toggle() {
  open.value = !open.value
}

function choose(value: Locale) {
  setLocale(value)
  open.value = false
}

function onDocPointerDown(e: PointerEvent) {
  if (root.value && !root.value.contains(e.target as Node)) open.value = false
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') open.value = false
}

onMounted(() => {
  document.addEventListener('pointerdown', onDocPointerDown)
  document.addEventListener('keydown', onKeydown)
})

onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', onDocPointerDown)
  document.removeEventListener('keydown', onKeydown)
})
</script>

<template>
  <div ref="root" class="relative">
    <button
      type="button"
      class="flex items-center gap-1.5 rounded-full border border-brand-100 bg-white/70 px-3.5 py-1.5 text-sm text-[#4a4a68] transition hover:border-brand-200 hover:text-brand-600"
      :aria-expanded="open"
      aria-haspopup="listbox"
      @click="toggle"
    >
      <svg
        viewBox="0 0 24 24"
        class="h-4 w-4 text-brand-400"
        fill="none"
        stroke="currentColor"
        stroke-width="1.5"
        stroke-linecap="round"
        aria-hidden="true"
      >
        <circle cx="12" cy="12" r="8.5" />
        <path d="M3.5 12h17" />
        <path d="M12 3.5c2.5 2.3 3.8 5.2 3.8 8.5s-1.3 6.2-3.8 8.5c-2.5-2.3-3.8-5.2-3.8-8.5s1.3-6.2 3.8-8.5Z" />
      </svg>
      {{ current.label }}
      <svg
        viewBox="0 0 24 24"
        class="h-3.5 w-3.5 text-[#8b8ba3] transition-transform duration-200"
        :class="{ 'rotate-180': open }"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <path d="m6 9 6 6 6-6" />
      </svg>
    </button>

    <Transition name="dropdown" :duration="160">
      <ul
        v-if="open"
        role="listbox"
        class="absolute right-0 top-full z-50 mt-2 w-40 overflow-hidden rounded-xl border border-brand-100 bg-white/92 py-1.5 shadow-xl shadow-brand-100/70 backdrop-blur-xl"
      >
        <li
          v-for="l in locales"
          :key="l.value"
          role="option"
          :aria-selected="l.value === locale"
        >
          <button
            type="button"
            class="flex w-full items-center justify-between px-4 py-2 text-sm transition"
            :class="
              l.value === locale
                ? 'bg-brand-50 font-semibold text-brand-600'
                : 'text-[#4a4a68] hover:bg-brand-50/70 hover:text-brand-600'
            "
            @click="choose(l.value)"
          >
            {{ l.label }}
            <svg
              v-if="l.value === locale"
              viewBox="0 0 24 24"
              class="h-4 w-4 text-brand-500"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              aria-hidden="true"
            >
              <path d="m5 12.5 4.5 4.5L19 7.5" />
            </svg>
          </button>
        </li>
      </ul>
    </Transition>
  </div>
</template>
