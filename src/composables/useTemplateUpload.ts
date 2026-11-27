import { ref, watch } from "vue";
import { useServerStore } from "../stores/server";
import { useTemplateStore } from "../stores/template";
import { useAnalytics } from "./useAnalytics";
import { open } from "@tauri-apps/plugin-dialog";
import { readTextFile } from "@tauri-apps/plugin-fs";

/**
 * Shared OPT upload flow (drag & drop + choose-file) used by both the
 * "Upload Template" modal and the inline drop zone shown when a server has
 * no templates yet. Each caller gets its own independent state — that's
 * fine, since the two entry points are never meant to reflect each other's
 * drag-over/progress state.
 */
export function useTemplateUpload() {
  const serverStore = useServerStore();
  const templateStore = useTemplateStore();
  const analytics = useAnalytics();

  const dragOver = ref(false);
  const uploading = ref(false);
  const uploadStatus = ref<string | null>(null);
  const uploadError = ref<string | null>(null);

  // OPT text held back after the server rejected it as a duplicate (HTTP 409),
  // awaiting the user's explicit decision to overwrite.
  const pendingOverwrite = ref<string | null>(null);

  function resetState() {
    dragOver.value = false;
    uploading.value = false;
    uploadStatus.value = null;
    uploadError.value = null;
    pendingOverwrite.value = null;
  }

  async function performUpload(text: string, overwrite = false) {
    if (!serverStore.activeServerId) return;

    uploading.value = true;
    uploadStatus.value = null;
    uploadError.value = null;
    pendingOverwrite.value = null;

    try {
      const result = await templateStore.uploadTemplate(
        serverStore.activeServerId,
        text,
        overwrite,
      );
      uploadStatus.value = result;
      void analytics.track("template_uploaded");
      await templateStore.fetchTemplates(serverStore.activeServerId);
      // An overwrite keeps the template ID, so the open template's details
      // (Web Template, OPT, term bindings) must be reloaded explicitly.
      if (overwrite) await templateStore.reloadSelected();
    } catch (e) {
      const message = String(e);
      if (!overwrite && message.includes("HTTP 409")) {
        pendingOverwrite.value = text;
        uploadError.value = "A template with this ID already exists on the server.";
      } else {
        uploadError.value = message;
      }
    } finally {
      uploading.value = false;
    }
  }

  async function confirmOverwrite() {
    if (pendingOverwrite.value === null) return;
    await performUpload(pendingOverwrite.value, true);
  }

  function cancelOverwrite() {
    pendingOverwrite.value = null;
    uploadError.value = null;
  }

  // A pending duplicate prompt belongs to the server that returned HTTP 409;
  // never let a later confirmation overwrite on a different server.
  watch(
    () => serverStore.activeServerId,
    () => cancelOverwrite(),
  );

  async function handleDrop(event: DragEvent) {
    event.preventDefault();
    dragOver.value = false;
    const file = event.dataTransfer?.files[0];
    if (!file) return;

    await performUpload(await file.text());
  }

  async function handleFileSelect() {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "OPT Files",
            extensions: ["opt", "xml"],
          },
        ],
      });

      if (selected && typeof selected === "string") {
        // Read file using Tauri's FS plugin
        await performUpload(await readTextFile(selected));
      }
    } catch (e) {
      uploadError.value = String(e);
    }
  }

  return {
    dragOver,
    uploading,
    uploadStatus,
    uploadError,
    pendingOverwrite,
    confirmOverwrite,
    cancelOverwrite,
    resetState,
    handleDrop,
    handleFileSelect,
  };
}
