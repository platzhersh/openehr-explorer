import type { Meta, StoryObj } from "@storybook/vue3-vite";
import JsonViewer from "./JsonViewer.vue";

const meta: Meta<typeof JsonViewer> = {
  title: "Components/JsonViewer",
  component: JsonViewer,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Reusable syntax-highlighted JSON display (ADR-0021). Walks a parsed value directly rather than re-highlighting `JSON.stringify` output, so line numbers, per-node collapse, and search highlighting all stay in sync with the real structure.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof JsonViewer>;

// A trimmed but realistic openEHR-shaped composition fragment: an
// OBSERVATION with a coded finding (DV_CODED_TEXT, external terminology)
// and two DV_QUANTITY values.
const SAMPLE_VALUE = {
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
    value: SAMPLE_VALUE,
  },
};

export const WithoutLineNumbers: Story = {
  args: {
    value: SAMPLE_VALUE,
    showLineNumbers: false,
  },
};

export const WithoutCopyButton: Story = {
  args: {
    value: SAMPLE_VALUE,
    showCopyButton: false,
  },
};

/** Object/array nodes at depth 2 or deeper (e.g. `data.events[0].data`) start collapsed. */
export const DefaultCollapsed: Story = {
  args: {
    value: SAMPLE_VALUE,
    defaultCollapsedDepth: 2,
  },
};

export const WithSearchTerm: Story = {
  args: {
    value: SAMPLE_VALUE,
    searchTerm: "magnitude",
  },
};

export const WithCurrentMatch: Story = {
  args: {
    value: SAMPLE_VALUE,
    searchTerm: "magnitude",
    currentMatchIndex: 1,
  },
};

export const Primitive: Story = {
  args: {
    value: "just a string",
  },
};

/** An empty object still renders — as a single `{}` line. */
export const EmptyObject: Story = {
  args: {
    value: {},
  },
};
