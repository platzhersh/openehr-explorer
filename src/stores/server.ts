import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Public profile returned from the backend — secrets are NOT included. */
export interface ServerProfile {
  id: string;
  name: string;
  base_url: string;
  server_type: "ehrbase" | "better_platform" | "ferro_ehr" | "generic";
  auth_method:
    | { type: "none" }
    | { type: "basic"; username: string; has_password: boolean }
    | { type: "bearer"; has_token: boolean };
  admin_auth_method?:
    | { type: "none" }
    | { type: "basic"; username: string; has_password: boolean }
    | { type: "bearer"; has_token: boolean }
    | null;
  terminology_url?: string | null;
  credential_backend: string;
  is_default: boolean;
}

/** Input profile sent to the backend during save — includes credential values. */
export interface ServerProfileInput {
  id: string;
  name: string;
  base_url: string;
  server_type: "ehrbase" | "better_platform" | "ferro_ehr" | "generic";
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
      const defaultProfile = profiles.value.find((p) => p.is_default);
      activeServerId.value = defaultProfile?.id ?? profiles.value[0].id;
    }
  }

  async function saveProfile(profile: ServerProfileInput) {
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

  async function testConnection(profileId: string): Promise<string> {
    try {
      const result = await invoke<string>("test_server_connection", { profileId });
      connectionStatus.value[profileId] = "connected";
      fetchServerVersion(profileId);

      // Fire a coarse-grained analytics event recording only the CDR platform
      // type — never the URL, host, or credentials. Lazy-imported to avoid a
      // cycle between the server store and the analytics composable.
      const profile = profiles.value.find((p) => p.id === profileId);
      if (profile) {
        const { useAnalytics } = await import("../composables/useAnalytics");
        void useAnalytics().track("server_connected", {
          server_type: profile.server_type,
        });
      }

      return result;
    } catch (e) {
      connectionStatus.value[profileId] = "error";
      throw e;
    }
  }

  async function testUnsavedConnection(profile: ServerProfileInput): Promise<string> {
    const result = await invoke<string>("test_unsaved_connection", { profile });
    return result;
  }

  function setActiveServer(id: string) {
    activeServerId.value = id;
  }

  async function setDefaultProfile(id: string) {
    profiles.value = await invoke<ServerProfile[]>("set_default_server_profile", { id });
  }

  async function fetchServerVersion(profileId: string): Promise<ServerVersionInfo | null> {
    try {
      const version = await invoke<ServerVersionInfo>("get_server_version", { profileId });
      versionInfo.value[profileId] = version;
      return version;
    } catch (e) {
      console.error("Failed to fetch server version:", e);
      return null;
    }
  }

  // Every backend HTTP call goes through `send_instrumented`, which emits a
  // `cdr-inspector-entry` event (url + status) regardless of which command
  // triggered it — see src-tauri/src/inspector.rs. We piggyback on that same
  // event to keep the sidebar connection indicator live: any successful
  // response marks its server "connected", any 4xx marks it "error", without
  // requiring every command to separately report connection health.
  function updateConnectionFromRequest(url: string, status: number) {
    const stripTrailingSlash = (s: string) => s.replace(/\/+$/, "");

    // Match against the longest base_url prefix so two profiles that share a
    // host (but differ by path) resolve to the more specific one.
    let matched: ServerProfile | null = null;
    let matchedBaseLength = -1;
    for (const profile of profiles.value) {
      const base = stripTrailingSlash(profile.base_url);
      if (base && url.startsWith(base) && base.length > matchedBaseLength) {
        matched = profile;
        matchedBaseLength = base.length;
      }
    }
    if (!matched) return;

    if (status >= 400 && status < 500) {
      connectionStatus.value[matched.id] = "error";
    } else if (status < 400) {
      connectionStatus.value[matched.id] = "connected";
    }
  }

  let unlistenRequests: UnlistenFn | null = null;

  async function startTrackingRequests() {
    if (unlistenRequests) return;
    unlistenRequests = await listen<{ url: string; status: number }>(
      "cdr-inspector-entry",
      (event) => {
        updateConnectionFromRequest(event.payload.url, event.payload.status);
      },
    );
  }

  function stopTrackingRequests() {
    if (unlistenRequests) {
      unlistenRequests();
      unlistenRequests = null;
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
    testUnsavedConnection,
    setActiveServer,
    setDefaultProfile,
    fetchServerVersion,
    startTrackingRequests,
    stopTrackingRequests,
  };
});
