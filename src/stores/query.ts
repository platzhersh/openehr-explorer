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

// STORED_QUERY: queries registered on the CDR itself (PUT/GET /definition/query,
// execution via /query/{qualified_query_name}/{version}). Distinct from
// SavedQuery above, which is persisted locally per server profile.
export interface StoredQuerySummary {
  qualified_query_name: string;
  version: string | null;
  query_type: string | null;
  saved_time: string | null;
}

export interface StoredQueryDefinition {
  qualified_query_name: string;
  version: string | null;
  query_type: string | null;
  q: string | null;
  saved_time: string | null;
}

export const useQueryStore = defineStore("query", () => {
  const result = ref<AqlResult | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const savedQueries = ref<SavedQuery[]>([]);

  const storedQueries = ref<StoredQuerySummary[]>([]);
  const storedQueriesLoading = ref(false);
  const storedQueriesError = ref<string | null>(null);
  const selectedStoredQuery = ref<StoredQueryDefinition | null>(null);
  const selectedStoredQueryLoading = ref(false);
  const selectedStoredQueryError = ref<string | null>(null);

  // Revision counters guard against a slow request (e.g. for a server the
  // user has since switched away from) overwriting state from a newer one.
  let storedQueriesRevision = 0;
  let definitionRevision = 0;

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

  async function loadStoredQueries(serverId: string) {
    const revision = ++storedQueriesRevision;
    storedQueriesLoading.value = true;
    storedQueriesError.value = null;
    try {
      const result = await invoke<StoredQuerySummary[]>("list_stored_queries", { serverId });
      if (revision !== storedQueriesRevision) return; // superseded by a newer request
      storedQueries.value = result;
    } catch (e) {
      if (revision !== storedQueriesRevision) return;
      // Not every CDR implements STORED_QUERY — surface the error without
      // blocking the rest of the query runner.
      storedQueriesError.value = String(e);
      storedQueries.value = [];
    } finally {
      if (revision === storedQueriesRevision) storedQueriesLoading.value = false;
    }
  }

  async function loadStoredQueryDefinition(
    serverId: string,
    qualifiedQueryName: string,
    version?: string | null,
  ) {
    const revision = ++definitionRevision;
    selectedStoredQuery.value = null;
    selectedStoredQueryError.value = null;
    selectedStoredQueryLoading.value = true;
    try {
      const definition = await invoke<StoredQueryDefinition>("get_stored_query_definition", {
        serverId,
        qualifiedQueryName,
        version: version ?? null,
      });
      if (revision !== definitionRevision) return null; // superseded — discard
      selectedStoredQuery.value = definition;
      return definition;
    } catch (e) {
      if (revision !== definitionRevision) return null;
      selectedStoredQueryError.value = String(e);
      return null;
    } finally {
      if (revision === definitionRevision) selectedStoredQueryLoading.value = false;
    }
  }

  function clearSelectedStoredQuery() {
    definitionRevision++; // invalidate any definition request still in flight
    selectedStoredQuery.value = null;
    selectedStoredQueryError.value = null;
    selectedStoredQueryLoading.value = false;
  }

  async function executeStoredQuery(
    serverId: string,
    qualifiedQueryName: string,
    version: string | null,
    parameters: Record<string, unknown>,
  ) {
    loading.value = true;
    error.value = null;
    result.value = null;
    try {
      result.value = await invoke<AqlResult>("execute_stored_query", {
        serverId,
        qualifiedQueryName,
        version,
        parameters,
      });
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
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
    storedQueries,
    storedQueriesLoading,
    storedQueriesError,
    selectedStoredQuery,
    selectedStoredQueryLoading,
    selectedStoredQueryError,
    loadStoredQueries,
    loadStoredQueryDefinition,
    clearSelectedStoredQuery,
    executeStoredQuery,
  };
});
