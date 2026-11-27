import { computed, ref, watch } from "vue";
import { useServerStore } from "../stores/server";
import { useTemplateStore } from "../stores/template";
import { optTemplateId } from "../lib/optTemplateId";
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
  // awaiting the user's explicit decision to overwrite. It is pinned to the
  // server that returned the 409 so a confirmation can never reach another one.
  const pending = ref<{ text: string; serverId: string } | null>(null);
  const pendingOverwrite = computed(() => pending.value?.text ?? null);

  // For the overwrite confirmation: which template would be replaced, and on which server.
  const pendingTemplateId = computed(() =>
    pending.value ? optTemplateId(pending.value.text) : null,
  );
  const pendingServerName = computed(
    () => serverStore.profiles.find((p) => p.id === pending.value?.serverId)?.name ?? null,
  );

  // Only the latest upload attempt may update the local upload state (status, error,
  // overwrite prompt): a slower, superseded attempt must not replace it. Resetting also
  // supersedes every attempt started before it.
  let latestAttempt = 0;

  function resetState() {
    latestAttempt++;
    dragOver.value = false;
    uploading.value = false;
    uploadStatus.value = null;
    uploadError.value = null;
    pending.value = null;
  }

  async function performUpload(text: string, overwrite = false) {
    // Pin the target server: the user may switch servers while the request is in flight.
    const serverId = serverStore.activeServerId;
    if (!serverId) return;
    const attempt = ++latestAttempt;
    const isLatest = () => attempt === latestAttempt;
    const serverStillActive = () => serverStore.activeServerId === serverId;

    uploading.value = true;
    uploadStatus.value = null;
    uploadError.value = null;
    pending.value = null;

    try {
      const result = await templateStore.uploadTemplate(serverId, text, overwrite);
      // Drop results for a server the user has since left.
      if (!serverStillActive()) return;
      if (isLatest()) uploadStatus.value = result;
      void analytics.track("template_uploaded");
      // The upload changed the server even if a newer attempt owns the local state.
      await templateStore.fetchTemplates(serverId);
      // An overwrite keeps the template ID, so the open template's details
      // (Web Template, OPT, term bindings) must be reloaded explicitly.
      if (overwrite && serverStillActive()) templateStore.requestDetailReload();
    } catch (e) {
      if (!isLatest() || !serverStillActive()) return;
      const message = String(e);
      if (!overwrite && message.includes("HTTP 409")) {
        pending.value = { text, serverId };
        uploadError.value = "A template with this ID already exists on the server.";
      } else {
        uploadError.value = message;
      }
    } finally {
      if (isLatest()) uploading.value = false;
    }
  }

  async function confirmOverwrite() {
    const current = pending.value;
    if (!current) return;
    // Verify the server again: only overwrite where the duplicate was reported.
    if (current.serverId !== serverStore.activeServerId) {
      cancelOverwrite();
      return;
    }
    await performUpload(current.text, true);
  }

  function cancelOverwrite() {
    pending.value = null;
    uploadError.value = null;
  }

  // A pending duplicate prompt belongs to the server that returned HTTP 409.
  watch(
    () => serverStore.activeServerId,
    () => {
      latestAttempt++; // invalidate any in-flight upload for the server being left
      uploading.value = false;
      cancelOverwrite();
    },
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
    pendingTemplateId,
    pendingServerName,
    confirmOverwrite,
    cancelOverwrite,
    resetState,
    handleDrop,
    handleFileSelect,
  };
}
