import { createI18n } from 'vue-i18n'
import zhCN from './locales/zh-CN.json'
import enUS from './locales/en-US.json'
import jaJP from './locales/ja-JP.json'

export type Locale = 'zh-CN' | 'en-US' | 'ja-JP'

const STORAGE_KEY = 'floralis-site-locale'
const SUPPORTED: Locale[] = ['zh-CN', 'en-US', 'ja-JP']

function detectLocale(): Locale {
  try {
    const saved = localStorage.getItem(STORAGE_KEY)
    if (saved && (SUPPORTED as string[]).includes(saved)) return saved as Locale
  } catch {
    /* storage unavailable */
  }
  const langs =
    navigator.languages && navigator.languages.length > 0
      ? navigator.languages
      : [navigator.language || 'en-US']
  for (const lang of langs) {
    const lower = lang.toLowerCase()
    if (lower.startsWith('zh')) return 'zh-CN'
    if (lower.startsWith('ja')) return 'ja-JP'
  }
  return 'en-US'
}

const initial = detectLocale()

export const i18n = createI18n({
  legacy: false,
  locale: initial,
  fallbackLocale: 'en-US',
  messages: {
    'zh-CN': zhCN,
    'en-US': enUS,
    'ja-JP': jaJP
  }
})

export function setLocale(locale: Locale): void {
  i18n.global.locale.value = locale
  document.documentElement.lang = locale
  try {
    localStorage.setItem(STORAGE_KEY, locale)
  } catch {
    /* storage unavailable */
  }
}

document.documentElement.lang = initial
