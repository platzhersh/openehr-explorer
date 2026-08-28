import { ref } from "vue";
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

  function resetState() {
    dragOver.value = false;
    uploading.value = false;
    uploadStatus.value = null;
    uploadError.value = null;
  }

  async function performUpload(text: string) {
    if (!serverStore.activeServerId) return;

    uploading.value = true;
    uploadStatus.value = null;
    uploadError.value = null;

    try {
      const result = await templateStore.uploadTemplate(serverStore.activeServerId, text);
      uploadStatus.value = result;
      void analytics.track("template_uploaded");
      await templateStore.fetchTemplates(serverStore.activeServerId);
    } catch (e) {
      uploadError.value = String(e);
    } finally {
      uploading.value = false;
    }
  }

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
    resetState,
    handleDrop,
    handleFileSelect,
  };
}
