import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { fn } from "storybook/test";
import AnalyticsConsentDialog from "./AnalyticsConsentDialog.vue";

const meta: Meta<typeof AnalyticsConsentDialog> = {
  title: "Components/AnalyticsConsentDialog",
  component: AnalyticsConsentDialog,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "First-run modal shown exactly once, when there's no persisted analytics-consent decision yet. No close button by design — see the component's own doc comment for why.",
      },
    },
  },
  args: {
    onAccept: fn(),
    onDecline: fn(),
  },
};

export default meta;
type Story = StoryObj<typeof AnalyticsConsentDialog>;

export const Default: Story = {
  render: (args) => ({
    components: { AnalyticsConsentDialog },
    setup() {
      return { args };
    },
    template: `<AnalyticsConsentDialog @accept="args.onAccept" @decline="args.onDecline" />`,
  }),
};
