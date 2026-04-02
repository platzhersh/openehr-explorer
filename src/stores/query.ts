import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface AqlColumn {
  name: string;
  path: string | null;
}

export interface AqlResult {
  columns: AqlColumn[];
  rows: unknown[][];
  total_count: number;
  execution_time_ms: number;
}

export interface SavedQuery {
  id: string;
  name: string;
  query: string;
  server_id: string | null;
  created_at: string;
}

export const useQueryStore = defineStore("query", () => {
  const result = ref<AqlResult | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const savedQueries = ref<SavedQuery[]>([]);

  async function executeAql(serverId: string, query: string) {
    loading.value = true;
    error.value = null;
    result.value = null;
    try {
      result.value = await invoke<AqlResult>("execute_aql", { serverId, query });
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function loadSavedQueries(serverId?: string) {
    savedQueries.value = await invoke<SavedQuery[]>("list_saved_queries", {
      serverId: serverId ?? null,
    });
  }

  async function saveQuery(query: SavedQuery) {
    savedQueries.value = await invoke<SavedQuery[]>("save_query", { query });
  }

  async function deleteSavedQuery(id: string) {
    savedQueries.value = await invoke<SavedQuery[]>("delete_saved_query", { id });
  }

  return {
    result,
    loading,
    error,
    savedQueries,
    executeAql,
    loadSavedQueries,
    saveQuery,
    deleteSavedQuery,
  };
});
