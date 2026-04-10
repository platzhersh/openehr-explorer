import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface ServerProfile {
  id: string;
  name: string;
  base_url: string;
  server_type: "ehrbase" | "better_platform" | "generic";
  auth_method:
    | { type: "none" }
    | { type: "basic"; username: string; password: string }
    | { type: "bearer"; token: string };
  admin_auth_method?:
    | { type: "none" }
    | { type: "basic"; username: string; password: string }
    | { type: "bearer"; token: string }
    | null;
  terminology_url?: string | null;
}

export interface ServerVersionInfo {
  server_version: string | null;
  ehrbase_version: string | null;
  sdk_version: string | null;
  archie_version: string | null;
  jvm_version: string | null;
  os_version: string | null;
  postgres_version: string | null;
}

export const useServerStore = defineStore("server", () => {
  const profiles = ref<ServerProfile[]>([]);
  const activeServerId = ref<string | null>(null);
  const connectionStatus = ref<Record<string, "connected" | "error" | "unknown">>({});
  const versionInfo = ref<Record<string, ServerVersionInfo>>({});

  const activeServer = computed(
    () => profiles.value.find((p) => p.id === activeServerId.value) ?? null,
  );

  async function loadProfiles() {
    profiles.value = await invoke<ServerProfile[]>("list_server_profiles");
    if (profiles.value.length > 0 && !activeServerId.value) {
      activeServerId.value = profiles.value[0].id;
    }
  }

  async function saveProfile(profile: ServerProfile) {
    profiles.value = await invoke<ServerProfile[]>("save_server_profile", { profile });
    if (!activeServerId.value) {
      activeServerId.value = profile.id;
    }
  }

  async function deleteProfile(id: string) {
    profiles.value = await invoke<ServerProfile[]>("delete_server_profile", { id });
    if (activeServerId.value === id) {
      activeServerId.value = profiles.value[0]?.id ?? null;
    }
  }

  async function testConnection(profile: ServerProfile): Promise<string> {
    try {
      const result = await invoke<string>("test_server_connection", { profile });
      connectionStatus.value[profile.id] = "connected";
      // Also fetch version info on successful connection test
      fetchServerVersion(profile);
      return result;
    } catch (e) {
      connectionStatus.value[profile.id] = "error";
      throw e;
    }
  }

  function setActiveServer(id: string) {
    activeServerId.value = id;
  }

  async function fetchServerVersion(profile: ServerProfile): Promise<ServerVersionInfo | null> {
    try {
      const version = await invoke<ServerVersionInfo>("get_server_version", { profile });
      versionInfo.value[profile.id] = version;
      return version;
    } catch (e) {
      console.error("Failed to fetch server version:", e);
      return null;
    }
  }

  return {
    profiles,
    activeServerId,
    activeServer,
    connectionStatus,
    versionInfo,
    loadProfiles,
    saveProfile,
    deleteProfile,
    testConnection,
    setActiveServer,
    fetchServerVersion,
  };
});
