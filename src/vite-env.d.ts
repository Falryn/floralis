/// <reference types="vite/client" />

export {};

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

// Tauri 启用 devtools 后会在运行时向 Window 实例注入该方法，官方 .d.ts 未声明，这里补充类型
declare module "@tauri-apps/api/window" {
  interface Window {
    toggleDevTools(): Promise<void>;
  }
}
