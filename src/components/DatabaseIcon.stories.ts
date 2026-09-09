import type { Meta, StoryObj } from "@storybook/vue3-vite";
import DatabaseIcon from "./DatabaseIcon.vue";

const meta: Meta<typeof DatabaseIcon> = {
  title: "Components/DatabaseIcon",
  component: DatabaseIcon,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Static inline SVG used for the Dashboard's 'EHRs' stat card (OEH-57). A cylinder/database glyph in the app's thin-stroke line-icon style — no props, inherits color via `currentColor`.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof DatabaseIcon>;

export const Default: Story = {};

/** How it's actually used: inside the tinted rounded-square badge on a Dashboard stat card. */
export const InBadge: Story = {
  render: () => ({
    components: { DatabaseIcon },
    template: `
      <div style="display: inline-flex; align-items: center; justify-content: center; width: 36px; height: 36px; border-radius: 8px; background: rgba(100, 255, 218, 0.12); color: var(--color-primary);">
        <DatabaseIcon />
      </div>
    `,
  }),
};
