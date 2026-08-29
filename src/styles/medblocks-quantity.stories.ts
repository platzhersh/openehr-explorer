import type { Meta, StoryObj } from "@storybook/vue3-vite";

// See `medblocks-overrides.stories.ts` for the shared rationale. This file
// covers the field `mb-auto-form` renders for `DV_QUANTITY` (OEH-45): the
// real `<mb-quantity>`/`<mb-unit>` web components, per the medblocks-ui@
// 0.0.217 bundle's `transformations.DV_QUANTITY` — a magnitude `<sl-input>`
// paired with a unit `<sl-select>`/`<sl-menu-item>` dropdown, exactly the
// same Shoelace part tree the `mb-select` coded-text dropdown uses, which is
// what made it prone to the dropdown-contrast bug PR #166 fixed. `<mb-unit>`
// children are consumed as hidden configuration (like `<mb-option>` for
// `mb-select`), not rendered directly.

interface QuantityArgs {
  label: string;
}

// Mirrors a real Web Template's weight field: local units, a low/high range
// per unit for validation, same shape as `inputs[1].list` in the bundle's
// `DV_QUANTITY` transform.
const UNITS = [
  { unit: "kg", label: "kg", min: 0, max: 500 },
  { unit: "lb", label: "lb", min: 0, max: 1100 },
];

const meta: Meta<QuantityArgs> = {
  title: "Medblocks/Quantity",
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "The real `<mb-quantity>`/`<mb-unit>` medblocks-ui web components — what `mb-auto-form` renders for a `DV_QUANTITY` field — styled through `src/styles/medblocks-overrides.css`. Not backed by a project Vue component (see ADR-0002, ADR-0008).",
      },
    },
  },
  render: (args) => ({
    setup() {
      return { args, units: UNITS };
    },
    template: `
      <div style="width: 360px; padding: 16px;">
        <mb-quantity :label="args.label" hoist>
          <mb-unit
            v-for="u in units"
            :key="u.unit"
            :unit="u.unit"
            :label="u.label"
            :min="u.min"
            :max="u.max"
          />
        </mb-quantity>
      </div>
    `,
  }),
  args: {
    label: "Body weight",
  },
};

export default meta;
type Story = StoryObj<QuantityArgs>;

export const Empty: Story = {};

export const Populated: Story = {
  play: async ({ canvasElement }) => {
    const mbQuantity = canvasElement.querySelector("mb-quantity") as any;
    await customElements.whenDefined("mb-quantity");
    await mbQuantity?.updateComplete;
    mbQuantity.data = { magnitude: 72.5, unit: "kg" };
  },
};

// Forces the unit dropdown open — the `sl-select`/`sl-menu-item` part tree
// most likely to regress the same way the coded-text dropdown did.
export const Open: Story = {
  play: async ({ canvasElement }) => {
    const mbQuantity = canvasElement.querySelector("mb-quantity") as any;
    await customElements.whenDefined("mb-quantity");
    await mbQuantity?.updateComplete;
    mbQuantity.data = { magnitude: 72.5, unit: "kg" };
    await mbQuantity?.updateComplete;
    const slSelect = mbQuantity?.shadowRoot?.querySelector("sl-select") as any;
    await slSelect?.updateComplete;
    slSelect?.dropdown?.show();
  },
};
