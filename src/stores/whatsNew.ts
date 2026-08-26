import { defineStore } from "pinia";
import { ref } from "vue";
import { useSettingsStore } from "./settings";
import { WHATS_NEW, getEntriesSince, type WhatsNewEntry } from "../lib/whats-new";

/**
 * Drives `WhatsNewModal.vue` — see PRD-0018.
 *
 * `checkForUpdate` is called once per launch (after settings load and any
 * analytics consent decision) with the running app version. It:
 *   - On a fresh install (`last_seen_version` is `null`), records the
 *     current version as the baseline and shows nothing — there is no
 *     "before" to compare against, and dumping the full history on a
 *     brand-new user is noise, not a feature announcement.
 *   - On an upgrade, shows every entry newer than `last_seen_version`.
 *   - No-ops if already on the last-seen version (same-version relaunch).
 */
export const useWhatsNewStore = defineStore("whatsNew", () => {
  const settingsStore = useSettingsStore();

  const visible = ref(false);
  const entries = ref<WhatsNewEntry[]>([]);

  function checkForUpdate(currentVersion: string) {
    const lastSeen = settingsStore.settings.last_seen_version;

    if (lastSeen === null) {
      void recordSeenVersion(currentVersion);
      return;
    }
    if (lastSeen === currentVersion || !settingsStore.settings.tours_enabled) return;

    const relevant = getEntriesSince(lastSeen);
    if (relevant.length === 0) {
      void recordSeenVersion(currentVersion);
      return;
    }
    entries.value = relevant;
    visible.value = true;
  }

  /** Reopen the modal on demand (Settings → "View What's New") with the latest entry, regardless of last-seen state. */
  function showLatest() {
    if (WHATS_NEW.length === 0) return;
    entries.value = [WHATS_NEW[0]];
    visible.value = true;
  }

  async function dismiss(currentVersion: string) {
    visible.value = false;
    await recordSeenVersion(currentVersion);
  }

  async function recordSeenVersion(version: string) {
    if (settingsStore.settings.last_seen_version === version) return;
    await settingsStore.saveSettings({
      ...settingsStore.settings,
      last_seen_version: version,
    });
  }

  return { visible, entries, checkForUpdate, showLatest, dismiss };
});
