import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { fn } from "storybook/test";
import ConfirmDialog from "./ConfirmDialog.vue";

const meta: Meta<typeof ConfirmDialog> = {
  title: "Components/ConfirmDialog",
  component: ConfirmDialog,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Generic confirmation for destructive actions. Cancel is focused by default; Escape closes only this dialog; the rest of the page is inert while open.",
      },
    },
  },
  args: {
    title: "Clear request history?",
    confirmLabel: "Clear history",
    onConfirm: fn(),
    onCancel: fn(),
  },
  render: (args) => ({
    components: { ConfirmDialog },
    setup: () => ({ args }),
    template: `<ConfirmDialog v-bind="args"><p>This removes all 12 logged requests. It can't be undone.</p></ConfirmDialog>`,
  }),
};

export default meta;
type Story = StoryObj<typeof ConfirmDialog>;

export const Default: Story = {};
