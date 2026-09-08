import type { Meta, StoryObj } from "@storybook/vue3-vite";
import LockIcon from "./LockIcon.vue";

const meta: Meta<typeof LockIcon> = {
  title: "Components/LockIcon",
  component: LockIcon,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Static inline SVG padlock, used in place of the 🔒 emoji on the app's 'stored securely' markers — the credential-backend badge on a server profile card and the 'stored securely' hints in the server form. No props: it's stroke-drawn in `currentColor`, so it picks up the green (or muted) tone of whatever badge or hint it sits in, which the emoji's fixed gold-and-grey glyph could not.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof LockIcon>;

export const Default: Story = {};

/** How it's actually used: inside the server card's credential-backend badge. */
export const InBadge: Story = {
  render: () => ({
    components: { LockIcon },
    template: `
      <span
        class="badge"
        style="gap: 5px; background: rgba(34, 197, 94, 0.1); color: #22c55e; font-weight: 600; border: 1px solid rgba(34, 197, 94, 0.3);"
      >
        <LockIcon />
        OS Keychain
      </span>
    `,
  }),
};

/** And inline in a form hint, where it inherits the hint's green. */
export const InHint: Story = {
  render: () => ({
    components: { LockIcon },
    template: `
      <p style="display: flex; align-items: center; gap: 5px; margin: 0; font-size: 12px; color: #22c55e;">
        <LockIcon />
        Password is stored securely. Leave empty to keep the existing password.
      </p>
    `,
  }),
};
