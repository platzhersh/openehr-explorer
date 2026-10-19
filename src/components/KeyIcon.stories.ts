import type { Meta, StoryObj } from "@storybook/vue3-vite";
import KeyIcon from "./KeyIcon.vue";

const meta: Meta<typeof KeyIcon> = {
  title: "Components/KeyIcon",
  component: KeyIcon,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Static inline SVG key, shown in the auth-method badge on a server profile card. No props: it's stroke-drawn in `currentColor`, so it takes the badge's text color.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof KeyIcon>;

export const Default: Story = {};

/** How it's used: inside the server card's auth-method badge. */
export const InBadge: Story = {
  render: () => ({
    components: { KeyIcon },
    template: `
      <span class="badge" style="gap: 5px;">
        <KeyIcon />
        basic
      </span>
    `,
  }),
};
