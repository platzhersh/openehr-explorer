import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { expect, userEvent, within } from "storybook/test";
import EditButton from "./EditButton.vue";

const meta: Meta<typeof EditButton> = {
  title: "Components/EditButton",
  component: EditButton,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Reusable icon-only edit button (see OEH-55), companion to CopyButton (OEH-37), RefreshButton (OEH-52), and DeleteButton (OEH-54) — every bare 'Edit' text button in the app shares the same icon-button + tooltip pattern.",
      },
    },
  },
  args: {
    title: "Edit",
  },
};

export default meta;
type Story = StoryObj<typeof EditButton>;

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
    components: { EditButton },
    setup() {
      return { args };
    },
    template: `
      <div style="display: flex; gap: 16px; align-items: center;">
        <EditButton v-bind="args" size="sm" variant="ghost" />
        <EditButton v-bind="args" size="sm" variant="bordered" />
        <EditButton v-bind="args" size="md" variant="ghost" />
        <EditButton v-bind="args" size="md" variant="bordered" />
      </div>
    `,
  }),
};

/** Clicking emits a `click` event that the parent uses to enter edit mode. */
export const ClickToEdit: Story = {
  args: { variant: "bordered", size: "md" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const button = canvas.getByRole("button");
    await expect(button).toHaveAccessibleName("Edit");
    await userEvent.click(button);
  },
};
