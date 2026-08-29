import type { Meta, StoryObj } from "@storybook/vue3-vite";

// See `medblocks-overrides.stories.ts` for the shared rationale (no `Foo.vue`
// behind this file, CDN wiring, why this lives in `src/styles/`) — this file
// covers the field `mb-auto-form` renders for a plain `DV_TEXT` node: the
// real `<mb-input>` web component (OEH-45), styled through the app's
// `medblocks-overrides.css`.
//
// Per the medblocks-ui@0.0.217 bundle's `transformations.DV_TEXT`, a
// single-occurrence text field with no value-set annotation renders as
// `<mb-input path label>` — a Shoelace `<sl-input>` internally — and the
// same element switches to a `<sl-textarea>` when the `textarea` attribute
// is set (used for the app's `CompositionForm.vue` `TEXT_AREA` path config).
// Both are exercised here since both are real production output, not just
// the default.

interface TextInputArgs {
  label: string;
  placeholder: string;
}

const meta: Meta<TextInputArgs> = {
  title: "Medblocks/Text Input",
  parameters: {
    docs: {
      description: {
        component:
          "The real `<mb-input>` medblocks-ui web component — what `mb-auto-form` renders for a `DV_TEXT` field — styled through `src/styles/medblocks-overrides.css`. Not backed by a project Vue component (see ADR-0002, ADR-0008).",
      },
    },
  },
  render: (args) => ({
    setup() {
      return { args };
    },
    template: `
      <div style="width: 360px; padding: 16px;">
        <mb-input :label="args.label" :placeholder="args.placeholder"></mb-input>
      </div>
    `,
  }),
  args: {
    label: "Chief complaint",
    placeholder: "Enter free text",
  },
};

export default meta;
type Story = StoryObj<TextInputArgs>;

export const Empty: Story = {};

export const Populated: Story = {
  play: async ({ canvasElement }) => {
    const mbInput = canvasElement.querySelector("mb-input") as any;
    await customElements.whenDefined("mb-input");
    await mbInput?.updateComplete;
    mbInput.data = "Shortness of breath on exertion";
  },
};

// `mb-auto-form` sets the `textarea` attribute for paths configured under
// `CompositionForm.vue`'s `TEXT_AREA` list, switching the internal element
// from `<sl-input>` to `<sl-textarea>` — a different Shoelace part tree
// that needs its own contrast check.
export const Textarea: Story = {
  args: {
    label: "Clinical notes",
    placeholder: "Enter free text",
  },
  render: (args) => ({
    setup() {
      return { args };
    },
    template: `
      <div style="width: 360px; padding: 16px;">
        <mb-input textarea :label="args.label" :placeholder="args.placeholder"></mb-input>
      </div>
    `,
  }),
  play: async ({ canvasElement }) => {
    const mbInput = canvasElement.querySelector("mb-input") as any;
    await customElements.whenDefined("mb-input");
    await mbInput?.updateComplete;
    mbInput.data =
      "Patient reports gradual onset of symptoms over the past three days, denies fever or chest pain.";
  },
};
