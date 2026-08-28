import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { fn } from "storybook/test";
import DirectoryTree from "./DirectoryTree.vue";

const meta: Meta<typeof DirectoryTree> = {
  title: "Components/DirectoryTree",
  component: DirectoryTree,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Renders an openEHR DIRECTORY resource — a FOLDER tree where each folder has nested folders and items (OBJECT_REFs, almost always pointing at COMPOSITIONs). Recurses into itself for subfolders.",
      },
    },
  },
  args: {
    depth: 0,
    "onOpen-item": fn(),
  },
};

export default meta;
type Story = StoryObj<typeof DirectoryTree>;

export const Empty: Story = {
  args: {
    folder: { name: { value: "Root" }, items: [], folders: [] },
  },
};

export const WithCompositions: Story = {
  args: {
    folder: {
      name: { value: "Encounters" },
      items: [
        {
          id: { value: "8f3c1e2a-1234-4a5b-8c9d-1234567890ab" },
          type: "COMPOSITION",
          namespace: "local",
        },
        {
          id: { value: "a1b2c3d4-5678-4a5b-8c9d-0987654321ba" },
          type: "COMPOSITION",
          namespace: "local",
        },
      ],
      folders: [],
    },
  },
};

export const NestedFolders: Story = {
  args: {
    folder: {
      name: { value: "Patient Records" },
      items: [],
      folders: [
        {
          uid: { value: "folder-2026" },
          name: { value: "2026" },
          items: [{ id: { value: "e5f6a7b8-1234-4a5b-8c9d-1234567890ab" }, type: "COMPOSITION" }],
          folders: [
            {
              uid: { value: "folder-2026-08" },
              name: { value: "August" },
              items: [],
              folders: [],
            },
          ],
        },
      ],
    },
  },
};
