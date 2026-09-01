<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import LanguageSwitcher from './LanguageSwitcher.vue'

const { t } = useI18n()
</script>

<template>
  <header
    class="sticky top-0 z-50 border-b border-white/50 bg-white/60 backdrop-blur-xl"
  >
    <div class="mx-auto flex h-16 max-w-6xl items-center gap-6 px-5">
      <a href="#top" class="flex items-center gap-2.5">
        <img
          src="/app-icon.png"
          alt="Floralis"
          class="h-9 w-9 rounded-[10px] shadow-sm"
        />
        <span class="text-lg font-bold tracking-wide text-[#1a1a2e]">
          花譜
          <span class="bg-gradient-to-r from-brand-500 to-blossom-500 bg-clip-text text-transparent">
            Floralis
          </span>
        </span>
      </a>

      <nav class="ml-4 hidden items-center gap-1 md:flex">
        <a
          v-for="item in (['features', 'gallery', 'download', 'support'] as const)"
          :key="item"
          :href="`#${item}`"
          class="rounded-full px-3.5 py-1.5 text-sm font-medium text-[#4a4a68] transition hover:bg-brand-50 hover:text-brand-600"
        >
          {{ t(`nav.${item}`) }}
        </a>
      </nav>

      <div class="ml-auto flex items-center gap-3">
        <LanguageSwitcher />

        <a
          href="#download"
          class="rounded-full bg-gradient-to-r from-brand-500 to-blossom-500 px-4 py-1.5 text-sm font-semibold text-white shadow-md shadow-brand-200 transition hover:brightness-110"
        >
          {{ t('nav.download') }}
        </a>
      </div>
    </div>
  </header>
</template>
