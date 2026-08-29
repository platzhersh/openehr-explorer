import type { Meta, StoryObj } from "@storybook/vue3-vite";

// See `medblocks-overrides.stories.ts` for the shared rationale. This file
// covers the field `mb-auto-form` renders for `DV_PROPORTION` (OEH-45): the
// real `<mb-proportion>` (unitary, e.g. a ratio) and `<mb-percent>`
// (percent) web components, per the medblocks-ui@0.0.217 bundle's
// `transformations.DV_PROPORTION` — both share the same `MbProportion`
// implementation and render a magnitude `<sl-input type="number">` next to
// an always-*disabled* `<sl-select>` used purely to display the unit
// suffix ("/ 1" or "%"). That disabled `<sl-select>` uses the theme's
// `--sl-input-*-disabled` tokens, a different contrast pairing than any
// other field's default (enabled) state, so it's worth its own check here.

interface ProportionArgs {
  label: string;
}

const meta: Meta<ProportionArgs> = {
  title: "Medblocks/Proportion",
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "The real `<mb-proportion>`/`<mb-percent>` medblocks-ui web components — what `mb-auto-form` renders for a `DV_PROPORTION` field — styled through `src/styles/medblocks-overrides.css`. Not backed by a project Vue component (see ADR-0002, ADR-0008).",
      },
    },
  },
  render: (args) => ({
    setup() {
      return { args };
    },
    template: `
      <div style="width: 360px; padding: 16px;">
        <mb-proportion :label="args.label"></mb-proportion>
      </div>
    `,
  }),
  args: {
    label: "Ejection fraction",
  },
};

export default meta;
type Story = StoryObj<ProportionArgs>;

export const Unitary: Story = {
  play: async ({ canvasElement }) => {
    const mbProportion = canvasElement.querySelector("mb-proportion") as any;
    await customElements.whenDefined("mb-proportion");
    await mbProportion?.updateComplete;
    mbProportion.data = { numerator: 0.55, denominator: 1, type: 1 };
  },
};

export const Percent: Story = {
  args: {
    label: "Oxygen saturation",
  },
  render: (args) => ({
    setup() {
      return { args };
    },
    template: `
      <div style="width: 360px; padding: 16px;">
        <mb-percent :label="args.label"></mb-percent>
      </div>
    `,
  }),
  play: async ({ canvasElement }) => {
    const mbPercent = canvasElement.querySelector("mb-percent") as any;
    await customElements.whenDefined("mb-percent");
    await mbPercent?.updateComplete;
    mbPercent.data = { numerator: 97, denominator: 100, type: 2 };
  },
};
