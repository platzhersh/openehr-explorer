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
          "Static inline SVG used in place of a literal \"+\" text glyph on the app's 'create new' buttons (Upload Template, New EHR, New Composition, Add Server, ...). Renders as a filled circle badge with the plus mark cut out of it. No props — the circle fill inherits `color` via `currentColor`, and the plus mark is a genuine hole cut out of the circle via an SVG mask, so it always reveals whatever is actually behind the icon (button background, hover or not) instead of a hardcoded color.",
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

/**
 * `.btn-primary:hover` (shared-utilities.css) swaps both the button
 * background and its text/icon color to `var(--color-primary)` /
 * `var(--color-bg)` — which used to make the icon's circle and its
 * hardcoded "cut-out" color collapse to the same value, so the plus mark
 * disappeared into a solid dot. Reproduced here with an inline style
 * (rather than a real `:hover`, since there's no pseudo-state addon
 * installed) so the regression is visible without moving a mouse; the
 * mask-based cut-out keeps the plus legible in this state too.
 */
export const InButtonHovered: Story = {
  render: () => ({
    components: { PlusIcon },
    template: `
      <button
        class="btn btn-sm btn-primary"
        style="display: inline-flex; align-items: center; gap: 6px; background: var(--color-primary); color: var(--color-bg);"
      >
        <PlusIcon />
        New EHR
      </button>
    `,
  }),
};
