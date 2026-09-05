import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { onUnmounted } from "vue";
import DownloadsSparkline from "./DownloadsSparkline.vue";

const meta: Meta<typeof DownloadsSparkline> = {
  title: "Website/DownloadsSparkline",
  component: DownloadsSparkline,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Fetches the live downloads-history JSON from the repo's `data` branch on mount, same as production — there's nothing to control via args, and the `Default` story stays hidden until that fetch resolves with more than 3 days of history (or renders nothing if it's offline/unavailable, which is the common case in Storybook). `Populated` stubs the fetch with canned history so the rendered sparkline itself is visible here too.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof DownloadsSparkline>;

export const Default: Story = {};

// A fortnight of steadily climbing totals — enough points to clear
// MIN_POINTS and show a real upward curve.
const MOCK_HISTORY = Array.from({ length: 14 }, (_, i) => ({
  date: `2026-08-${String(i + 1).padStart(2, "0")}`,
  total: 1200 + i * 35 + (i % 3 === 0 ? 15 : 0), // slight wobble, not a perfectly straight line
}));

export const Populated: Story = {
  render: () => ({
    components: { DownloadsSparkline },
    setup() {
      const originalFetch = window.fetch;
      window.fetch = async () => new Response(JSON.stringify(MOCK_HISTORY), { status: 200 });
      onUnmounted(() => {
        window.fetch = originalFetch;
      });
    },
    template: "<DownloadsSparkline />",
  }),
};
