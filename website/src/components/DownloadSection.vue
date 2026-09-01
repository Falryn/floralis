<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { FALLBACK_VERSION, GITHUB_RELEASES } from '../constants'

const { t } = useI18n()

const version = ref(FALLBACK_VERSION)

onMounted(async () => {
  try {
    const res = await fetch(
      'https://api.github.com/repos/Falryn/floralis/releases/latest',
      { headers: { Accept: 'application/vnd.github+json' } }
    )
    if (!res.ok) return
    const data: { tag_name?: string } = await res.json()
    if (data.tag_name) version.value = data.tag_name
  } catch {
    /* offline or rate-limited: keep fallback version */
  }
})
</script>

<template>
  <section id="download" class="scroll-mt-20 py-20 lg:py-28">
    <div class="mx-auto max-w-6xl px-5">
      <div
        class="relative overflow-hidden rounded-3xl bg-gradient-to-br from-brand-500 via-[#8f5de0] to-blossom-500 px-6 py-14 text-center text-white shadow-2xl shadow-brand-200 lg:px-16 lg:py-20"
      >
        <div
          class="pointer-events-none absolute -left-16 -top-16 h-56 w-56 rounded-full bg-white/10"
        />
        <div
          class="pointer-events-none absolute -bottom-20 -right-10 h-64 w-64 rounded-full bg-white/10"
        />

        <h2 class="relative text-3xl font-bold lg:text-4xl">
          {{ t('download.title') }}
        </h2>
        <p class="relative mt-4 text-white/85">{{ t('download.subtitle') }}</p>

        <div class="relative mt-8 flex flex-wrap items-center justify-center gap-3">
          <span
            class="rounded-full border border-white/40 bg-white/15 px-4 py-1.5 text-sm font-medium backdrop-blur"
          >
            {{ t('download.latest') }} {{ version }}
          </span>
          <span
            class="rounded-full border border-white/40 bg-white/15 px-4 py-1.5 text-sm font-medium backdrop-blur"
          >
            {{ t('download.windows') }}
          </span>
        </div>

        <div class="relative mt-8">
          <a
            :href="GITHUB_RELEASES"
            target="_blank"
            rel="noopener"
            class="inline-flex items-center gap-2 rounded-full bg-white px-8 py-3.5 text-base font-bold text-brand-600 shadow-xl transition hover:-translate-y-0.5 hover:shadow-2xl"
          >
            <svg viewBox="0 0 24 24" class="h-5 w-5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 3v11" />
              <path d="m7.5 9.5 4.5 4.5 4.5-4.5" />
              <path d="M4.5 17.5V19a1.5 1.5 0 0 0 1.5 1.5h12a1.5 1.5 0 0 0 1.5-1.5v-1.5" />
            </svg>
            {{ t('download.btn') }}
          </a>
        </div>

        <p class="relative mt-6 text-sm text-white/75">{{ t('download.zipNote') }}</p>
        <p class="relative mt-2 text-sm text-white/75">{{ t('download.license') }}</p>
      </div>
    </div>
  </section>
</template>
