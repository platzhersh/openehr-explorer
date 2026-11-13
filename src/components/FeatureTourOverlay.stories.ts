import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { onUnmounted } from "vue";
import { createPinia, setActivePinia } from "pinia";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import FeatureTourOverlay from "./FeatureTourOverlay.vue";
import { useSettingsStore } from "../stores/settings";
import { useTourStore } from "../stores/tour";
import { APP_INTRO_TOUR_ID } from "../lib/tours";

// FeatureTourOverlay reads everything from the `tour` Pinia store and finds
// its step targets with `document.querySelector`, so the story supplies a
// stand-in app chrome carrying the same `data-tour` attributes the real
// App.vue does (the app-intro tour's five steps), plus a button that starts
// the tour — the button is the "opener" that focus returns to afterwards.
const meta: Meta<typeof FeatureTourOverlay> = {
  title: "Components/FeatureTourOverlay",
  component: FeatureTourOverlay,
  parameters: {
    layout: "fullscreen",
    docs: {
      description: {
        component:
          "Route-aware feature tour tooltip. Rendered as a native modal `<dialog>` (`showModal()`): the page behind it is inert, Tab/Shift+Tab stay inside the tooltip, Escape skips the tour, and focus returns to the element that started it.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof FeatureTourOverlay>;

export const AppIntro: Story = {
  render: () => ({
    components: { FeatureTourOverlay },
    setup() {
      setActivePinia(createPinia());
      // Persisting completion calls `save_settings`; echo it back instead of
      // hitting a missing Tauri backend.
      mockIPC((cmd, payload) => {
        if (cmd === "save_settings") return (payload as { settings: unknown }).settings;
      });
      onUnmounted(() => clearMocks());
      useSettingsStore().loaded = true;
      const tour = useTourStore();
      return { startTour: () => tour.start(APP_INTRO_TOUR_ID) };
    },
    template: `
      <div class="story-chrome" style="display:flex;flex-direction:column;min-height:100vh;color:var(--color-text)">
        <component :is="'style'">.story-chrome a { color: var(--color-primary); }</component>
        <header style="display:flex;gap:16px;align-items:center;padding:12px 16px;border-bottom:1px solid var(--color-border)">
          <select data-tour="server-select" aria-label="Server"><option>Local EHRBase</option></select>
          <nav data-tour="nav-tabs" style="display:flex;gap:12px">
            <a href="#dashboard">Dashboard</a><a href="#ehrs">EHRs</a><a href="#templates">Templates</a><a href="#aql">AQL</a>
          </nav>
          <a data-tour="nav-docs" href="#docs" style="margin-left:auto">Docs</a>
          <a data-tour="nav-settings" href="#settings">Settings</a>
        </header>
        <main style="flex:1;padding:24px">
          <button id="start-tour" type="button" class="btn btn-primary" @click="startTour">Start app tour</button>
          <p><a href="#background-link">A background link that must not receive focus during the tour</a></p>
          <input aria-label="Background input" placeholder="Background input" />
        </main>
        <footer data-tour="inspector-header" style="padding:12px 16px;border-top:1px solid var(--color-border)">Request Inspector</footer>
        <FeatureTourOverlay />
      </div>`,
  }),
};
