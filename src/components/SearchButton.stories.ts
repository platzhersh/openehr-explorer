import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import SearchButton from "./SearchButton.vue";

const meta: Meta<typeof SearchButton> = {
  title: "Components/SearchButton",
  component: SearchButton,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Reusable icon-only search button, companion to CopyButton (OEH-37), RefreshButton (OEH-52), DeleteButton (OEH-54) and EditButton (OEH-55). Opens a panel's search/filter overlay — the same thing Ctrl/Cmd+F does, which is otherwise discoverable only by trying it. Usually sits immediately left of a CopyButton in a viewer's floating action row, so it shares that family's sizes and variants. Its hover tooltip comes from the shared `[data-tooltip]` utility rather than markup of its own.",
      },
    },
  },
  args: {
    title: "Search (Ctrl+F)",
  },
};

export default meta;
type Story = StoryObj<typeof SearchButton>;

export const Bordered: Story = {
  args: { variant: "bordered", size: "md" },
};

export const Ghost: Story = {
  args: { variant: "ghost", size: "sm" },
};

export const Disabled: Story = {
  args: { variant: "bordered", size: "md", disabled: true },
};

/** All four size/variant combinations side by side. */
export const AllVariants: Story = {
  render: (args) => ({
    components: { SearchButton },
    setup() {
      return { args };
    },
    template: `
      <div style="display: flex; gap: 16px; align-items: center;">
        <SearchButton v-bind="args" size="sm" variant="ghost" />
        <SearchButton v-bind="args" size="sm" variant="bordered" />
        <SearchButton v-bind="args" size="md" variant="ghost" />
        <SearchButton v-bind="args" size="md" variant="bordered" />
      </div>
    `,
  }),
};

/** Clicking emits a `click` event that the parent uses to open its search overlay. */
export const ClickToSearch: Story = {
  args: { variant: "bordered", size: "md", onClick: fn() },
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    const button = canvas.getByRole("button");
    await expect(button).toHaveAccessibleName("Search (Ctrl+F)");
    await userEvent.click(button);
    await expect(args.onClick).toHaveBeenCalledTimes(1);
  },
};
