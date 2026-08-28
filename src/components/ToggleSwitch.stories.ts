import type { Meta, StoryObj } from "@storybook/vue3-vite";
import ToggleSwitch from "./ToggleSwitch.vue";

const meta: Meta<typeof ToggleSwitch> = {
  title: "Components/ToggleSwitch",
  component: ToggleSwitch,
  tags: ["autodocs"],
  args: {
    label: "Enable feature",
    disabled: false,
    modelValue: false,
  },
  argTypes: {
    modelValue: { control: "boolean" },
  },
};

export default meta;
type Story = StoryObj<typeof ToggleSwitch>;

export const Off: Story = {
  args: { modelValue: false },
};

export const On: Story = {
  args: { modelValue: true },
};

export const Disabled: Story = {
  args: { modelValue: false, disabled: true },
};

export const WithoutLabel: Story = {
  args: { label: undefined },
};
