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
}

export const useServerStore = defineStore("server", () => {
  const profiles = ref<ServerProfile[]>([]);
  const activeServerId = ref<string | null>(null);
  const connectionStatus = ref<Record<string, "connected" | "error" | "unknown">>({});

  const activeServer = computed(() =>
    profiles.value.find((p) => p.id === activeServerId.value) ?? null
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
      return result;
    } catch (e) {
      connectionStatus.value[profile.id] = "error";
      throw e;
    }
  }

  function setActiveServer(id: string) {
    activeServerId.value = id;
  }

  return {
    profiles,
    activeServerId,
    activeServer,
    connectionStatus,
    loadProfiles,
    saveProfile,
    deleteProfile,
    testConnection,
    setActiveServer,
  };
});
