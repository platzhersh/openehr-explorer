import type { Meta, StoryObj } from "@storybook/vue3-vite";
import AqlEditor from "./AqlEditor.vue";
import type { AqlPathEntry } from "../lib/aql/aqlPathIndex";

const meta: Meta<typeof AqlEditor> = {
  title: "Components/AqlEditor",
  component: AqlEditor,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "CodeMirror 6-based AQL editor (see ADR-0012). Syntax highlighting, keyword/RM-path/template-path autocomplete, Ctrl/Cmd+Enter to execute, Shift+Alt+F to format.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof AqlEditor>;

const SAMPLE_QUERY = `SELECT
    c/uid/value as uid,
    o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude as systolic
FROM EHR e
CONTAINS COMPOSITION c
CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2]
WHERE e/ehr_id/value = '7d44b88c-4199-4bad-97dc-d78268e01398'`;

// v-model is wired back to args (mutated from the template, not typed TS —
// Storybook's own args type marks props readonly) so typing/formatting in
// the Storybook canvas behaves like it would with a real parent (e.g.
// AqlRunner.vue).
const render: NonNullable<Story["render"]> = (args) => ({
  components: { AqlEditor },
  setup() {
    return { args };
  },
  template: `
    <div style="height: 260px; border: 1px solid var(--color-border); border-radius: var(--radius);">
      <AqlEditor v-bind="args" @update:modelValue="args.modelValue = $event" />
    </div>
  `,
});

export const Default: Story = {
  render,
  args: {
    modelValue: SAMPLE_QUERY,
  },
};

export const Empty: Story = {
  render,
  args: {
    modelValue: "",
  },
};

/** Template-aware paths feed the 3rd autocomplete tier — try typing `o/` in the query above. */
export const WithTemplatePaths: Story = {
  render,
  args: {
    modelValue: "SELECT o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value\nFROM ",
    allTemplatePaths: [
      {
        aqlPath: "/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude",
        label: "Systolic",
        rmType: "DV_QUANTITY",
      },
      {
        aqlPath: "/data[at0001]/events[at0006]/data[at0003]/items[at0005]/value/magnitude",
        label: "Diastolic",
        rmType: "DV_QUANTITY",
      },
    ] satisfies AqlPathEntry[],
  },
};
