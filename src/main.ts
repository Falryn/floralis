/**
 * Floralis 应用入口
 * 
 * 初始化 Vue 3 应用，注册 Pinia 状态管理、虚拟滚动、国际化等插件
 */

import { createApp } from "vue";
import { createPinia } from "pinia";
import VueVirtualScroller from "vue-virtual-scroller";
import "vue-virtual-scroller/dist/vue-virtual-scroller.css";
import App from "./App.vue";
import "./style.css";
import i18n from "./i18n";

const app = createApp(App);

app.config.errorHandler = (err, instance, info) => {
  console.error("[Floralis] 全局错误:", err, "\n组件:", instance?.$options?.name || "unknown", "\n信息:", info);
};

app.config.warnHandler = (msg) => {
  if (import.meta.env.PROD) return;
  console.warn("[Floralis] 警告:", msg);
};

app.use(createPinia());
app.use(VueVirtualScroller);
app.use(i18n);
app.mount("#app");
