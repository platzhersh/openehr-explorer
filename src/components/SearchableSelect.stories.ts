import type { Meta, StoryObj } from "@storybook/vue3-vite";
import SearchableSelect from "./SearchableSelect.vue";

const SERVERS = [
  { value: "srv-1", label: "EhrBase Sandkiste" },
  { value: "srv-2", label: "FerroEHR [ok]" },
  { value: "srv-3", label: "FerroEHR Sandbox" },
];

const meta: Meta<typeof SearchableSelect> = {
  title: "Components/SearchableSelect",
  component: SearchableSelect,
  tags: ["autodocs"],
  args: {
    options: SERVERS,
    placeholder: "Select...",
    searchPlaceholder: "Search...",
    label: undefined,
    noOptionsText: "No matches",
    disabled: false,
    clearable: false,
    modelValue: null,
  },
  argTypes: {
    modelValue: { control: "text" },
  },
};

export default meta;
type Story = StoryObj<typeof SearchableSelect>;

export const Empty: Story = {
  args: { modelValue: null },
};

export const WithSelection: Story = {
  args: { modelValue: "srv-2" },
};

export const Clearable: Story = {
  args: { modelValue: "srv-2", clearable: true },
};

export const WithLabel: Story = {
  args: { label: "Server", modelValue: "srv-1" },
};

export const LongList: Story = {
  args: {
    label: "Template",
    placeholder: "— No template context —",
    options: Array.from({ length: 30 }, (_, i) => ({
      value: `template.v${i + 1}`,
      label: `cistec.openehr.template_${i + 1}.v1`,
    })),
    modelValue: null,
  },
};

export const Disabled: Story = {
  args: { modelValue: "srv-1", disabled: true },
};
