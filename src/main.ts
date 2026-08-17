/**
 * Floralis 应用入口
 * 
 * 初始化 Vue 3 应用，注册 Pinia 状态管理、国际化等插件
 */

import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import "./style.css";
import i18n from "./i18n";
import { addToast } from "./composables/useToast";
import { InvokeError } from "./utils/invoke";

const app = createApp(App);

app.config.errorHandler = (err, instance, info) => {
  console.error("[Floralis] 全局错误:", err, "\n组件:", instance?.$options?.name || "unknown", "\n信息:", info);
};

app.config.warnHandler = (msg) => {
  if (import.meta.env.PROD) return;
  console.warn("[Floralis] 警告:", msg);
};

// 统一失败兜底：未被调用点捕获的 invoke 失败（如 launchGame 等 fire-and-forget 调用）
// 以 toast 形式对用户可见，避免错误静默丢失
window.addEventListener("unhandledrejection", (event) => {
  if (event.reason instanceof InvokeError) {
    event.preventDefault();
    addToast(event.reason.message, "error");
  }
});

app.use(createPinia());
app.use(i18n);
app.mount("#app");
