import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { fn } from "storybook/test";
import SearchOverlay from "./SearchOverlay.vue";

const meta: Meta<typeof SearchOverlay> = {
  title: "Components/SearchOverlay",
  component: SearchOverlay,
  tags: ["autodocs"],
  args: {
    modelValue: "",
    placeholder: "Search...",
    "onUpdate:modelValue": fn(),
    onClose: fn(),
    onNext: fn(),
    onPrevious: fn(),
  },
};

export default meta;
type Story = StoryObj<typeof SearchOverlay>;

// v-model is wired back to args (mutated from the template, not typed TS —
// Storybook's own args type marks props readonly) so typing in the
// Storybook canvas behaves like it would in the real app.
const render: NonNullable<Story["render"]> = (args) => ({
  components: { SearchOverlay },
  setup() {
    return { args };
  },
  template: `<SearchOverlay v-bind="args" @update:modelValue="args.modelValue = $event" />`,
});

export const Empty: Story = {
  render,
};

export const WithMatches: Story = {
  render,
  args: {
    modelValue: "blood",
    matchCount: 1,
    totalMatches: 4,
  },
};

export const NoMatches: Story = {
  render,
  args: {
    modelValue: "xyzzy",
    totalMatches: 0,
  },
};
