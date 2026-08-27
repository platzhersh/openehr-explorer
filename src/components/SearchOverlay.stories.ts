import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { fn } from "storybook/test";
import { useArgs } from "storybook/preview-api";
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

// v-model is wired back through Storybook's useArgs so both the input and
// the Controls panel stay in sync, like a real parent's v-model ref would.
const render: NonNullable<Story["render"]> = (args) => {
  const [, updateArgs] = useArgs();
  return {
    components: { SearchOverlay },
    setup() {
      function onInput(value: string) {
        updateArgs({ modelValue: value });
      }
      return { args, onInput };
    },
    template: `<SearchOverlay v-bind="args" @update:modelValue="onInput" />`,
  };
};

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
