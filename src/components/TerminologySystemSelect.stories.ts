import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { ref } from "vue";
import { expect, userEvent, within } from "storybook/test";
import TerminologySystemSelect from "./TerminologySystemSelect.vue";

const meta: Meta<typeof TerminologySystemSelect> = {
  title: "Components/TerminologySystemSelect",
  component: TerminologySystemSelect,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Terminology system picker used across the Terminology Browser's tabs. Shows a visible dropdown of the common systems the backend recognises by name (SNOMED CT, LOINC, ICD-10, ICD-11, ATC), plus a 'Custom…' option that reveals a free-text field for anything else — a raw canonical system URI, a national extension, or an identifier the dropdown doesn't know about. Replaces a plain `<input list>` datalist, which gives no visible affordance that suggestions exist.",
      },
    },
  },
  render: (args) => ({
    components: { TerminologySystemSelect },
    setup() {
      const value = ref(args.modelValue);
      return { value, label: args.label };
    },
    template: `<TerminologySystemSelect v-model="value" :label="label" />`,
  }),
  args: {
    modelValue: "SNOMED-CT",
    label: "Terminology system",
  },
};

export default meta;
type Story = StoryObj<typeof TerminologySystemSelect>;

/** A known system is selected in the dropdown; no custom field is shown. */
export const KnownSystem: Story = {
  args: { modelValue: "LOINC" },
};

/** A value outside the common list (a raw canonical URI, in this case) shows as "Custom…" with the text field revealed. */
export const CustomSystem: Story = {
  args: { modelValue: "http://hl7.org/fhir/sid/ndc" },
};

/** Picking "Custom…" from the dropdown reveals an empty text field to type into. */
export const SwitchToCustom: Story = {
  args: { modelValue: "SNOMED-CT" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const select = canvas.getByRole("combobox");
    await userEvent.selectOptions(select, "Custom…");
    await expect(canvas.getByPlaceholderText(/canonical system uri/i)).toBeInTheDocument();
  },
};
