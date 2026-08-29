import type { Meta, StoryObj } from "@storybook/vue3-vite";

// medblocks-ui (`mb-select` / `mb-option`) is a CDN-loaded Web Components
// library (ADR-0008), not a Vue component this repo owns — so unlike the
// rest of `src/components/`, there's no `Foo.vue` behind this story.
//
// It renders the real component `mb-auto-form` generates for a
// `DV_CODED_TEXT` node — e.g. the "Nutritional state finding" field from
// PR #166 — through the app's Shoelace theme override in this directory's
// `medblocks-overrides.css`, so a future change to that file can be
// visually checked here without running the full Tauri app and navigating
// to a composition form.
//
// `.storybook/preview-head.html` loads the same pinned medblocks-ui/Shoelace
// CDN tags as `index.html` (registering `<mb-select>`/`<mb-option>` and
// Shoelace's base theme), and `.storybook/preview.ts` imports the real
// `medblocks-overrides.css` — not a copy — so this story exercises exactly
// what the app ships.

interface CodedTextArgs {
  label: string;
  placeholder: string;
}

// Mirrors the "Nutritional state finding" field's local value set, per the
// dropdown-contrast bug this story exists to catch a regression of.
// `code` is the local openEHR code mb-option's `value` binds to (what
// mb-select actually filters/compares against internally); `label` is the
// human-readable text shown for it — same split as a real Web Template's
// `inputs[].list`.
const OPTIONS = [
  { code: "1", label: "Underweight", ordinal: 1 },
  { code: "2", label: "Normal weight", ordinal: 2 },
  { code: "3", label: "Overweight", ordinal: 3 },
  { code: "4", label: "Obese", ordinal: 4 },
];

const meta: Meta<CodedTextArgs> = {
  title: "Medblocks/Coded Text Dropdown",
  parameters: {
    docs: {
      description: {
        component:
          "The real `<mb-select>`/`<mb-option>` medblocks-ui web components — what `mb-auto-form` renders for a `DV_CODED_TEXT` field — styled through `src/styles/medblocks-overrides.css`. Not backed by a project Vue component (see ADR-0002, ADR-0008).",
      },
    },
  },
  render: (args) => ({
    setup() {
      return { args, options: OPTIONS };
    },
    template: `
      <div style="width: 360px; padding: 16px;">
        <mb-select :label="args.label" :placeholder="args.placeholder" hoist>
          <mb-option
            v-for="opt in options"
            :key="opt.code"
            :value="opt.code"
            :label="opt.label"
            :ordinal="opt.ordinal"
          />
        </mb-select>
      </div>
    `,
  }),
  args: {
    label: "Nutritional state finding",
    placeholder: "Select a value",
  },
};

export default meta;
type Story = StoryObj<CodedTextArgs>;

export const Default: Story = {};

export const Preselected: Story = {
  play: async ({ canvasElement }) => {
    const mbSelect = canvasElement.querySelector("mb-select") as any;
    await customElements.whenDefined("mb-select");
    await mbSelect?.updateComplete;
    mbSelect.data = { code: "3", value: "Overweight", terminology: "local" };
  },
};

// Forces the dropdown open so the option list — the thing that was actually
// broken (near-invisible text on a near-matching dark background, see
// PR #166) — is visible on load instead of requiring a manual click.
export const Open: Story = {
  play: async ({ canvasElement }) => {
    const mbSelect = canvasElement.querySelector("mb-select") as any;
    await customElements.whenDefined("mb-select");
    await mbSelect?.updateComplete;
    mbSelect.data = { code: "3", value: "Overweight", terminology: "local" };
    await mbSelect?.updateComplete;
    const slSelect = mbSelect?.shadowRoot?.querySelector("sl-select") as any;
    await slSelect?.updateComplete;
    slSelect?.dropdown?.show();
  },
};
