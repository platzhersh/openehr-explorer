import type { Meta, StoryObj } from "@storybook/vue3-vite";
import PlusIcon from "./PlusIcon.vue";

const meta: Meta<typeof PlusIcon> = {
  title: "Components/PlusIcon",
  component: PlusIcon,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Static inline SVG used in place of a literal \"+\" text glyph on the app's 'create new' buttons (Upload Template, New EHR, New Composition, Add Server, ...). No props — inherits `color` via `currentColor`, so it's typically shown inside a button.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof PlusIcon>;

export const Default: Story = {};

/** How it's actually used: inline inside a themed button, at its native 14x14 size. */
export const InButton: Story = {
  render: () => ({
    components: { PlusIcon },
    template: `
      <button class="btn btn-sm btn-primary" style="display: inline-flex; align-items: center; gap: 6px;">
        <PlusIcon />
        New EHR
      </button>
    `,
  }),
};
