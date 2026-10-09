import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { expect, userEvent, within } from "storybook/test";
import JsonTreeNode from "./JsonTreeNode.vue";

const meta: Meta<typeof JsonTreeNode> = {
  title: "Components/JsonTreeNode",
  component: JsonTreeNode,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Recursive JSON tree node used by the composition JSON view. Renders itself for each child of an object/array value, so a single top-level node can render an arbitrarily deep tree.",
      },
    },
  },
  args: {
    label: "root",
    depth: 0,
  },
};

export default meta;
type Story = StoryObj<typeof JsonTreeNode>;

export const StringValue: Story = {
  args: { label: "name", value: "Blood Pressure" },
};

export const NumberValue: Story = {
  args: { label: "magnitude", value: 120.5 },
};

export const BooleanValue: Story = {
  args: { label: "is_active", value: true },
};

export const NullValue: Story = {
  args: { label: "comment", value: null },
};

export const ObjectValue: Story = {
  args: {
    label: "systolic",
    value: {
      _type: "DV_QUANTITY",
      magnitude: 120,
      units: "mm[Hg]",
      precision: 0,
    },
  },
};

export const ArrayValue: Story = {
  args: {
    label: "events",
    value: [
      { name: "Point event", time: "2026-08-27T09:00:00Z" },
      { name: "Point event", time: "2026-08-27T09:15:00Z" },
    ],
  },
};

/** A small openEHR-shaped composition fragment, several levels deep. */
export const OpenEhrComposition: Story = {
  args: {
    label: "blood_pressure",
    value: {
      _type: "OBSERVATION",
      archetype_node_id: "openEHR-EHR-OBSERVATION.blood_pressure.v2",
      name: { value: "Blood pressure" },
      data: {
        _type: "HISTORY",
        origin: { value: "2026-08-27T09:00:00Z" },
        events: [
          {
            _type: "POINT_EVENT",
            data: {
              systolic: { _type: "DV_QUANTITY", magnitude: 120, units: "mm[Hg]" },
              diastolic: { _type: "DV_QUANTITY", magnitude: 80, units: "mm[Hg]" },
            },
          },
        ],
      },
    },
    depth: 0,
  },
};

export const WithSearchTermMatch: Story = {
  args: {
    label: "systolic",
    value: { _type: "DV_QUANTITY", magnitude: 120, units: "mm[Hg]" },
    searchTerm: "systolic",
  },
};

export const ToggleIsKeyboardAccessible: Story = {
  args: {
    label: "systolic",
    value: { magnitude: 120, units: "mm[Hg]" },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const toggle = await canvas.findByRole("button", { name: /systolic/i });
    const initial = toggle.getAttribute("aria-expanded");
    // Keyboard: Enter and Space each flip the state exactly once.
    toggle.focus();
    await userEvent.keyboard("{Enter}");
    await expect(toggle).toHaveAttribute("aria-expanded", initial === "true" ? "false" : "true");
    await userEvent.keyboard(" ");
    await expect(toggle).toHaveAttribute("aria-expanded", initial ?? "false");
    // Mouse: one click flips once (the row's own click handler must not double-toggle).
    await userEvent.click(toggle);
    await expect(toggle).toHaveAttribute("aria-expanded", initial === "true" ? "false" : "true");
  },
};
