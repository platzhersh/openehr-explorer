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
          "Static inline SVG used in place of a literal \"+\" text glyph on the app's 'create new' buttons (Upload Template, New EHR, New Composition, Add Server, ...). Renders as a filled circle badge with the plus mark cut out of it. No props — the circle fill inherits `color` via `currentColor`, and the cut-out plus mark uses the `--color-bg` CSS variable (falling back to the app's dark background) for contrast, so it's typically shown inside a button.",
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
