import type { StorybookConfig } from "@storybook/vue3-vite";

const config: StorybookConfig = {
  // Both the app's own components (src/) and the marketing/docs site's Vue
  // islands (website/src/components/vue/) are plain Vue 3 SFCs with no
  // cross-project imports, so one Storybook can render both — see
  // ADR-0026. website/'s own build tooling (Astro) is untouched; Storybook
  // just transforms these .vue files directly via the app's Vite/Vue
  // plugin, the same way it already does for src/.
  stories: ["../src/**/*.stories.@(js|ts)", "../website/src/components/vue/**/*.stories.@(js|ts)"],
  staticDirs: ["../website/public"],
  addons: ["@storybook/addon-docs", "@storybook/addon-a11y"],
  framework: "@storybook/vue3-vite",
};

export default config;
