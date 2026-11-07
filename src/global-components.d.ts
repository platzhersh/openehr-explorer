// Type declarations for medblocks-ui web components used in Vue templates.
// vite.config.mjs treats every `mb-*` tag as a custom element at runtime, but
// the Vue language tools (v4 / TypeScript 7.1 content mapper) type-check
// unknown tags strictly, so each one used in a template is declared here.
import type { DefineComponent } from "vue";

declare module "vue" {
  interface GlobalComponents {
    "mb-auto-form": DefineComponent<Record<string, unknown>>;
  }
}
