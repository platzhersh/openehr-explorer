import type { Meta, StoryObj } from "@storybook/vue3-vite";
import { onUnmounted } from "vue";
import { createPinia, setActivePinia } from "pinia";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { expect, userEvent, within } from "storybook/test";
import TemplateUploadModal from "./TemplateUploadModal.vue";
import { useServerStore } from "../stores/server";

// Mocks the two Tauri plugin commands behind the "choose file" flow (see
// useTemplateUpload.ts): plugin-dialog's open() resolves to a fake path,
// then plugin-fs's readTextFile() resolves that path to OPT XML bytes. Used
// instead of simulating a real drag & drop — a real browser's DragEvent
// exposes `dataTransfer` as a read-only accessor, so it can't be set from a
// synthetic event the way jsdom allows.
function mockChooseFile(optXml: string) {
  return (cmd: string) => {
    if (cmd === "plugin:dialog|open") return "/fake/international-patient-summary.opt";
    if (cmd === "plugin:fs|read_text_file") return Array.from(new TextEncoder().encode(optXml));
  };
}

// TemplateUploadModal reads its state through the useTemplateUpload
// composable, which talks to the server/template Pinia stores and — for the
// actual upload — Tauri IPC. Storybook runs outside a Tauri runtime, so each
// story seeds a fresh Pinia instance with an active server and, for the
// interactive stories, mocks the `upload_template`/`list_templates` commands
// with Tauri's own mock-IPC helper (the same one it recommends for tests).
function withStores() {
  return () => ({
    components: { TemplateUploadModal },
    setup() {
      setActivePinia(createPinia());
      const serverStore = useServerStore();
      serverStore.activeServerId = "story-server";
      return {};
    },
    template: `<TemplateUploadModal :open="true" @close="() => {}" />`,
  });
}

const meta: Meta<typeof TemplateUploadModal> = {
  title: "Components/TemplateUploadModal",
  component: TemplateUploadModal,
  tags: ["autodocs"],
  parameters: {
    docs: {
      description: {
        component:
          "Dialog for uploading a new Operational Template (OPT), opened from the '+ Upload Template' button in the Templates panel header. Wraps TemplateUploadZone for the actual drag & drop / choose-file widget.",
      },
    },
  },
  render: withStores(),
};

export default meta;
type Story = StoryObj<typeof TemplateUploadModal>;

/** Freshly opened — no upload attempted yet. */
export const Open: Story = {};

/** Closed modals render nothing (`v-if="open"`), so there's nothing to
 *  screenshot — this exists to document that `open: false` is a real,
 *  supported state rather than an oversight. */
export const Closed: Story = {
  render: () => ({
    components: { TemplateUploadModal },
    setup() {
      setActivePinia(createPinia());
      return {};
    },
    template: `<TemplateUploadModal :open="false" @close="() => {}" />`,
  }),
};

/** Choosing a valid OPT file uploads it and shows the server's confirmation
 *  message. */
export const UploadSucceeded: Story = {
  render: () => ({
    components: { TemplateUploadModal },
    setup() {
      setActivePinia(createPinia());
      const serverStore = useServerStore();
      serverStore.activeServerId = "story-server";

      const chooseFile = mockChooseFile("<template/>");
      mockIPC((cmd) => {
        if (cmd === "upload_template") {
          return "Template 'International Patient Summary v7.1' uploaded successfully.";
        }
        if (cmd === "list_templates") return [];
        return chooseFile(cmd);
      });
      onUnmounted(() => clearMocks());

      return {};
    },
    template: `<TemplateUploadModal :open="true" @close="() => {}" />`,
  }),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: /choose file/i }));
    await expect(await canvas.findByText(/uploaded successfully/i)).toBeVisible();
  },
};

/** The server rejects the OPT (e.g. malformed XML, or a duplicate template
 *  ID) and the error is shown inline instead of closing the dialog. */
export const UploadFailed: Story = {
  render: () => ({
    components: { TemplateUploadModal },
    setup() {
      setActivePinia(createPinia());
      const serverStore = useServerStore();
      serverStore.activeServerId = "story-server";

      const chooseFile = mockChooseFile("<not-a-valid-opt/>");
      mockIPC((cmd) => {
        if (cmd === "upload_template") {
          throw new Error("a template with this ID already exists");
        }
        if (cmd === "list_templates") return [];
        return chooseFile(cmd);
      });
      onUnmounted(() => clearMocks());

      return {};
    },
    template: `<TemplateUploadModal :open="true" @close="() => {}" />`,
  }),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: /choose file/i }));
    await expect(await canvas.findByText(/already exists/i)).toBeVisible();
  },
};
