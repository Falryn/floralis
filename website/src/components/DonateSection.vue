<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { KOFI_URL, PAYPAL_URL } from '../constants'

const { t } = useI18n()
</script>

<template>
  <section id="support" class="scroll-mt-20 pb-20 lg:pb-28">
    <div class="mx-auto max-w-6xl px-5">
      <div class="mx-auto max-w-2xl text-center">
        <h2 class="text-3xl font-bold text-[#1a1a2e] lg:text-4xl">
          {{ t('donate.title') }}
        </h2>
        <p class="mt-4 text-base text-[#6b6b85]">{{ t('donate.subtitle') }}</p>
      </div>

      <div class="mt-12 grid items-start gap-10 lg:grid-cols-2">
        <div>
          <h3 class="text-center text-sm font-semibold uppercase tracking-wider text-brand-500">
            {{ t('donate.domestic') }}
          </h3>
          <div class="mt-5 flex flex-wrap justify-center gap-6">
            <figure
              class="rounded-2xl border border-white bg-white/80 p-4 shadow-md backdrop-blur"
            >
              <img
                src="/donate/wechat.jpg"
                :alt="t('donate.wechat')"
                class="h-40 w-40 rounded-lg object-cover"
                loading="lazy"
              />
              <figcaption class="mt-3 text-center text-sm font-medium text-[#4a4a68]">
                {{ t('donate.wechat') }}
              </figcaption>
            </figure>
            <figure
              class="rounded-2xl border border-white bg-white/80 p-4 shadow-md backdrop-blur"
            >
              <img
                src="/donate/alipay.jpg"
                :alt="t('donate.alipay')"
                class="h-40 w-40 rounded-lg object-cover"
                loading="lazy"
              />
              <figcaption class="mt-3 text-center text-sm font-medium text-[#4a4a68]">
                {{ t('donate.alipay') }}
              </figcaption>
            </figure>
          </div>
        </div>

        <div>
          <h3 class="text-center text-sm font-semibold uppercase tracking-wider text-brand-500">
            {{ t('donate.overseas') }}
          </h3>
          <div class="mt-5 grid gap-4">
            <a
              :href="KOFI_URL"
              target="_blank"
              rel="noopener"
              class="group flex items-center justify-between rounded-2xl border border-white bg-white/80 px-6 py-5 shadow-md backdrop-blur transition hover:-translate-y-0.5 hover:shadow-lg"
            >
              <span>
                <span class="block text-base font-semibold text-[#1a1a2e]">
                  {{ t('donate.kofi') }}
                </span>
                <span class="mt-0.5 block text-sm text-[#8b8ba3]">
                  {{ t('donate.kofiDesc') }}
                </span>
              </span>
              <span
                class="text-brand-400 transition group-hover:translate-x-1 group-hover:text-brand-500"
              >
                <svg viewBox="0 0 24 24" class="h-5 w-5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M5 12h14M13 6l6 6-6 6" />
                </svg>
              </span>
            </a>
            <a
              :href="PAYPAL_URL"
              target="_blank"
              rel="noopener"
              class="group flex items-center justify-between rounded-2xl border border-white bg-white/80 px-6 py-5 shadow-md backdrop-blur transition hover:-translate-y-0.5 hover:shadow-lg"
            >
              <span>
                <span class="block text-base font-semibold text-[#1a1a2e]">
                  {{ t('donate.paypal') }}
                </span>
                <span class="mt-0.5 block text-sm text-[#8b8ba3]">
                  {{ t('donate.paypalDesc') }}
                </span>
              </span>
              <span
                class="text-brand-400 transition group-hover:translate-x-1 group-hover:text-brand-500"
              >
                <svg viewBox="0 0 24 24" class="h-5 w-5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M5 12h14M13 6l6 6-6 6" />
                </svg>
              </span>
            </a>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>
