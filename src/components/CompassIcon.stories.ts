import type { Meta, StoryObj } from "@storybook/vue3-vite";
import CompassIcon from "./CompassIcon.vue";

const meta: Meta<typeof CompassIcon> = {
  title: "Components/CompassIcon",
  component: CompassIcon,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Static inline SVG for the 'take a tour' trigger (see PRD-0018). No props — inherits `color` via `currentColor`, so it's typically shown inside a button.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof CompassIcon>;

export const Default: Story = {};

/** How it's actually used: inline inside a themed button, at its native 14x14 size. */
export const InButton: Story = {
  render: () => ({
    components: { CompassIcon },
    template: `
      <button class="btn" style="display: inline-flex; align-items: center; gap: 6px;">
        <CompassIcon />
        Take a tour
      </button>
    `,
  }),
};
