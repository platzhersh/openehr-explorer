import type { Preview } from "@storybook/vue3-vite";
import { setup } from "@storybook/vue3";
import { createMemoryHistory, createRouter } from "vue-router";
import "./preview.css";
// The real production override file (see medblocks-overrides.stories.ts) —
// not a Storybook-only copy, so it can't drift from what the app ships.
import "../src/styles/medblocks-overrides.css";

// Installed globally (rather than per-story) so any view/feature story that
// uses `<router-link>` / `useRouter()` — e.g. Dashboard.vue — just works
// without each story file having to wire this up itself. Memory history
// keeps navigation isolated to the Storybook preview iframe instead of
// touching the real address bar. Routes are stub targets only (a bare div)
// — enough for router-link to resolve an href and for `router.push()` to
// not throw; the real destination views aren't rendered here.
const router = createRouter({
  history: createMemoryHistory(),
  routes: [
    { path: "/", redirect: "/dashboard" },
    { path: "/dashboard", name: "dashboard", component: { template: "<div />" } },
    { path: "/ehrs", name: "ehrs", component: { template: "<div />" } },
    { path: "/templates", name: "templates", component: { template: "<div />" } },
    { path: "/aql", name: "aql", component: { template: "<div />" } },
    { path: "/servers", name: "servers", component: { template: "<div />" } },
    { path: "/settings", name: "settings", component: { template: "<div />" } },
  ],
});

setup((app) => {
  app.use(router);
});

const preview: Preview = {
  parameters: {
    backgrounds: {
      default: "app",
      values: [{ name: "app", value: "#1a1a2e" }],
    },
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
    a11y: {
      // 'todo' - show a11y violations in the test UI only
      // 'error' - fail CI on a11y violations
      // 'off' - skip a11y checks entirely
      test: "todo",
    },
  },
};

export default preview;
