import type { Meta, StoryObj } from "@storybook/vue3-vite";
import DownloadsSparkline from "./DownloadsSparkline.vue";

const meta: Meta<typeof DownloadsSparkline> = {
  title: "Website/DownloadsSparkline",
  component: DownloadsSparkline,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Fetches the live downloads-history JSON from the repo's `data` branch on mount, same as production — there's nothing to control via args, and the story stays hidden until that fetch resolves with at least a week of history (or renders nothing if it's offline/unavailable).",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof DownloadsSparkline>;

export const Default: Story = {};
