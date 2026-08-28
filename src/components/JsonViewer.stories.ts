import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { SAMPLE_COMPOSITION } from "../lib/storybook-fixtures";
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

export const Default: Story = {
  args: {
    value: SAMPLE_COMPOSITION,
  },
};

export const WithoutLineNumbers: Story = {
  args: {
    value: SAMPLE_COMPOSITION,
    showLineNumbers: false,
  },
};

export const WithoutCopyButton: Story = {
  args: {
    value: SAMPLE_COMPOSITION,
    showCopyButton: false,
  },
};

/** Object/array nodes at depth 2 or deeper (e.g. `data.events[0].data`) start collapsed. */
export const DefaultCollapsed: Story = {
  args: {
    value: SAMPLE_COMPOSITION,
    defaultCollapsedDepth: 2,
  },
};

export const WithSearchTerm: Story = {
  args: {
    value: SAMPLE_COMPOSITION,
    searchTerm: "magnitude",
  },
};

export const WithCurrentMatch: Story = {
  args: {
    value: SAMPLE_COMPOSITION,
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
