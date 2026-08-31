import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import DirectoryAddItemModal from "./DirectoryAddItemModal.vue";

const AVAILABLE_COMPOSITIONS = [
  {
    uid: "8f3c1e2a-1234-4a5b-8c9d-1234567890ab::ferroehr.local::1",
    label: "Vital Signs Encounter",
  },
  { uid: "a1b2c3d4-5678-4a5b-8c9d-0987654321ba::ferroehr.local::1", label: "Lab Report" },
];

const meta: Meta<typeof DirectoryAddItemModal> = {
  title: "Components/DirectoryAddItemModal",
  component: DirectoryAddItemModal,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Richer 'add item reference' picker for the DIRECTORY tree editor (see DirectoryTreeEditor.vue). Offers a scannable list of this EHR's own compositions (click a row to add it as a COMPOSITION/local/HIER_OBJECT_ID reference) plus a manual-entry fallback for any other OBJECT_REF — a composition in a different EHR, or a non-COMPOSITION reference entirely.",
      },
    },
  },
  args: {
    open: true,
    availableCompositions: AVAILABLE_COMPOSITIONS,
    onClose: fn(),
    onAdd: fn(),
  },
  render: (args) => ({
    components: { DirectoryAddItemModal },
    setup() {
      return { args };
    },
    template: `<DirectoryAddItemModal :open="args.open" :available-compositions="args.availableCompositions" @close="args.onClose" @add="args.onAdd" />`,
  }),
};

export default meta;
type Story = StoryObj<typeof DirectoryAddItemModal>;

/** Both entry points available: a clickable list of this EHR's compositions,
 *  and the manual namespace/type/id-scheme/id-value fields below it. */
export const Default: Story = {};

/** No compositions exist for this EHR yet — the list collapses to a hint,
 *  leaving manual entry as the only way to add a reference. */
export const NoCompositions: Story = {
  args: {
    availableCompositions: [],
  },
};

/** Clicking a composition row emits `add` immediately, with COMPOSITION /
 *  local / HIER_OBJECT_ID filled in, then closes the modal (`close`) —
 *  adding a second reference means reopening via "+ Item reference". */
export const ClickingACompositionAddsIt: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    const row = canvas.getByTitle(AVAILABLE_COMPOSITIONS[0].uid);
    await userEvent.click(row);
    await expect(args.onAdd).toHaveBeenCalledWith({
      id: AVAILABLE_COMPOSITIONS[0].uid,
      type: "COMPOSITION",
      namespace: "local",
      idScheme: "HIER_OBJECT_ID",
    });
    await expect(args.onClose).toHaveBeenCalled();
  },
};

/** Manual entry requires an id value — submitting without one shows an
 *  inline error instead of emitting `add`, and leaves the modal open. */
export const ManualEntryRequiresAnIdValue: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: /add reference/i }));
    await expect(canvas.getByText(/enter an id value/i)).toBeVisible();
    await expect(args.onAdd).not.toHaveBeenCalled();
    await expect(args.onClose).not.toHaveBeenCalled();
  },
};

/** A fully filled-in manual reference emits `add` with the entered fields,
 *  for a reference the composition list can't express (e.g. a different
 *  EHR's composition, or a non-COMPOSITION object) — then closes the modal. */
export const ManualEntrySubmits: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.type(canvas.getByLabelText(/namespace/i), "issuer.example.org");
    await userEvent.clear(canvas.getByLabelText(/^type$/i));
    await userEvent.type(canvas.getByLabelText(/^type$/i), "PERSON");
    await userEvent.clear(canvas.getByLabelText(/id scheme/i));
    await userEvent.type(canvas.getByLabelText(/id scheme/i), "GENERIC_ID");
    await userEvent.type(canvas.getByLabelText(/id value/i), "external-ref-1");
    await userEvent.click(canvas.getByRole("button", { name: /add reference/i }));
    await expect(args.onAdd).toHaveBeenCalledWith({
      id: "external-ref-1",
      type: "PERSON",
      namespace: "issuer.example.org",
      idScheme: "GENERIC_ID",
    });
    await expect(args.onClose).toHaveBeenCalled();
  },
};
