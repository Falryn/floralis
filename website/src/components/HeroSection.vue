<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import HeroArt from './HeroArt.vue'
import PetalDrift from './PetalDrift.vue'
import { GITHUB_REPO } from '../constants'

const { t } = useI18n()
</script>

<template>
  <section id="top" class="relative overflow-hidden">
    <div
      class="absolute inset-0 bg-gradient-to-br from-[#fce4f0] via-[#e8e0f7] to-[#dce8f8]"
    />
    <PetalDrift />

    <div
      class="relative mx-auto grid max-w-6xl items-center gap-12 px-5 pb-20 pt-14 lg:grid-cols-2 lg:pb-28 lg:pt-20"
    >
      <div class="text-center lg:text-left">
        <span
          class="inline-block rounded-full border border-white/70 bg-white/60 px-4 py-1.5 text-sm font-medium text-brand-600 shadow-sm backdrop-blur"
        >
          {{ t('hero.badge') }}
        </span>

        <h1 class="mt-6 text-5xl font-bold leading-tight tracking-wide lg:text-6xl">
          <span class="text-[#1a1a2e]">花譜</span>
          <span
            class="ml-3 bg-gradient-to-r from-brand-500 via-brand-400 to-blossom-500 bg-clip-text text-transparent"
          >
            Floralis
          </span>
        </h1>

        <p class="mt-4 text-2xl font-semibold text-[#3a3a55]">
          {{ t('hero.tagline') }}
        </p>

        <p class="mx-auto mt-4 max-w-xl text-base leading-relaxed text-[#6b6b85] lg:mx-0">
          {{ t('hero.description') }}
        </p>

        <div class="mt-8 flex flex-wrap items-center justify-center gap-4 lg:justify-start">
          <a
            href="#download"
            class="rounded-full bg-gradient-to-r from-brand-500 to-blossom-500 px-7 py-3 text-base font-semibold text-white shadow-lg shadow-brand-200 transition hover:-translate-y-0.5 hover:brightness-110"
          >
            {{ t('hero.ctaDownload') }}
          </a>
          <a
            :href="GITHUB_REPO"
            target="_blank"
            rel="noopener"
            class="rounded-full border border-brand-200 bg-white/70 px-7 py-3 text-base font-semibold text-brand-600 backdrop-blur transition hover:-translate-y-0.5 hover:border-brand-300 hover:bg-white"
          >
            {{ t('hero.ctaGithub') }}
          </a>
        </div>

        <p class="mt-5 text-sm text-[#8b8ba3]">{{ t('hero.platform') }}</p>
      </div>

      <div class="relative mx-auto w-full max-w-lg lg:max-w-none">
        <div
          class="animate-sway relative overflow-hidden rounded-3xl shadow-2xl shadow-brand-200/60 ring-1 ring-white/60"
        >
          <div class="aspect-[4/3]">
            <HeroArt />
          </div>
        </div>

        <div
          class="animate-float absolute -bottom-10 -left-4 w-3/5 rotate-[-4deg] overflow-hidden rounded-xl border border-white/70 bg-white shadow-2xl shadow-brand-300/40 lg:-left-10"
        >
          <div class="flex items-center gap-1.5 border-b border-[#f0f0f5] px-3 py-2">
            <span class="h-2.5 w-2.5 rounded-full bg-blossom-300" />
            <span class="h-2.5 w-2.5 rounded-full bg-[#ffd97a]" />
            <span class="h-2.5 w-2.5 rounded-full bg-[#9be3b8]" />
          </div>
          <img
            src="/screenshots/01-library.png"
            alt="Floralis UI preview"
            class="block w-full"
            loading="lazy"
          />
        </div>
      </div>
    </div>
  </section>
</template>
