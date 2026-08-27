import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { fn } from "storybook/test";
import TemplateUploadZone from "./TemplateUploadZone.vue";

const meta: Meta<typeof TemplateUploadZone> = {
  title: "Components/TemplateUploadZone",
  component: TemplateUploadZone,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Drag & drop / choose-file OPT upload widget shared by TemplateUploadModal.vue and the empty-server inline state in TemplateBrowser.vue. Purely presentational — all upload state (drag-over, in-flight, status/error) lives in the useTemplateUpload composable and is passed in as props, so both callers render the same widget instead of duplicating it.",
      },
    },
  },
  args: {
    dragOver: false,
    uploading: false,
    uploadStatus: null,
    uploadError: null,
    onDragover: fn(),
    onDragleave: fn(),
    onDrop: fn(),
    onChooseFile: fn(),
  },
  render: (args) => ({
    components: { TemplateUploadZone },
    setup() {
      return { args };
    },
    template: `<TemplateUploadZone
      :drag-over="args.dragOver"
      :uploading="args.uploading"
      :upload-status="args.uploadStatus"
      :upload-error="args.uploadError"
      @dragover="args.onDragover"
      @dragleave="args.onDragleave"
      @drop="args.onDrop"
      @choose-file="args.onChooseFile"
    />`,
  }),
};

export default meta;
type Story = StoryObj<typeof TemplateUploadZone>;

/** Idle, ready to accept a file. */
export const Default: Story = {};

/** A file is being dragged over the drop target. */
export const DragOver: Story = {
  args: { dragOver: true },
};

/** Upload in flight — the choose-file button is disabled and relabelled. */
export const Uploading: Story = {
  args: { uploading: true },
};

/** The server accepted the OPT and returned a confirmation message. */
export const UploadSucceeded: Story = {
  args: {
    uploadStatus: "Template 'International Patient Summary v7.1' uploaded successfully.",
  },
};

/** The server rejected the OPT — e.g. malformed XML or a duplicate template ID. */
export const UploadFailed: Story = {
  args: {
    uploadError: "Failed to upload template: a template with this ID already exists.",
  },
};
