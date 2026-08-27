import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { expect, userEvent, within } from "storybook/test";
import CopyButton from "./CopyButton.vue";

const meta: Meta<typeof CopyButton> = {
  title: "Components/CopyButton",
  component: CopyButton,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Reusable icon-only copy-to-clipboard button (see OEH-37). Originally built inline in JsonViewer.vue, extracted so every 'Copy' button in the app shares the same icon + checkmark pattern — now used by JsonTreeNode, FlatPathPanel, OptMetadata, JsonViewer, and XmlViewer.",
      },
    },
  },
  args: {
    text: "openEHR-EHR-OBSERVATION.blood_pressure.v2",
    title: "Copy to clipboard",
  },
};

export default meta;
type Story = StoryObj<typeof CopyButton>;

export const Ghost: Story = {
  args: { variant: "ghost", size: "sm" },
};

export const Bordered: Story = {
  args: { variant: "bordered", size: "md" },
};

/** All four size/variant combinations side by side. */
export const AllVariants: Story = {
  render: (args) => ({
    components: { CopyButton },
    setup() {
      return { args };
    },
    template: `
      <div style="display: flex; gap: 16px; align-items: center;">
        <CopyButton v-bind="args" size="sm" variant="ghost" />
        <CopyButton v-bind="args" size="sm" variant="bordered" />
        <CopyButton v-bind="args" size="md" variant="ghost" />
        <CopyButton v-bind="args" size="md" variant="bordered" />
      </div>
    `,
  }),
};

/** Clicking copies `text` to the clipboard and swaps the icon to a checkmark for ~1.2s. */
export const ClickToCopy: Story = {
  args: { variant: "bordered", size: "md" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const button = canvas.getByRole("button");
    await userEvent.click(button);
    await expect(button).toHaveAccessibleName("Copied");
  },
};
