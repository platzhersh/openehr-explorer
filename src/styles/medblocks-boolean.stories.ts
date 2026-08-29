import type { Meta, StoryObj } from "@storybook/vue3-vite";

// See `medblocks-overrides.stories.ts` for the shared rationale. This file
// covers the field `mb-auto-form` renders for `DV_BOOLEAN` (OEH-45): the
// real `<mb-checkbox>` web component, a single Shoelace `<sl-checkbox>`,
// per the medblocks-ui@0.0.217 bundle's `transformations.DV_BOOLEAN`.
//
// `mb-checkbox` reflects `data === null`/`undefined` (an unanswered field,
// which is the initial state `mb-auto-form` renders for every optional
// boolean node) as `?indeterminate=${this.data == null}` — a third visual
// state distinct from checked/unchecked that's easy to skip when eyeballing
// contrast, so it gets its own story here.

interface BooleanArgs {
  label: string;
}

const meta: Meta<BooleanArgs> = {
  title: "Medblocks/Boolean",
  parameters: {
    docs: {
      description: {
        component:
          "The real `<mb-checkbox>` medblocks-ui web component — what `mb-auto-form` renders for a `DV_BOOLEAN` field — styled through `src/styles/medblocks-overrides.css`. Not backed by a project Vue component (see ADR-0002, ADR-0008).",
      },
    },
  },
  render: (args) => ({
    setup() {
      return { args };
    },
    template: `
      <div style="width: 360px; padding: 16px;">
        <mb-checkbox :label="args.label"></mb-checkbox>
      </div>
    `,
  }),
  args: {
    label: "Patient consented",
  },
};

export default meta;
type Story = StoryObj<BooleanArgs>;

// The default state: `data` is `undefined`, so `mb-checkbox` renders
// indeterminate rather than unchecked.
export const Indeterminate: Story = {};

export const Checked: Story = {
  play: async ({ canvasElement }) => {
    const mbCheckbox = canvasElement.querySelector("mb-checkbox") as any;
    await customElements.whenDefined("mb-checkbox");
    await mbCheckbox?.updateComplete;
    mbCheckbox.data = true;
  },
};

export const Unchecked: Story = {
  play: async ({ canvasElement }) => {
    const mbCheckbox = canvasElement.querySelector("mb-checkbox") as any;
    await customElements.whenDefined("mb-checkbox");
    await mbCheckbox?.updateComplete;
    mbCheckbox.data = false;
  },
};
