import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { fn } from "storybook/test";
import TemplateOverwriteConfirmDialog from "./TemplateOverwriteConfirmDialog.vue";

const meta: Meta<typeof TemplateOverwriteConfirmDialog> = {
  title: "Components/TemplateOverwriteConfirmDialog",
  component: TemplateOverwriteConfirmDialog,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Explicit confirmation shown before an uploaded OPT replaces an existing template on the server. Cancel is focused by default; Escape closes only this dialog.",
      },
    },
  },
  args: {
    templateId: "International_Patient_Summary.v1",
    serverName: "Local EHRBase",
    onConfirm: fn(),
    onCancel: fn(),
  },
};

export default meta;
type Story = StoryObj<typeof TemplateOverwriteConfirmDialog>;

/** The template ID and server are known. */
export const Default: Story = {};

/** The template ID couldn't be read from the OPT. */
export const UnknownTemplate: Story = {
  args: { templateId: null, serverName: null },
};
