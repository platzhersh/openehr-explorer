import type { Meta, StoryObj } from "@storybook/vue3-vite";
import CompositionStateBanner from "./CompositionStateBanner.vue";

const meta: Meta<typeof CompositionStateBanner> = {
  title: "Components/CompositionStateBanner",
  component: CompositionStateBanner,
  tags: ["autodocs"],
};

export default meta;
type Story = StoryObj<typeof CompositionStateBanner>;

const latest_uid = "c1a2b3c4::cdr.example.com::3";

export const Deleted: Story = {
  args: { state: { deleted: true, is_latest: true, latest_uid } },
};

export const NotLatest: Story = {
  args: { state: { deleted: false, is_latest: false, latest_uid } },
};

export const RestoreFailed: Story = {
  args: {
    state: { deleted: true, is_latest: false, latest_uid },
    error: "Server returned HTTP 403: Forbidden",
  },
};
