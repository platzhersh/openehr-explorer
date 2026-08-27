import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { fn } from "storybook/test";
import { emptyFolder, addItem, addSubfolder } from "../lib/directoryEdit";
import DirectoryTreeEditor from "./DirectoryTreeEditor.vue";

const meta: Meta<typeof DirectoryTreeEditor> = {
  title: "Components/DirectoryTreeEditor",
  component: DirectoryTreeEditor,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Editable counterpart to DirectoryTree — lets a user rename folders, add/remove subfolders, and add/remove composition references, then serializes back to DIRECTORY FOLDER JSON via `toWireFolder` (src/lib/directoryEdit.ts).",
      },
    },
  },
  args: {
    depth: 0,
    isRoot: true,
    availableCompositions: [
      { uid: "8f3c1e2a-1234-4a5b-8c9d-1234567890ab", label: "Vital Signs Encounter" },
      { uid: "a1b2c3d4-5678-4a5b-8c9d-0987654321ba", label: "Lab Report" },
    ],
    onRemove: fn(),
  },
};

export default meta;
type Story = StoryObj<typeof DirectoryTreeEditor>;

export const EmptyRoot: Story = {
  args: {
    folder: emptyFolder("Root"),
  },
};

export const WithCompositionsAndSubfolders: Story = {
  args: {
    folder: (() => {
      const root = emptyFolder("Patient Records");
      addItem(root, "8f3c1e2a-1234-4a5b-8c9d-1234567890ab");
      addSubfolder(root, "2026");
      addItem(root.folders[0], "a1b2c3d4-5678-4a5b-8c9d-0987654321ba");
      return root;
    })(),
  },
};
