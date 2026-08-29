import type { Meta, StoryObj } from "@storybook/vue3-vite";

// See `medblocks-overrides.stories.ts` for the shared rationale. This file
// covers the field `mb-auto-form` renders for `DV_DATE` and `DV_DATE_TIME`
// (OEH-45): the real `<mb-date>` web component, a single Shoelace
// `<sl-input type="date">` (or `type="datetime-local"` when the `time`
// attribute is set for `DV_DATE_TIME`), per the medblocks-ui@0.0.217
// bundle's `transformations.DV_DATE` / `DV_DATE_TIME`.

interface DateArgs {
  label: string;
}

const meta: Meta<DateArgs> = {
  title: "Medblocks/Date",
  parameters: {
    docs: {
      description: {
        component:
          "The real `<mb-date>` medblocks-ui web component — what `mb-auto-form` renders for `DV_DATE`/`DV_DATE_TIME` fields — styled through `src/styles/medblocks-overrides.css`. Not backed by a project Vue component (see ADR-0002, ADR-0008).",
      },
    },
  },
  render: (args) => ({
    setup() {
      return { args };
    },
    template: `
      <div style="width: 360px; padding: 16px;">
        <mb-date :label="args.label"></mb-date>
      </div>
    `,
  }),
  args: {
    label: "Date of onset",
  },
};

export default meta;
type Story = StoryObj<DateArgs>;

export const Empty: Story = {};

export const Populated: Story = {
  play: async ({ canvasElement }) => {
    const mbDate = canvasElement.querySelector("mb-date") as any;
    await customElements.whenDefined("mb-date");
    await mbDate?.updateComplete;
    mbDate.data = "2026-08-29";
  },
};

// `mb-auto-form` renders `DV_DATE_TIME` as the same element with the `time`
// attribute set, switching the internal `<sl-input>` to
// `type="datetime-local"`.
export const DateTime: Story = {
  args: {
    label: "Time of assessment",
  },
  render: (args) => ({
    setup() {
      return { args };
    },
    template: `
      <div style="width: 360px; padding: 16px;">
        <mb-date time :label="args.label"></mb-date>
      </div>
    `,
  }),
  play: async ({ canvasElement }) => {
    const mbDate = canvasElement.querySelector("mb-date") as any;
    await customElements.whenDefined("mb-date");
    await mbDate?.updateComplete;
    mbDate.data = "2026-08-29T14:30:00.000Z";
  },
};
