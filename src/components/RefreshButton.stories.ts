import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { expect, userEvent, within } from "storybook/test";
import RefreshButton from "./RefreshButton.vue";

const meta: Meta<typeof RefreshButton> = {
  title: "Components/RefreshButton",
  component: RefreshButton,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Reusable icon-only refresh button (see OEH-51), companion to CopyButton (OEH-37) — every 'Refresh' text button in the app shares the same icon-button + tooltip pattern. Hover shows a tooltip with the refresh label; while `loading` is true the icon spins and the tooltip reads 'Refreshing…'.",
      },
    },
  },
  args: {
    title: "Refresh",
  },
};

export default meta;
type Story = StoryObj<typeof RefreshButton>;

export const Ghost: Story = {
  args: { variant: "ghost", size: "sm" },
};

export const Bordered: Story = {
  args: { variant: "bordered", size: "md" },
};

export const Loading: Story = {
  args: { variant: "bordered", size: "md", loading: true },
};

/** All four size/variant combinations side by side. */
export const AllVariants: Story = {
  render: (args) => ({
    components: { RefreshButton },
    setup() {
      return { args };
    },
    template: `
      <div style="display: flex; gap: 16px; align-items: center;">
        <RefreshButton v-bind="args" size="sm" variant="ghost" />
        <RefreshButton v-bind="args" size="sm" variant="bordered" />
        <RefreshButton v-bind="args" size="md" variant="ghost" />
        <RefreshButton v-bind="args" size="md" variant="bordered" />
      </div>
    `,
  }),
};

/** Clicking emits a `click` event that the parent uses to trigger the refresh. */
export const ClickToRefresh: Story = {
  args: { variant: "bordered", size: "md" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const button = canvas.getByRole("button");
    await expect(button).toHaveAccessibleName("Refresh");
    await userEvent.click(button);
  },
};
