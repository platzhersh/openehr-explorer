import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface GlobalSettings {
  version: number;
  terminology_server_url: string | null;
  check_updates_on_startup: boolean;
  analytics_enabled: boolean;
  analytics_consent_asked: boolean;
}

export const useSettingsStore = defineStore("settings", () => {
  const settings = ref<GlobalSettings>({
    version: 1,
    terminology_server_url: null,
    check_updates_on_startup: true,
    analytics_enabled: false,
    analytics_consent_asked: false,
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
