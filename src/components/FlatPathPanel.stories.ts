import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { fn } from "storybook/test";
import FlatPathPanel from "./FlatPathPanel.vue";

const meta: Meta<typeof FlatPathPanel> = {
  title: "Components/FlatPathPanel",
  component: FlatPathPanel,
  tags: ["autodocs"],
  args: {
    onHighlight: fn(),
  },
};

export default meta;
type Story = StoryObj<typeof FlatPathPanel>;

export const WithPaths: Story = {
  args: {
    paths: [
      "blood_pressure/blood_pressure/any_event:0/systolic|magnitude",
      "blood_pressure/blood_pressure/any_event:0/systolic|unit",
      "blood_pressure/blood_pressure/any_event:0/diastolic|magnitude",
      "blood_pressure/blood_pressure/any_event:0/diastolic|unit",
      "blood_pressure/context/start_time",
      "blood_pressure/language|code",
    ],
  },
};

export const WithAqlPaths: Story = {
  args: {
    paths: [
      "blood_pressure/blood_pressure/any_event:0/systolic|magnitude",
      "blood_pressure/language|code",
    ],
    aqlPaths: {
      "blood_pressure/blood_pressure/any_event:0/systolic|magnitude":
        "/content[openEHR-EHR-OBSERVATION.blood_pressure.v2]/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value",
    },
  },
};

export const Empty: Story = {
  args: { paths: [] },
};
