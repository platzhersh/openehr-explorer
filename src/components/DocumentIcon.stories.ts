import type { Meta, StoryObj } from "@storybook/vue3-vite";
import DocumentIcon from "./DocumentIcon.vue";

const meta: Meta<typeof DocumentIcon> = {
  title: "Components/DocumentIcon",
  component: DocumentIcon,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Static inline SVG used for the Dashboard's 'Compositions' stat card (OEH-57). A folded-corner document glyph in the app's thin-stroke line-icon style — no props, inherits color via `currentColor`.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof DocumentIcon>;

export const Default: Story = {};

/** How it's actually used: inside the tinted rounded-square badge on a Dashboard stat card. */
export const InBadge: Story = {
  render: () => ({
    components: { DocumentIcon },
    template: `
      <div style="display: inline-flex; align-items: center; justify-content: center; width: 36px; height: 36px; border-radius: 8px; background: rgba(100, 255, 218, 0.12); color: var(--color-primary);">
        <DocumentIcon />
      </div>
    `,
  }),
};
