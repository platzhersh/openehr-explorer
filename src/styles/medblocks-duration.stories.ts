import type { Meta, StoryObj } from "@storybook/vue3-vite";

// See `medblocks-overrides.stories.ts` for the shared rationale. This file
// covers the field `mb-auto-form` renders for `DV_DURATION` (OEH-45): the
// real `<mb-duration>` web component, per the medblocks-ui@0.0.217 bundle's
// `transformations.DV_DURATION` — one Shoelace `<sl-input type="number">`
// per enabled unit (`year`/`month`/`week`/`day`/`hour`/`minute`/`second`
// boolean attributes, taken from the Web Template's `inputs[].suffix`
// list), accepting/emitting an ISO 8601 duration string (e.g. `"P3DT4H"`)
// through its `data` property.

interface DurationArgs {
  label: string;
}

const meta: Meta<DurationArgs> = {
  title: "Medblocks/Duration",
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "The real `<mb-duration>` medblocks-ui web component — what `mb-auto-form` renders for a `DV_DURATION` field — styled through `src/styles/medblocks-overrides.css`. Not backed by a project Vue component (see ADR-0002, ADR-0008).",
      },
    },
  },
  render: (args) => ({
    setup() {
      return { args };
    },
    // A typical clinical duration template restricts inputs to day/hour/
    // minute (e.g. "time since symptom onset"), same as a real Web
    // Template's `inputs[].suffix` list would.
    template: `
      <div style="width: 480px; padding: 16px;">
        <mb-duration day hour minute :label="args.label"></mb-duration>
      </div>
    `,
  }),
  args: {
    label: "Duration of symptoms",
  },
};

export default meta;
type Story = StoryObj<DurationArgs>;

export const Empty: Story = {};

export const Populated: Story = {
  play: async ({ canvasElement }) => {
    const mbDuration = canvasElement.querySelector("mb-duration") as any;
    await customElements.whenDefined("mb-duration");
    await mbDuration?.updateComplete;
    mbDuration.data = "P3DT4H30M";
  },
};
