import { ref } from "vue";
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

export interface DownloadToastState {
  message: string;
  /** Absolute path of the saved file, so "Show in Folder" can reveal it. */
  path: string;
  isError: boolean;
}

const TOAST_DURATION_MS = 6000;

/**
 * Native "Save As" download flow: prompts for a destination via the OS save
 * dialog (instead of silently dropping the file into the browser-style
 * default downloads folder), writes the content, and surfaces a brief toast
 * confirming where it landed with a one-click "Show in Folder" action.
 *
 * Shared by any view that offers a plain-text/XML export (OPT download,
 * CSV export, ...) so they all get the same save-location + confirmation
 * behavior instead of each hand-rolling a Blob/anchor-click download.
 */
export function useFileDownload() {
  const toast = ref<DownloadToastState | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | null = null;

  function showToast(message: string, path: string, isError = false) {
    if (toastTimer) clearTimeout(toastTimer);
    toast.value = { message, path, isError };
    toastTimer = setTimeout(() => {
      toast.value = null;
    }, TOAST_DURATION_MS);
  }

  function dismissToast() {
    if (toastTimer) clearTimeout(toastTimer);
    toast.value = null;
  }

  async function revealDownload() {
    if (toast.value) {
      await revealItemInDir(toast.value.path);
    }
  }

  /**
   * Prompts the user for a save location, defaulting to `defaultFileName`,
   * writes `content` to it, and shows a success toast. Resolves to the
   * chosen path, or `null` if the user cancelled the dialog — callers
   * should treat cancellation as a normal, silent no-op rather than an
   * error.
   */
  async function saveTextFile(
    content: string,
    options: { defaultFileName: string; filterName: string; extensions: string[] },
  ): Promise<string | null> {
    const path = await save({
      defaultPath: options.defaultFileName,
      filters: [{ name: options.filterName, extensions: options.extensions }],
    });
    if (!path) return null;

    try {
      await writeTextFile(path, content);
    } catch (e) {
      showToast(`Failed to save ${options.defaultFileName}: ${String(e)}`, path, true);
      throw e;
    }

    showToast(`Saved ${options.defaultFileName}`, path);
    return path;
  }

  return { toast, dismissToast, revealDownload, saveTextFile };
}
