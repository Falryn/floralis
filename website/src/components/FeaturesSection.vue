<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const icons = [
  // 游戏库管理
  '<path d="M5 4.5A1.5 1.5 0 0 1 6.5 3H18a1 1 0 0 1 1 1v16a1 1 0 0 1-1 1H6.5A1.5 1.5 0 0 1 5 19.5Z"/><path d="M8.5 3v18"/><path d="M12 8h3.5M12 11.5h3.5"/>',
  // 智能导入
  '<path d="M12 3v8.5"/><path d="m8.5 8 3.5 3.5L15.5 8"/><path d="M4 14.5V18a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-3.5"/><path d="m18.5 3.5.6 1.4 1.4.6-1.4.6-.6 1.4-.6-1.4-1.4-.6 1.4-.6Z"/>',
  // 一键启动
  '<circle cx="12" cy="12" r="8.5"/><path d="m10.2 8.6 5 3.4-5 3.4Z"/>',
  // 游玩时长统计
  '<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2.2"/>',
  // Mod 管理
  '<rect x="4" y="4" width="6.8" height="6.8" rx="1.6"/><rect x="13.2" y="4" width="6.8" height="6.8" rx="1.6"/><rect x="4" y="13.2" width="6.8" height="6.8" rx="1.6"/><path d="M16.6 13.4v6.4M13.4 16.6h6.4"/>',
  // 安心可靠
  '<path d="m12 3 7 2.8v5.4c0 4.3-2.9 7.8-7 9.8-4.1-2-7-5.5-7-9.8V5.8Z"/><path d="m9 11.6 2.1 2.1 3.9-4"/>'
]

const gradients = [
  'from-brand-400 to-brand-600',
  'from-blossom-400 to-blossom-500',
  'from-brand-500 to-blossom-400',
  'from-[#7c6dff] to-[#5a4bd6]',
  'from-[#f759ab] to-[#c73e85]',
  'from-[#6c5ce7] to-[#4a3db5]'
]

const count = 6
</script>

<template>
  <section id="features" class="relative scroll-mt-20 py-20 lg:py-28">
    <div class="mx-auto max-w-6xl px-5">
      <div class="mx-auto max-w-2xl text-center">
        <h2 class="text-3xl font-bold text-[#1a1a2e] lg:text-4xl">
          {{ t('features.title') }}
        </h2>
        <p class="mt-4 text-base text-[#6b6b85]">{{ t('features.subtitle') }}</p>
      </div>

      <div class="mt-12 grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
        <div
          v-for="i in count"
          :key="i"
          class="group rounded-2xl border border-white bg-white/70 p-6 shadow-sm backdrop-blur transition hover:-translate-y-1 hover:shadow-xl hover:shadow-brand-100"
        >
          <div
            class="flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br text-white shadow-md transition group-hover:scale-105"
            :class="gradients[i - 1]"
          >
            <svg
              viewBox="0 0 24 24"
              class="h-6 w-6"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
              v-html="icons[i - 1]"
            />
          </div>
          <h3 class="mt-4 text-lg font-semibold text-[#1a1a2e]">
            {{ t(`features.items.${i - 1}.title`) }}
          </h3>
          <p class="mt-2 text-sm leading-relaxed text-[#6b6b85]">
            {{ t(`features.items.${i - 1}.desc`) }}
          </p>
        </div>
      </div>
    </div>
  </section>
</template>
