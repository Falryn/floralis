/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

declare module "vue-virtual-scroller" {
  import type { DefineComponent } from "vue";
  export const RecycleScroller: DefineComponent<any>;
  export const DynamicScroller: DefineComponent<any>;
  export const DynamicScrollerItem: DefineComponent<any>;
  const _default: {
    install: (app: any) => void;
  };
  export default _default;
}
