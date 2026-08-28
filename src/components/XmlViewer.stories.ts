import type { Meta, StoryObj } from "@storybook/vue3-vite";
import XmlViewer from "./XmlViewer.vue";

const meta: Meta<typeof XmlViewer> = {
  title: "Components/XmlViewer",
  component: XmlViewer,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Reusable syntax-highlighted XML display (ADR-0021, OEH-35). Tokenizes the raw XML text (`src/lib/xml.ts`) instead of re-highlighting an HTML-escaped string with regex, so namespaced tags (e.g. `xs:string`) highlight correctly and line numbers/search stay in sync with the real tag structure.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof XmlViewer>;

// A trimmed but realistic OPT (Operational Template) XML fragment, including
// a namespaced schema type (`xs:string`) and a comment.
const SAMPLE_XML = `<?xml version="1.0" encoding="UTF-8"?>
<template xmlns="http://schemas.openehr.org/v1">
  <!-- Blood pressure observation template -->
  <template_id>
    <value>blood_pressure.v1</value>
  </template_id>
  <concept>blood_pressure</concept>
  <definition xsi:type="xs:string" archetype_id="openEHR-EHR-OBSERVATION.blood_pressure.v2">
    <node_id>at0000</node_id>
    <attributes>
      <children xsi:type="C_COMPLEX_OBJECT">
        <rm_type_name>ELEMENT</rm_type_name>
        <occurrences lower_included="true" upper_included="true">
          <lower>0</lower>
          <upper>1</upper>
        </occurrences>
      </children>
    </attributes>
  </definition>
</template>
`;

export const Default: Story = {
  args: {
    xml: SAMPLE_XML,
  },
};

export const WithoutLineNumbers: Story = {
  args: {
    xml: SAMPLE_XML,
    showLineNumbers: false,
  },
};

export const WithoutCopyButton: Story = {
  args: {
    xml: SAMPLE_XML,
    showCopyButton: false,
  },
};

export const WithSearchTerm: Story = {
  args: {
    xml: SAMPLE_XML,
    searchTerm: "xs:string",
  },
};

export const WithCurrentMatch: Story = {
  args: {
    xml: SAMPLE_XML,
    searchTerm: "occurrences",
    currentMatchIndex: 1,
  },
};

export const Empty: Story = {
  args: {
    xml: "",
  },
};
