import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import EhrFilterModal from "./EhrFilterModal.vue";
import type { EhrSearchCriteria } from "../stores/ehr";

const meta: Meta<typeof EhrFilterModal> = {
  title: "Components/EhrFilterModal",
  component: EhrFilterModal,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Structured, form-based builder for EHR search criteria (see EhrBrowser.vue's Filters button). Replaces hand-typing the colon-syntax query string with text inputs and tri-state selects, and seeds itself from whatever criteria are already active so reopening it reflects the current filters. The colon-syntax reference for the quick search box lives here too, folded away behind a 'Show shortcut syntax' toggle.",
      },
    },
  },
  args: {
    open: true,
    criteria: {},
    onClose: fn(),
    onApply: fn(),
  },
  render: (args) => ({
    components: { EhrFilterModal },
    setup() {
      return { args };
    },
    template: `<EhrFilterModal :open="args.open" :criteria="args.criteria" @close="args.onClose" @apply="args.onApply" />`,
  }),
};

export default meta;
type Story = StoryObj<typeof EhrFilterModal>;

/** No filters applied yet — every field starts empty/"Any". */
export const Empty: Story = {};

/** Reopened with filters already active (e.g. from removable chips or a prior
 *  search) — the form seeds itself from `criteria` so editing continues
 *  from where the list currently stands. */
export const Prefilled: Story = {
  args: {
    criteria: {
      subject_id: "6f4b5848",
      modifiable: true,
      has_directory: true,
    } satisfies EhrSearchCriteria,
  },
};

/** Clicking "Show shortcut syntax" reveals the colon-syntax reference table
 *  for anyone who'd rather type filters into the quick search box. */
export const ShortcutSyntaxExpanded: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const toggle = canvas.getByRole("button", { name: /show shortcut syntax/i });
    await userEvent.click(toggle);
    await expect(canvas.getByText("hasDirectory:true|false")).toBeVisible();
  },
};
