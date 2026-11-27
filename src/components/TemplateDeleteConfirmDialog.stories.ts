import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { fn } from "storybook/test";
import TemplateDeleteConfirmDialog from "./TemplateDeleteConfirmDialog.vue";

const meta: Meta<typeof TemplateDeleteConfirmDialog> = {
  title: "Components/TemplateDeleteConfirmDialog",
  component: TemplateDeleteConfirmDialog,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Explicit confirmation before a template is permanently deleted from the server. Cancel is focused by default.",
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
type Story = StoryObj<typeof TemplateDeleteConfirmDialog>;

export const Default: Story = {};

/** The server refused, e.g. because a composition uses the template. */
export const WithError: Story = {
  args: {
    error:
      "Delete failed (HTTP 422): the template is still used by at least one composition, so it can't be deleted",
  },
};
