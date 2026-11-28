import type { Meta, StoryObj } from "@storybook/vue3-vite";
import CompositionValidationResult from "./CompositionValidationResult.vue";

const meta: Meta<typeof CompositionValidationResult> = {
  title: "Components/CompositionValidationResult",
  component: CompositionValidationResult,
  tags: ["autodocs"],
};

export default meta;
type Story = StoryObj<typeof CompositionValidationResult>;

export const Valid: Story = {
  args: { result: { valid: true, code: null, message: null, details: [] } },
};

export const Invalid: Story = {
  args: {
    result: {
      valid: false,
      code: "COMP-1081",
      message: "Composition validation failed (there are missing or invalid values).",
      details: [
        "Missing DvCodedText.value at vitals/vitals/body_temperature:0/any_event:0/symptoms:0",
      ],
    },
  },
};

export const InvalidWithoutReason: Story = {
  args: { result: { valid: false, code: null, message: null, details: [] } },
};
