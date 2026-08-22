import { defineStore } from "pinia";
import { ref, shallowRef } from "vue";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateCheckStatus = "idle" | "checking" | "up-to-date" | "available" | "error";

/**
 * Shared update-check state. Both the startup banner (`UpdateNotification.vue`)
 * and manual triggers (Settings page button, native "Check for Updates…" menu
 * item) go through this single store so there's one source of truth for
 * "is an update available" — triggering a check from any surface updates
 * every other surface too.
 */
export const useUpdateStore = defineStore("update", () => {
  // IMPORTANT: must be shallowRef, not ref. `Update` extends the Tauri
  // `Resource` class, which stores its `rid` in a WeakMap keyed by `this`
  // (TypeScript's private-field emulation). A plain `ref()` deep-wraps the
  // object in a reactive Proxy, so later calls like `update.value.rid` run
  // with `this` bound to the Proxy — a different object than the one used
  // as the WeakMap key — and throw "Cannot read private member from an
  // object whose class did not declare it". shallowRef keeps the raw
  // instance intact while still being reactive when `.value` is reassigned.
  const update = shallowRef<Update | null>(null);
  const status = ref<UpdateCheckStatus>("idle");
  const error = ref<string | null>(null);
  const dismissed = ref(false);

  const downloading = ref(false);
  const downloadedBytes = ref(0);
  const totalBytes = ref(0);

  /**
   * `Update` extends Tauri's `Resource`, which holds a native-side handle
   * that must be released explicitly via `.close()` — it isn't
   * garbage-collected. Best-effort: there's nothing useful to do if the
   * handle was already closed or the IPC call fails.
   */
  async function closeUpdateHandle(instance: Update | null) {
    if (!instance) return;
    try {
      await instance.close();
    } catch (err) {
      console.warn("Failed to close stale Update resource:", err);
    }
  }

  async function checkForUpdates() {
    status.value = "checking";
    error.value = null;
    try {
      const result = await check();
      // Every call constructs a fresh `Update` instance, so if one from a
      // previous check is still stored, it's now stale — close it before
      // replacing it (see `closeUpdateHandle`) so repeated manual checks
      // (Settings button, native menu item) don't leak native resources.
      const previous = update.value;
      if (result?.available) {
        update.value = result;
        status.value = "available";
        dismissed.value = false;
      } else {
        update.value = null;
        status.value = "up-to-date";
      }
      await closeUpdateHandle(previous);
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      status.value = "error";
    }
  }

  async function downloadAndInstall() {
    if (!update.value) return;
    downloading.value = true;
    error.value = null;
    downloadedBytes.value = 0;
    totalBytes.value = 0;

    try {
      await update.value.downloadAndInstall((event) => {
        if (event.event === "Started") {
          totalBytes.value = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloadedBytes.value += event.data.chunkLength;
        }
      });
      // On macOS/Linux the current binary has been replaced; relaunch to pick
      // up the new version. On Windows the NSIS installer takes over.
      await relaunch();
    } catch (err) {
      // Deliberately not closing `update.value` here: the UI currently has
      // no retry affordance while `error` is set (see UpdateNotification.vue),
      // so the instance is unreachable until the next `checkForUpdates()`
      // call, which closes it there instead (see `closeUpdateHandle`).
      error.value = err instanceof Error ? err.message : String(err);
      downloading.value = false;
    }
  }

  function dismiss() {
    dismissed.value = true;
  }

  return {
    update,
    status,
    error,
    dismissed,
    downloading,
    downloadedBytes,
    totalBytes,
    checkForUpdates,
    downloadAndInstall,
    dismiss,
  };
});
