import type { Meta, StoryObj } from "@storybook/vue3-vite";
import DownloadButton from "./DownloadButton.vue";

const meta: Meta<typeof DownloadButton> = {
  title: "Website/DownloadButton",
  component: DownloadButton,
  tags: ["autodocs"],
  parameters: {
    // The label swaps to "Download for <OS>" on mount, based on the
    // browser rendering this story — there's nothing to control via args.
    docs: {
      description: {
        component: "Detects the visitor's OS client-side and relabels itself accordingly (e.g. \"Download for macOS\"). The label you see reflects whatever OS is rendering this story.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof DownloadButton>;

export const Default: Story = {};
