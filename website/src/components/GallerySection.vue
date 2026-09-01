<script setup lang="ts">
import { onBeforeUnmount, ref, watchEffect } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const shots = [
  { file: '01-library.png', span: 'lg:col-span-3' },
  { file: '02-detail.png', span: 'lg:col-span-3' },
  { file: '03-import.png', span: 'lg:col-span-2' },
  { file: '04-mods.png', span: 'lg:col-span-2' },
  { file: '05-about.png', span: 'lg:col-span-2' }
]

const active = ref<number | null>(null)

function close() {
  active.value = null
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') close()
}

watchEffect(() => {
  if (active.value !== null) {
    document.addEventListener('keydown', onKeydown)
    document.body.style.overflow = 'hidden'
  } else {
    document.removeEventListener('keydown', onKeydown)
    document.body.style.overflow = ''
  }
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKeydown)
  document.body.style.overflow = ''
})
</script>

<template>
  <section
    id="gallery"
    class="relative scroll-mt-20 bg-gradient-to-b from-transparent via-[#f3effb] to-transparent py-20 lg:py-28"
  >
    <div class="mx-auto max-w-6xl px-5">
      <div class="mx-auto max-w-2xl text-center">
        <h2 class="text-3xl font-bold text-[#1a1a2e] lg:text-4xl">
          {{ t('gallery.title') }}
        </h2>
        <p class="mt-4 text-base text-[#6b6b85]">{{ t('gallery.subtitle') }}</p>
      </div>

      <div class="mt-12 grid gap-6 sm:grid-cols-2 lg:grid-cols-6">
        <figure
          v-for="(shot, idx) in shots"
          :key="shot.file"
          class="group cursor-zoom-in sm:col-span-1"
          :class="shot.span"
          @click="active = idx"
        >
          <div
            class="overflow-hidden rounded-2xl border border-white bg-white shadow-md transition group-hover:-translate-y-1 group-hover:shadow-xl group-hover:shadow-brand-100"
          >
            <img
              :src="`/screenshots/${shot.file}`"
              :alt="t(`gallery.shots.${idx}.title`)"
              class="block w-full transition duration-300 group-hover:scale-[1.02]"
              loading="lazy"
            />
          </div>
          <figcaption class="mt-3 text-center text-sm font-medium text-[#6b6b85]">
            {{ t(`gallery.shots.${idx}.title`) }}
          </figcaption>
        </figure>
      </div>
    </div>

    <Teleport to="body">
      <div
        v-if="active !== null"
        class="fixed inset-0 z-[100] flex items-center justify-center bg-[#1a1a2e]/85 p-6 backdrop-blur-sm"
        role="dialog"
        aria-modal="true"
        :aria-label="t(`gallery.shots.${active}.title`)"
        @click.self="close"
      >
        <div class="relative max-h-full">
          <img
            :src="`/screenshots/${shots[active].file}`"
            :alt="t(`gallery.shots.${active}.title`)"
            class="max-h-[85vh] w-auto rounded-xl shadow-2xl"
          />
          <button
            type="button"
            class="absolute -right-3 -top-3 flex h-9 w-9 items-center justify-center rounded-full bg-white text-[#1a1a2e] shadow-lg transition hover:bg-brand-50"
            :aria-label="t('gallery.close')"
            @click="close"
          >
            <svg viewBox="0 0 24 24" class="h-4.5 w-4.5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
              <path d="m6 6 12 12M18 6 6 18" />
            </svg>
          </button>
        </div>
      </div>
    </Teleport>
  </section>
</template>
