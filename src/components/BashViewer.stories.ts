import type { Meta, StoryObj } from "@storybook/vue3-vite";
import BashViewer from "./BashViewer.vue";

const meta: Meta<typeof BashViewer> = {
  title: "Components/BashViewer",
  component: BashViewer,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Reusable syntax-highlighted bash/cURL display (ADR-0021, mirroring JsonViewer/XmlViewer). Tokenizes the raw command text (`src/lib/bash.ts`) instead of re-highlighting an already-escaped string, so the command word, flags, quoted strings, and line continuations highlight independently. Used for the cURL Command panel in the Request Inspector.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof BashViewer>;

// Matches the shape `generateCurl` (src/stores/inspector.ts) produces: one
// `curl -X <method>` line, then each argument on its own continuation line.
const SAMPLE_CURL = [
  "curl -X POST \\",
  "  'https://sandbox.ehrbase.org/ehrbase/rest/openehr/v1/ehr/9b48...cbb4/composition' \\",
  "  -H 'authorization: Basic ****' \\",
  "  -H 'accept: application/json' \\",
  "  -H 'content-type: application/json' \\",
  '  -d \'{"name":"blood_pressure"}\'',
].join("\n");

export const Default: Story = {
  args: {
    code: SAMPLE_CURL,
  },
};

export const WithoutLineNumbers: Story = {
  args: {
    code: SAMPLE_CURL,
    showLineNumbers: false,
  },
};

export const WithoutCopyButton: Story = {
  args: {
    code: SAMPLE_CURL,
    showCopyButton: false,
  },
};

export const SingleLine: Story = {
  args: {
    code: "curl -X GET 'https://sandbox.ehrbase.org/ehrbase/rest/openehr/v1/definition/template/adl1.4'",
  },
};

export const Empty: Story = {
  args: {
    code: "",
  },
};
