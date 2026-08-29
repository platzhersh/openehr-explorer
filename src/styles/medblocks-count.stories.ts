import type { Meta, StoryObj } from "@storybook/vue3-vite";

// See `medblocks-overrides.stories.ts` for the shared rationale. This file
// covers the field `mb-auto-form` renders for `DV_COUNT` (OEH-45): the real
// `<mb-count>` web component, a single Shoelace `<sl-input type="number">`,
// per the medblocks-ui@0.0.217 bundle's `transformations.DV_COUNT`.

interface CountArgs {
  label: string;
}

const meta: Meta<CountArgs> = {
  title: "Medblocks/Count",
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "The real `<mb-count>` medblocks-ui web component — what `mb-auto-form` renders for a `DV_COUNT` field — styled through `src/styles/medblocks-overrides.css`. Not backed by a project Vue component (see ADR-0002, ADR-0008).",
      },
    },
  },
  render: (args) => ({
    setup() {
      return { args };
    },
    template: `
      <div style="width: 360px; padding: 16px;">
        <mb-count :label="args.label" min="0"></mb-count>
      </div>
    `,
  }),
  args: {
    label: "Number of episodes",
  },
};

export default meta;
type Story = StoryObj<CountArgs>;

export const Empty: Story = {};

export const Populated: Story = {
  play: async ({ canvasElement }) => {
    const mbCount = canvasElement.querySelector("mb-count") as any;
    await customElements.whenDefined("mb-count");
    await mbCount?.updateComplete;
    mbCount.data = 3;
  },
};
