import type { Meta, StoryObj } from "@storybook/vue3-vite";
import CompositionTree from "./CompositionTree.vue";

const meta: Meta<typeof CompositionTree> = {
  title: "Components/CompositionTree",
  component: CompositionTree,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Walks a raw composition JSON structure (canonical FHIR-style openEHR RM JSON) into a collapsible tree, resolving labels from a Web Template when provided. The one Tauri call it can make (`lookup_code`, for external terminology hover lookups) degrades gracefully to no-op outside a Tauri runtime, so it renders safely here.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof CompositionTree>;

// A trimmed but realistic openEHR-shaped composition fragment: an
// OBSERVATION with a coded finding (DV_CODED_TEXT, external terminology)
// and two DV_QUANTITY values.
const SAMPLE_DATA = {
  _type: "OBSERVATION",
  archetype_node_id: "openEHR-EHR-OBSERVATION.blood_pressure.v2",
  name: { value: "Blood pressure" },
  data: {
    _type: "HISTORY",
    origin: { value: "2026-08-27T09:00:00Z" },
    events: [
      {
        _type: "POINT_EVENT",
        archetype_node_id: "at0006",
        name: { value: "Any event" },
        data: {
          _type: "ITEM_TREE",
          items: [
            {
              _type: "ELEMENT",
              archetype_node_id: "at0004",
              name: { value: "Systolic" },
              value: { _type: "DV_QUANTITY", magnitude: 120, units: "mm[Hg]" },
            },
            {
              _type: "ELEMENT",
              archetype_node_id: "at0005",
              name: { value: "Diastolic" },
              value: { _type: "DV_QUANTITY", magnitude: 80, units: "mm[Hg]" },
            },
            {
              _type: "ELEMENT",
              archetype_node_id: "at1000",
              name: { value: "Position" },
              value: {
                _type: "DV_CODED_TEXT",
                value: "Standing",
                defining_code: {
                  terminology_id: { value: "SNOMED-CT" },
                  code_string: "10904000",
                },
              },
            },
          ],
        },
      },
    ],
  },
};

export const Default: Story = {
  args: {
    data: SAMPLE_DATA,
    webTemplate: null,
  },
};

export const WithHighlightedPath: Story = {
  args: {
    data: SAMPLE_DATA,
    webTemplate: null,
    highlightedPath: "/data/events[0]/data/items[0]",
  },
};

export const FilteredBySearch: Story = {
  args: {
    data: SAMPLE_DATA,
    webTemplate: null,
    searchQuery: "diastolic",
  },
};

/** An empty object still yields a single root node (`buildTree` only treats null/undefined as "nothing"). */
export const EmptyObject: Story = {
  args: {
    data: {},
    webTemplate: null,
  },
};
