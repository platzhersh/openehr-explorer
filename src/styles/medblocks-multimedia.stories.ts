import type { Meta, StoryObj } from "@storybook/vue3-vite";

// See `medblocks-overrides.stories.ts` for the shared rationale. This file
// covers the field `mb-auto-form` renders for `DV_MULTIMEDIA` (OEH-45): the
// real `<mb-multimedia>` web component, per the medblocks-ui@0.0.217
// bundle's `transformations.DV_MULTIMEDIA`.
//
// Per ADR-0002, `DV_MULTIMEDIA` is one of the rmTypes the app treats as a
// read-only fallback: `CompositionForm.vue` doesn't wire up `mb-multimedia`'s
// upload plugin (`storageAPI`/`parentAxiosKey`), so in the real app this
// element only ever displays whatever multimedia value already exists on a
// composition being viewed/edited, never a working upload control. Unlike
// every other field in this OEH-45 batch, `mb-multimedia` renders a plain
// native `<input type="file">` and `<img>` preview rather than a Shoelace
// part tree — `medblocks-overrides.css` has no rule that touches it
// specifically, but it still inherits the shared `mb-auto-form` font/size
// rule and sits in the same form, so a story here catches it being
// unreadable against the app's dark background.
//
// A small embedded SVG data URI stands in for a real openEHR
// `DV_MULTIMEDIA` payload (no network fetch, no CDN dependency), only to
// exercise the `<img>` preview path.
const SAMPLE_IMAGE =
  "data:image/svg+xml;base64," +
  btoa(
    '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="120"><rect width="200" height="120" fill="#0d9488"/><text x="100" y="65" font-family="sans-serif" font-size="16" fill="#f0fdfa" text-anchor="middle">ECG scan</text></svg>',
  );

interface MultimediaArgs {
  label: string;
}

const meta: Meta<MultimediaArgs> = {
  title: "Medblocks/Multimedia",
  parameters: {
    docs: {
      description: {
        component:
          "The real `<mb-multimedia>` medblocks-ui web component — what `mb-auto-form` renders for a `DV_MULTIMEDIA` field — through the app's `medblocks-overrides.css` (a read-only fallback field per ADR-0002; not backed by a project Vue component, see ADR-0008).",
      },
    },
  },
  render: (args) => ({
    setup() {
      return { args };
    },
    template: `
      <div style="width: 360px; padding: 16px;">
        <mb-multimedia :label="args.label"></mb-multimedia>
      </div>
    `,
  }),
  args: {
    label: "Attached scan",
  },
};

export default meta;
type Story = StoryObj<MultimediaArgs>;

export const Empty: Story = {};

export const WithPreview: Story = {
  render: (args) => ({
    setup() {
      return { args, src: SAMPLE_IMAGE };
    },
    template: `
      <div style="width: 360px; padding: 16px;">
        <mb-multimedia :label="args.label" :src="src"></mb-multimedia>
      </div>
    `,
  }),
};

// `mb-repeatable-headless`/`mb-repeatable-simple` (registered by the bundle
// for `:N`-indexed repeatable node groups) are deliberately not given a
// story here. Checking the bundle's own `createRepeatableElement()`/
// `traverse()` logic: `mb-auto-form` only ever instantiates
// `mb-repeatable-headless`, whose own styles are `:host { display: none }`
// — it's a state controller with no visual output of its own. The "+"/
// "Delete" buttons a repeatable group actually shows are plain
// `document.createElement("sl-button")` elements the bundle wires up
// directly (`createRepeatableButtons()` in its autoform-utils), not
// anything `mb-repeatable-headless` renders — and `medblocks-overrides.css`
// has no rule targeting them beyond Shoelace's own button theming, which
// isn't specific to repeatable groups. There is no field-shaped component
// here for a story to render.
