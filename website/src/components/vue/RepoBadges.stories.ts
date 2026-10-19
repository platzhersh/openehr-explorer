import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { onUnmounted } from "vue";
import RepoBadges from "./RepoBadges.vue";

const meta: Meta<typeof RepoBadges> = {
  title: "Website/RepoBadges",
  component: RepoBadges,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Downloads and stars pills. Stars come from the GitHub API, downloads from the snapshot history on the repo's `data` branch; a pill whose fetch fails is hidden. `Populated` stubs both fetches.",
      },
    },
  },
};

export default meta;
type Story = StoryObj<typeof RepoBadges>;

export const Default: Story = {};

export const Populated: Story = {
  render: () => ({
    components: { RepoBadges },
    setup() {
      const originalFetch = window.fetch;
      window.fetch = (input) => {
        const url = String(input);
        const body = url.includes("api.github.com")
          ? { stargazers_count: 42 }
          : [{ date: "2026-10-16", total: 727 }];
        return Promise.resolve(new Response(JSON.stringify(body), { status: 200 }));
      };
      onUnmounted(() => {
        window.fetch = originalFetch;
      });
    },
    template: "<RepoBadges />",
  }),
};
