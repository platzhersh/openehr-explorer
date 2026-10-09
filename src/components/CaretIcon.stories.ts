import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { expect, within } from "storybook/test";
import CaretIcon from "./CaretIcon.vue";

const meta: Meta<typeof CaretIcon> = {
  title: "Components/CaretIcon",
  component: CaretIcon,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Shared expand/collapse caret (OEH-107). `right` = collapsed, `down` = expanded; it rotates between them, draws in `currentColor`, and is decorative (`aria-hidden`) — the toggle that contains it must expose `aria-expanded`.",
      },
    },
  },
  args: { direction: "right", size: 14 },
};

export default meta;
type Story = StoryObj<typeof CaretIcon>;

export const Collapsed: Story = { args: { direction: "right" } };

export const Expanded: Story = { args: { direction: "down" } };

/** Sizes used in the app: 12 for tree rows and selects, 14 for disclosure sections. */
export const Sizes: Story = {
  render: () => ({
    components: { CaretIcon },
    template: `
      <div style="display:flex; gap:16px; align-items:center; color: var(--color-text-secondary)">
        <CaretIcon :size="10" /><CaretIcon :size="12" /><CaretIcon :size="14" /><CaretIcon :size="18" />
        <CaretIcon direction="down" :size="10" /><CaretIcon direction="down" :size="12" />
        <CaretIcon direction="down" :size="14" /><CaretIcon direction="down" :size="18" />
      </div>`,
  }),
};

export const IsDecorative: Story = {
  render: (args) => ({
    components: { CaretIcon },
    setup: () => ({ args }),
    template: `<div data-testid="host"><CaretIcon v-bind="args" /></div>`,
  }),
  play: async ({ canvasElement }) => {
    const svg = within(canvasElement).getByTestId("host").querySelector("svg");
    await expect(svg).toHaveAttribute("aria-hidden", "true");
  },
};
