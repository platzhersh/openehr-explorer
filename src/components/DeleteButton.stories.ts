import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { expect, userEvent, within } from "storybook/test";
import DeleteButton from "./DeleteButton.vue";

const meta: Meta<typeof DeleteButton> = {
  title: "Components/DeleteButton",
  component: DeleteButton,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Reusable icon-only delete button (see OEH-54), companion to CopyButton (OEH-37) and RefreshButton (OEH-52) — every bare 'Delete' text button in the app shares the same icon-button + tooltip pattern. Unlike Copy/Refresh, the icon reads as danger (--color-error) even at rest, matching the .btn-danger styling it replaces.",
      },
    },
  },
  args: {
    title: "Delete",
  },
};

export default meta;
type Story = StoryObj<typeof DeleteButton>;

export const Ghost: Story = {
  args: { variant: "ghost", size: "sm" },
};

export const Bordered: Story = {
  args: { variant: "bordered", size: "md" },
};

export const Disabled: Story = {
  args: { variant: "bordered", size: "md", disabled: true },
};

/** All four size/variant combinations side by side. */
export const AllVariants: Story = {
  render: (args) => ({
    components: { DeleteButton },
    setup() {
      return { args };
    },
    template: `
      <div style="display: flex; gap: 16px; align-items: center;">
        <DeleteButton v-bind="args" size="sm" variant="ghost" />
        <DeleteButton v-bind="args" size="sm" variant="bordered" />
        <DeleteButton v-bind="args" size="md" variant="ghost" />
        <DeleteButton v-bind="args" size="md" variant="bordered" />
      </div>
    `,
  }),
};

/** Clicking emits a `click` event that the parent uses to trigger the delete (typically behind a confirmation dialog). */
export const ClickToDelete: Story = {
  args: { variant: "bordered", size: "md" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const button = canvas.getByRole("button");
    await expect(button).toHaveAccessibleName("Delete");
    await userEvent.click(button);
  },
};
