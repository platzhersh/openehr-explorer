import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface DashboardCounts {
  ehr_count: number;
  composition_count: number;
  template_count: number;
}

export const useDashboardStore = defineStore("dashboard", () => {
  const counts = ref<DashboardCounts | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  // Bumped on every fetch so a slow response for a since-abandoned server
  // (the user switched servers before this one came back) can't land after
  // a newer request and overwrite counts that no longer match the active
  // server — same pattern as ehr.ts's searchRequestId.
  let requestId = 0;

  async function fetchCounts(serverId: string) {
    const id = ++requestId;
    loading.value = true;
    error.value = null;
    try {
      const result = await invoke<DashboardCounts>("get_dashboard_counts", { serverId });
      if (id !== requestId) return; // superseded by a newer request
      counts.value = result;
    } catch (e) {
      if (id !== requestId) return;
      error.value = String(e);
      counts.value = null;
    } finally {
      if (id === requestId) loading.value = false;
    }
  }

  function clear() {
    requestId++; // invalidate any in-flight fetchCounts call
    counts.value = null;
    error.value = null;
    loading.value = false;
  }

  return {
    counts,
    loading,
    error,
    fetchCounts,
    clear,
  };
});
