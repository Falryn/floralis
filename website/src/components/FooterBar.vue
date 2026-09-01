<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { GITHUB_ISSUES, GITHUB_REPO } from '../constants'

const { t } = useI18n()
const year = new Date().getFullYear()
</script>

<template>
  <footer class="bg-[#1a1a2e] text-[#b9b9cf]">
    <div class="mx-auto grid max-w-6xl gap-10 px-5 py-14 md:grid-cols-3">
      <div>
        <div class="flex items-center gap-2.5">
          <img src="/app-icon.png" alt="Floralis" class="h-9 w-9 rounded-[10px]" />
          <span class="text-lg font-bold text-white">
            花譜 <span class="text-brand-300">Floralis</span>
          </span>
        </div>
        <p class="mt-4 max-w-xs text-sm leading-relaxed">
          {{ t('footer.tagline') }}
        </p>
      </div>

      <div>
        <h3 class="text-sm font-semibold uppercase tracking-wider text-white">
          {{ t('footer.product') }}
        </h3>
        <ul class="mt-4 space-y-2.5 text-sm">
          <li>
            <a href="#features" class="transition hover:text-white">
              {{ t('nav.features') }}
            </a>
          </li>
          <li>
            <a href="#gallery" class="transition hover:text-white">
              {{ t('nav.gallery') }}
            </a>
          </li>
          <li>
            <a href="#download" class="transition hover:text-white">
              {{ t('nav.download') }}
            </a>
          </li>
          <li>
            <a href="#support" class="transition hover:text-white">
              {{ t('nav.support') }}
            </a>
          </li>
        </ul>
      </div>

      <div>
        <h3 class="text-sm font-semibold uppercase tracking-wider text-white">
          {{ t('footer.links') }}
        </h3>
        <ul class="mt-4 space-y-2.5 text-sm">
          <li>
            <a :href="GITHUB_REPO" target="_blank" rel="noopener" class="transition hover:text-white">
              GitHub
            </a>
          </li>
          <li>
            <a :href="GITHUB_ISSUES" target="_blank" rel="noopener" class="transition hover:text-white">
              {{ t('footer.feedback') }}
            </a>
          </li>
          <li>
            <a
              :href="`${GITHUB_REPO}/blob/master/LICENSE`"
              target="_blank"
              rel="noopener"
              class="transition hover:text-white"
            >
              {{ t('footer.license') }}
            </a>
          </li>
        </ul>
      </div>
    </div>

    <div class="border-t border-white/10">
      <div
        class="mx-auto flex max-w-6xl flex-col items-center gap-2 px-5 py-6 text-center text-xs text-[#8b8ba3]"
      >
        <p>© {{ year }} 花譜 Floralis · {{ t('footer.builtWith') }}</p>
        <p>{{ t('footer.dataSource') }}</p>
      </div>
    </div>
  </footer>
</template>
