import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { onUnmounted } from "vue";
import { createPinia, setActivePinia } from "pinia";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import WhatsNewModal from "./WhatsNewModal.vue";
import { useSettingsStore } from "../stores/settings";
import { useWhatsNewStore } from "../stores/whatsNew";

const meta: Meta<typeof WhatsNewModal> = {
  title: "Components/WhatsNewModal",
  component: WhatsNewModal,
  parameters: {
    layout: "fullscreen",
    docs: {
      description: {
        component:
          'Shown once per app-version upgrade, or reopened from Settings. Rendered as a native modal `<dialog>` (`showModal()`): the page behind it is inert, Tab/Shift+Tab stay inside the dialog, Escape and "Got it" share one close path, and focus returns to the element that opened it.',
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof WhatsNewModal>;

export const Default: Story = {
  render: () => ({
    components: { WhatsNewModal },
    setup() {
      setActivePinia(createPinia());
      mockIPC((cmd, payload) => {
        if (cmd === "save_settings") return (payload as { settings: unknown }).settings;
      });
      onUnmounted(() => clearMocks());
      const settingsStore = useSettingsStore();
      settingsStore.loaded = true;
      const whatsNew = useWhatsNewStore();
      // Start open, like the real post-upgrade launch.
      whatsNew.showLatest();
      return { whatsNew };
    },
    template: `
      <div class="story-chrome" style="padding:24px;color:var(--color-text)">
        <component :is="'style'">.story-chrome a { color: var(--color-primary); }</component>
        <button id="open-whats-new" type="button" class="btn btn-primary" @click="whatsNew.showLatest()">View What's New</button>
        <p><a href="#background-link">A background link that must not receive focus while the dialog is open</a></p>
        <input aria-label="Background input" placeholder="Background input" />
        <WhatsNewModal v-if="whatsNew.visible" current-version="0.10.2" />
      </div>`,
  }),
};
