import { computed, ref, watch } from "vue";
import { useServerStore } from "../stores/server";
import { useTemplateStore } from "../stores/template";
import { invoke } from "@tauri-apps/api/core";
import {
  duplicateMayReplace,
  duplicateWithoutOverwriteMessage,
  supportsTemplateOverwrite,
} from "../lib/templateOverwrite";
import { useAnalytics } from "./useAnalytics";
import { open } from "@tauri-apps/plugin-dialog";
import { readTextFile } from "@tauri-apps/plugin-fs";

/** The template ID as the backend reads it from the OPT (null if it can't). */
function parseTemplateId(text: string): Promise<string | null> {
  return invoke<string | null>("parse_opt_template_id", { optXml: text }).catch(() => null);
}

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

  // OPT text held back because the template already exists on the server (it is listed, or
  // the server answered HTTP 409), awaiting the user's explicit decision to overwrite. It is
  // pinned to the server it was found on so a confirmation can never reach another one.
  // `replaceFirst`: the duplicate was spotted in the template list, before any request, so a
  // confirmed upload tries a plain upload first (some servers replace on that) and only then
  // the vendor-specific overwrite.
  const pending = ref<{
    text: string;
    serverId: string;
    templateId: string | null;
    replaceFirst: boolean;
  } | null>(null);
  const pendingOverwrite = computed(() => pending.value?.text ?? null);

  // For the overwrite confirmation: which template would be replaced (as the backend reads
  // it from the OPT, so it is exactly the one the overwrite targets) and on which server.
  const pendingTemplateId = computed(() => pending.value?.templateId ?? null);
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

  /** One upload, pinned to the server it was started for. */
  interface Attempt {
    serverId: string;
    /** No newer attempt (or reset) has superseded this one. */
    isLatest: () => boolean;
    /** The user is still on the server this upload was sent to. */
    serverStillActive: () => boolean;
    /** Safe to update the local upload state: latest attempt, same server. */
    isCurrent: () => boolean;
  }

  function beginAttempt(serverId: string): Attempt {
    const id = ++latestAttempt;
    const isLatest = () => id === latestAttempt;
    const serverStillActive = () => serverStore.activeServerId === serverId;
    return {
      serverId,
      isLatest,
      serverStillActive,
      isCurrent: () => isLatest() && serverStillActive(),
    };
  }

  /** How one upload is to be sent. */
  interface UploadOptions {
    /** Use the vendor-specific overwrite instead of a plain upload. */
    overwrite?: boolean;
    /** The user already confirmed replacing an existing template; don't ask again. */
    replaceConfirmed?: boolean;
  }

  const EXISTS_MESSAGE = "A template with this ID already exists on the server.";

  function serverTypeOf(serverId: string) {
    return serverStore.profiles.find((p) => p.id === serverId)?.server_type;
  }

  /**
   * Before sending anything: if the template is already listed on the server, hold the OPT
   * and ask the user. Servers may replace on a plain upload without any warning (the
   * EHRbase sandbox does), so waiting for a 409 is not enough. Returns true if the upload
   * must not go ahead now (the user is being asked, or has moved on).
   */
  async function askBeforeReplacing(attempt: Attempt, text: string): Promise<boolean> {
    // Nothing listed means nothing to compare with: skip the extra round trip to the backend.
    if (templateStore.templates.length === 0) return false;
    if (!duplicateMayReplace(serverTypeOf(attempt.serverId))) return false;
    const templateId = await parseTemplateId(text);
    const listed = templateStore.templates.some((t) => t.template_id === templateId);
    if (!templateId || !listed) return false;
    if (attempt.isCurrent()) {
      pending.value = { text, serverId: attempt.serverId, templateId, replaceFirst: true };
      uploadError.value = EXISTS_MESSAGE;
    }
    return true;
  }

  async function onUploadSucceeded(attempt: Attempt, result: string, options: UploadOptions) {
    // Drop results for a server the user has since left.
    if (!attempt.serverStillActive()) return;
    // A confirmed plain upload replaced the template if it existed; say so.
    const replaced = options.replaceConfirmed;
    const status = replaced ? result.replace(/^Template uploaded/, "Template overwritten") : result;
    if (attempt.isLatest()) uploadStatus.value = status;
    void analytics.track("template_uploaded");
    // The upload changed the server even if a newer attempt owns the local state.
    await templateStore.fetchTemplates(attempt.serverId);
    // An overwrite keeps the template ID, so the open template's details
    // (Web Template, OPT, term bindings) must be reloaded explicitly.
    const mayHaveReplaced = options.overwrite || replaced;
    if (mayHaveReplaced && attempt.serverStillActive()) templateStore.requestDetailReload();
  }

  async function onUploadFailed(
    attempt: Attempt,
    text: string,
    options: UploadOptions,
    error: unknown,
  ) {
    if (!attempt.isCurrent()) return;
    const message = String(error);
    if (options.overwrite || !message.includes("HTTP 409")) {
      uploadError.value = message;
      return;
    }
    // Duplicate. Only offer an overwrite where the server type can actually do one.
    const serverType = serverTypeOf(attempt.serverId);
    if (!supportsTemplateOverwrite(serverType)) {
      uploadError.value = duplicateWithoutOverwriteMessage(serverType);
      return;
    }
    // The user already confirmed replacing it and a plain upload was refused: overwrite.
    if (options.replaceConfirmed) {
      await performUpload(text, { overwrite: true });
      return;
    }
    // Ask the backend which template it is, so the confirmation names exactly
    // the one an overwrite would replace.
    const templateId = await parseTemplateId(text);
    // The user may have moved on while the backend parsed the OPT.
    if (!attempt.isCurrent()) return;
    pending.value = { text, serverId: attempt.serverId, templateId, replaceFirst: false };
    uploadError.value = EXISTS_MESSAGE;
  }

  async function performUpload(text: string, options: UploadOptions = {}) {
    // Pin the target server: the user may switch servers while the request is in flight.
    const serverId = serverStore.activeServerId;
    if (!serverId) return;
    const attempt = beginAttempt(serverId);

    uploading.value = true;
    uploadStatus.value = null;
    uploadError.value = null;
    pending.value = null;

    try {
      const confirmedOrOverwrite = options.overwrite || options.replaceConfirmed;
      if (!confirmedOrOverwrite && (await askBeforeReplacing(attempt, text))) return;
      const result = await templateStore.uploadTemplate(serverId, text, !!options.overwrite);
      await onUploadSucceeded(attempt, result, options);
    } catch (e) {
      await onUploadFailed(attempt, text, options, e);
    } finally {
      if (attempt.isLatest()) uploading.value = false;
    }
  }

  async function confirmOverwrite() {
    const current = pending.value;
    if (!current) return;
    // Verify the server again: only overwrite where the duplicate was found.
    if (current.serverId !== serverStore.activeServerId) {
      cancelOverwrite();
      return;
    }
    await performUpload(
      current.text,
      current.replaceFirst ? { replaceConfirmed: true } : { overwrite: true },
    );
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
