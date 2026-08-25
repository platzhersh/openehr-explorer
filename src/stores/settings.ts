import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface GlobalSettings {
  version: number;
  terminology_server_url: string | null;
  check_updates_on_startup: boolean;
  analytics_enabled: boolean;
  analytics_consent_asked: boolean;
  /** Master toggle for auto-starting feature tours and the What's New modal. */
  tours_enabled: boolean;
  /** IDs of feature tours the user has completed or skipped — see `src/lib/tours.ts`. */
  completed_tours: string[];
  /** App version the What's New modal was last shown/acknowledged for. */
  last_seen_version: string | null;
}

export const useSettingsStore = defineStore("settings", () => {
  const settings = ref<GlobalSettings>({
    version: 1,
    terminology_server_url: null,
    check_updates_on_startup: true,
    analytics_enabled: false,
    analytics_consent_asked: false,
    tours_enabled: true,
    completed_tours: [],
    last_seen_version: null,
  });
  const loaded = ref(false);

  async function loadSettings() {
    settings.value = await invoke<GlobalSettings>("get_settings");
    loaded.value = true;
  }

  async function saveSettings(updated: GlobalSettings) {
    settings.value = await invoke<GlobalSettings>("save_settings", { settings: updated });
  }

  return {
    settings,
    loaded,
    loadSettings,
    saveSettings,
  };
});
