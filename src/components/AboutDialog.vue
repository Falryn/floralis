<script setup lang="ts">
import { ref } from "vue";
import { open as openUrl } from "@tauri-apps/plugin-shell";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const emit = defineEmits<{
  close: [];
}>();

// 版本号需与 tauri.conf.json / package.json 保持一致
const APP_VERSION = "0.1.0";

const REPO_URL = "https://github.com/Falryn/floralis";
const ISSUES_URL = "https://github.com/Falryn/floralis/issues/new/choose";
const GPL_URL = "https://www.gnu.org/licenses/gpl-3.0.html";

// 海外渠道：注册后替换（与 .github/FUNDING.yml 保持一致）
// Ko-fi 一次性打赏 0 抽成，海外用户可刷卡/PayPal，提现经 PayPal
const KOFI_URL = "https://ko-fi.com/falryn";
const PAYPAL_URL = "https://paypal.me/falryn";
const hasOverseas = !KOFI_URL.includes("TODO_REPLACE") || !PAYPAL_URL.includes("TODO_REPLACE");

// 国内收款码：图片位于 public/donate/（wechat.jpg / alipay.jpg）
// 图片缺失时 @error 自动隐藏对应按钮
const wechatOk = ref(true);
const alipayOk = ref(true);

// 收款码灯箱：点击按钮弹出大图
const qrView = ref<"wechat" | "alipay" | null>(null);
</script>

<template>
  <div
    class="fixed inset-0 z-[60] flex items-center justify-center bg-black/30 backdrop-blur-sm"
    @click.self="emit('close')"
  >
    <div class="bg-modal-bg rounded-2xl shadow-2xl w-[420px] p-8 space-y-6">
      <!-- 应用标识 -->
      <div class="flex flex-col items-center gap-3 text-center">
        <img src="/app-icon.png" alt="icon" class="w-16 h-16 rounded-2xl shadow-md" />
        <div>
          <h3 class="text-lg font-bold bg-gradient-to-r from-primary-500 to-sakura-400 bg-clip-text text-transparent">
            {{ t('about.appName') }}
          </h3>
          <p class="text-xs text-text-sub mt-1">{{ t('about.version') }} {{ APP_VERSION }}</p>
        </div>
        <p class="text-sm text-text-sub leading-relaxed">{{ t('about.tagline') }}</p>
      </div>

      <!-- 许可证 -->
      <div class="rounded-xl bg-input-bg/50 px-4 py-3 space-y-1">
        <div class="flex items-center justify-between">
          <span class="text-sm text-text-main font-medium">{{ t('about.license') }}</span>
          <button
            class="text-xs text-primary-500 hover:text-primary-600 underline"
            @click="openUrl(GPL_URL)"
          >
            GPL-3.0
          </button>
        </div>
        <p class="text-xs text-text-sub leading-relaxed">{{ t('about.licenseDesc') }}</p>
      </div>

      <!-- 打赏入口（国内收款码 + 海外渠道） -->
      <div class="rounded-xl bg-input-bg/50 px-4 py-3 space-y-2">
        <p class="text-sm text-text-main font-medium">{{ t('about.donate') }}</p>
        <p class="text-xs text-text-sub leading-relaxed">{{ t('about.donateDesc') }}</p>

        <div class="grid grid-cols-2 gap-2 pt-1">
          <button
            v-if="wechatOk"
            class="py-2 rounded-xl text-sm font-medium border border-border-medium text-text-sub hover:bg-code-bg transition-colors"
            @click="qrView = 'wechat'"
          >
            💚 {{ t('about.wechatPay') }}
          </button>
          <button
            v-if="alipayOk"
            class="py-2 rounded-xl text-sm font-medium border border-border-medium text-text-sub hover:bg-code-bg transition-colors"
            @click="qrView = 'alipay'"
          >
            💙 {{ t('about.alipay') }}
          </button>
          <button
            v-if="!KOFI_URL.includes('TODO_REPLACE')"
            class="py-2 rounded-xl text-sm font-medium bg-primary-500 text-white hover:bg-primary-600 transition-all shadow-sm"
            @click="openUrl(KOFI_URL)"
          >
            ☕ Ko-fi
          </button>
          <button
            v-if="!PAYPAL_URL.includes('TODO_REPLACE')"
            class="py-2 rounded-xl text-sm font-medium border border-border-medium text-text-sub hover:bg-code-bg transition-colors"
            @click="openUrl(PAYPAL_URL)"
          >
            🅿️ PayPal
          </button>
        </div>
        <p v-if="!wechatOk && !alipayOk" class="text-xs text-text-sub/70 text-center pt-1">{{ t('about.qrPending') }}</p>

        <button
          class="w-full py-2 rounded-xl text-sm font-medium border border-border-medium text-text-sub hover:bg-code-bg transition-colors"
          @click="openUrl(REPO_URL)"
        >
          ⭐ GitHub
        </button>
        <button
          class="w-full py-2 rounded-xl text-sm font-medium border border-border-medium text-text-sub hover:bg-code-bg transition-colors"
          @click="openUrl(ISSUES_URL)"
        >
          🐛 {{ t('about.reportIssue') }}
        </button>
      </div>

      <!-- 底部 -->
      <div class="flex items-center justify-between pt-1">
        <p class="text-[11px] text-text-sub/70">{{ t('about.copyright') }}</p>
        <button
          class="px-4 py-1.5 rounded-xl border border-border-medium text-sm text-text-sub hover:bg-code-bg transition-colors"
          @click="emit('close')"
        >
          {{ t('common.close') }}
        </button>
      </div>
    </div>

    <!-- 收款码存在性探针：图片缺失时自动隐藏对应按钮 -->
    <img src="/donate/wechat.jpg" class="hidden" alt="" @error="wechatOk = false" />
    <img src="/donate/alipay.jpg" class="hidden" alt="" @error="alipayOk = false" />

    <!-- 收款码灯箱 -->
    <div
      v-if="qrView"
      class="fixed inset-0 z-[70] flex items-center justify-center bg-black/50 backdrop-blur-sm"
      @click.self="qrView = null"
    >
      <div class="bg-modal-bg rounded-2xl shadow-2xl p-6 flex flex-col items-center gap-3">
        <img
          :src="qrView === 'wechat' ? '/donate/wechat.jpg' : '/donate/alipay.jpg'"
          :alt="qrView === 'wechat' ? t('about.wechatPay') : t('about.alipay')"
          class="h-72 w-auto rounded-lg border border-border-light bg-white object-contain"
        />
        <p class="text-sm font-medium text-text-main">
          {{ qrView === 'wechat' ? `💚 ${t('about.wechatPay')}` : `💙 ${t('about.alipay')}` }}
        </p>
        <p class="text-xs text-text-sub">{{ t('about.scanTip') }}</p>
        <button
          class="px-4 py-1.5 rounded-xl border border-border-medium text-sm text-text-sub hover:bg-code-bg transition-colors"
          @click="qrView = null"
        >
          {{ t('common.close') }}
        </button>
      </div>
    </div>
  </div>
</template>
