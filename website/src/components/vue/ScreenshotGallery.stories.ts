import type { Meta, StoryObj } from "@storybook/vue3-vite";
import ScreenshotGallery from "./ScreenshotGallery.vue";

const meta: Meta<typeof ScreenshotGallery> = {
  title: "Website/ScreenshotGallery",
  component: ScreenshotGallery,
  tags: ["autodocs"],
  // Images are served from website/public via Storybook's staticDirs
  // config (.storybook/main.ts), at the same relative path the site
  // itself uses.
  args: {
    screenshots: [
      {
        src: "assets/screenshots/01-ehr-browser.webp",
        alt: "EHR Browser showing a paginated list of EHRs on the left and a selected EHR's details plus grouped compositions on the right",
        caption: "Browse EHRs and drill into their compositions, grouped by template.",
        width: 1440,
        height: 860,
      },
      {
        src: "assets/screenshots/04-templates.webp",
        alt: "Template Browser showing the list of templates on a server and an interactive OPT tree for the selected template",
        caption: "Inspect Web Templates as a readable tree, with FLAT paths and raw OPT XML alongside.",
        width: 1440,
        height: 610,
      },
      {
        src: "assets/screenshots/06-aql-runner.webp",
        alt: "AQL Runner showing a query editor with a SELECT query and a tabular result set below it",
        caption: "Write and run AQL queries with syntax highlighting, saved queries, and CSV export.",
        width: 1440,
        height: 645,
      },
    ],
  },
};

export default meta;
type Story = StoryObj<typeof ScreenshotGallery>;

export const Default: Story = {};
