import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { SAMPLE_COMPOSITION } from "../lib/storybook-fixtures";
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

export const Default: Story = {
  args: {
    data: SAMPLE_COMPOSITION,
    webTemplate: null,
  },
};

export const WithHighlightedPath: Story = {
  args: {
    data: SAMPLE_COMPOSITION,
    webTemplate: null,
    highlightedPath: "/data/events[0]/data/items[0]",
  },
};

export const FilteredBySearch: Story = {
  args: {
    data: SAMPLE_COMPOSITION,
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
