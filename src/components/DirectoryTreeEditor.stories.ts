import type { Decorator, Meta, StoryObj } from "@storybook/vue3-vite";
import { provide, reactive } from "vue";
import {
  addItem,
  addSubfolder,
  DIRECTORY_MUTATIONS_KEY,
  emptyFolder,
  getFolderAtPath,
  removeItem,
  removeSubfolder,
  type DirectoryMutations,
  type EditableFolder,
} from "../lib/directoryEdit";
import DirectoryTreeEditor from "./DirectoryTreeEditor.vue";

const AVAILABLE_COMPOSITIONS = [
  { uid: "8f3c1e2a-1234-4a5b-8c9d-1234567890ab", label: "Vital Signs Encounter" },
  { uid: "a1b2c3d4-5678-4a5b-8c9d-0987654321ba", label: "Lab Report" },
];

// DirectoryTreeEditor never mutates the `folder` prop it's given — every
// edit goes through the injected DIRECTORY_MUTATIONS_KEY (see
// src/lib/directoryEdit.ts). Outside the real app that's provided by
// EhrBrowser.vue, so stories need the same minimal provider — implemented
// against whatever root folder the story passed as `args.folder` — for
// +Subfolder/+Item clicks to do something in the Storybook canvas.
const withDirectoryMutations: Decorator<{ folder: EditableFolder }> = (story, { args }) => ({
  components: { story },
  setup() {
    const mutations: DirectoryMutations = {
      renameFolder: (path, name) => {
        getFolderAtPath(args.folder, path).name = name;
      },
      renameItemId: (path, key, id) => {
        const item = getFolderAtPath(args.folder, path).items.find((it) => it.key === key);
        if (item) item.id = id;
      },
      addSubfolder: (path) => addSubfolder(getFolderAtPath(args.folder, path)),
      addItem: (path, id, type, namespace, idScheme) =>
        addItem(getFolderAtPath(args.folder, path), id, type, namespace, idScheme),
      removeSubfolder: (parentPath, key) =>
        removeSubfolder(getFolderAtPath(args.folder, parentPath), key),
      removeItem: (parentPath, key) => removeItem(getFolderAtPath(args.folder, parentPath), key),
    };
    provide(DIRECTORY_MUTATIONS_KEY, mutations);
  },
  template: "<story />",
});

const meta: Meta<typeof DirectoryTreeEditor> = {
  title: "Components/DirectoryTreeEditor",
  component: DirectoryTreeEditor,
  tags: ["autodocs"],
  decorators: [withDirectoryMutations],
  parameters: {
    docs: {
      description: {
        component:
          "Editable counterpart to DirectoryTree — lets a user rename folders, add/remove subfolders, and add/remove composition references, then serializes back to DIRECTORY FOLDER JSON via `toWireFolder` (src/lib/directoryEdit.ts). Every edit is applied through the injected DIRECTORY_MUTATIONS_KEY rather than mutating the `folder` prop — these stories provide a minimal implementation of it so +Subfolder/+Item are interactive in the canvas.",
      },
    },
  },
  args: {
    path: [],
    depth: 0,
    isRoot: true,
    availableCompositions: AVAILABLE_COMPOSITIONS,
  },
};

export default meta;
type Story = StoryObj<typeof DirectoryTreeEditor>;

export const Empty: Story = {
  args: {
    folder: reactive(emptyFolder("Root")),
  },
};

export const WithCompositionsAndSubfolders: Story = {
  args: {
    folder: reactive(
      (() => {
        const root = emptyFolder("Patient Records");
        addItem(root, AVAILABLE_COMPOSITIONS[0].uid);
        addSubfolder(root, "2026");
        addItem(root.folders[0], AVAILABLE_COMPOSITIONS[1].uid);
        return root;
      })(),
    ),
  },
};

export const NestedFolders: Story = {
  args: {
    folder: reactive(
      (() => {
        const root = emptyFolder("Patient Records");
        addSubfolder(root, "2026");
        addItem(root.folders[0], AVAILABLE_COMPOSITIONS[0].uid);
        addSubfolder(root.folders[0], "August");
        return root;
      })(),
    ),
  },
};
