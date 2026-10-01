import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { expect, userEvent, within } from "storybook/test";
import AppSidebar from "./AppSidebar.vue";
import { mockTauriStores } from "../lib/storybook-tauri";

const meta: Meta<typeof AppSidebar> = {
  title: "Components/AppSidebar",
  component: AppSidebar,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Main navigation. Tabs and their Ctrl/Cmd+N shortcuts both come from `lib/navShortcuts.ts` (OEH-98). Holding Cmd (macOS) / Ctrl (elsewhere) for ~400 ms overlays each tab's shortcut key; releasing the key or blurring the window hides it again.",
      },
    },
  },
  render: () => ({
    components: { AppSidebar },
    setup() {
      mockTauriStores((cmd) => (cmd === "get_app_version" ? "0.0.0" : null));
    },
    template: '<div style="width: 220px; display: flex; height: 420px"><AppSidebar /></div>',
  }),
};

export default meta;
type Story = StoryObj<typeof AppSidebar>;

export const Default: Story = {};

/** Holding the platform modifier reveals the per-tab shortcut badges. */
export const ShortcutHintsWhileModifierHeld: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const isMac = /mac/i.test(navigator.platform || "");
    await userEvent.keyboard(isMac ? "{Meta>}" : "{Control>}");
    await new Promise((r) => setTimeout(r, 500));
    await expect(canvas.getAllByText(/^(⌘|Ctrl\+)[1-6,]$/).length).toBe(7);
    await userEvent.keyboard(isMac ? "{/Meta}" : "{/Control}");
    await expect(canvas.queryAllByText(/^(⌘|Ctrl\+)[1-6,]$/).length).toBe(0);
  },
};
