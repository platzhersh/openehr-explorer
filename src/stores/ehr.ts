import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface EhrSummary {
  ehr_id: string;
  system_id: string | null;
  time_created: string | null;
  subject_id: string | null;
}

export interface CompositionSummary {
  uid: string;
  template_id: string | null;
  name: string | null;
  composer: string | null;
  time_committed: string | null;
}

export interface EhrDetail {
  ehr_id: string;
  system_id: string | null;
  time_created: string | null;
  is_modifiable: boolean | null;
  is_queryable: boolean | null;
  subject_id: string | null;
  subject_namespace: string | null;
  compositions: CompositionSummary[];
}

export interface EhrListResponse {
  ehrs: EhrSummary[];
  total: number;
  offset: number;
  limit: number;
}

export interface EhrSearchCriteria {
  ehr_id_prefix?: string;
  subject_id?: string;
  subject_namespace?: string;
  system_id?: string;
  modifiable?: boolean;
  has_compositions?: boolean;
  created_on?: string;
  created_before?: string;
  created_after?: string;
}

export interface EhrSearchResult {
  ehr_id: string;
  time_created: string | null;
  subject_id: string | null;
  subject_namespace: string | null;
  is_modifiable: boolean | null;
  is_queryable: boolean | null;
  system_id: string | null;
}

export interface EhrSearchResponse {
  results: EhrSearchResult[];
  total: number;
  limit_reached: boolean;
}

export const useEhrStore = defineStore("ehr", () => {
  const ehrs = ref<EhrSummary[]>([]);
  const total = ref(0);
  const offset = ref(0);
  const limit = ref(20);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const selectedEhr = ref<EhrDetail | null>(null);

  // DIRECTORY state (OEH-27) — the FOLDER/OBJECT_REF tree is arbitrary-depth
  // and data-driven, so it's kept as raw JSON rather than a typed interface,
  // matching how `composition.ts` handles the composition body.
  const directory = ref<Record<string, unknown> | null>(null);
  const directoryLoading = ref(false);
  const directoryError = ref<string | null>(null);

  // Search state (PRD-0013)
  const searchResults = ref<EhrSearchResult[]>([]);
  const searchActive = ref(false);
  const searchLoading = ref(false);
  const searchError = ref<string | null>(null);
  const searchLimitReached = ref(false);

  async function fetchEhrs(serverId: string, page = 0) {
    loading.value = true;
    error.value = null;
    try {
      const result = await invoke<EhrListResponse>("list_ehrs", {
        serverId,
        offset: page * limit.value,
        limit: limit.value,
      });
      ehrs.value = result.ehrs;
      total.value = result.total;
      offset.value = result.offset;
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function fetchEhrDetail(serverId: string, ehrId: string) {
    loading.value = true;
    error.value = null;
    try {
      selectedEhr.value = await invoke<EhrDetail>("get_ehr_detail", {
        serverId,
        ehrId,
      });
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function createEhr(
    serverId: string,
    request: {
      subject_namespace?: string;
      subject_id?: string;
      is_queryable?: boolean;
      is_modifiable?: boolean;
      ehr_id?: string;
    },
  ) {
    loading.value = true;
    error.value = null;
    try {
      const result = await invoke<{
        ehr_id: string;
        system_id: string | null;
        time_created: string | null;
      }>("create_ehr", {
        serverId,
        request,
      });
      return result;
    } catch (e) {
      error.value = String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function updateEhrStatus(
    serverId: string,
    ehrId: string,
    request: {
      is_queryable: boolean;
      is_modifiable: boolean;
      subject_namespace?: string;
      subject_id?: string;
    },
  ) {
    loading.value = true;
    error.value = null;
    try {
      await invoke<string>("update_ehr_status", {
        serverId,
        ehrId,
        request,
      });
    } catch (e) {
      error.value = String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function deleteEhr(serverId: string, ehrId: string) {
    loading.value = true;
    error.value = null;
    try {
      await invoke<string>("delete_ehr", {
        serverId,
        ehrId,
      });
    } catch (e) {
      error.value = String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function searchEhrs(serverId: string, criteria: EhrSearchCriteria) {
    searchLoading.value = true;
    searchError.value = null;
    searchActive.value = true;
    try {
      const result = await invoke<EhrSearchResponse>("search_ehrs", {
        serverId,
        criteria,
      });
      searchResults.value = result.results;
      searchLimitReached.value = result.limit_reached;
    } catch (e) {
      searchError.value = String(e);
    } finally {
      searchLoading.value = false;
    }
  }

  async function fetchDirectory(serverId: string, ehrId: string, versionAtTime?: string) {
    directoryLoading.value = true;
    directoryError.value = null;
    try {
      directory.value = await invoke<Record<string, unknown>>("get_directory", {
        serverId,
        ehrId,
        versionAtTime: versionAtTime ?? null,
      });
    } catch (e) {
      // A 404 (no DIRECTORY set for this EHR) is expected and common — the
      // view treats this the same as any other fetch failure by showing the
      // error message, rather than crashing or silently hiding the tab.
      directory.value = null;
      directoryError.value = String(e);
    } finally {
      directoryLoading.value = false;
    }
  }

  function clearDirectory() {
    directory.value = null;
    directoryError.value = null;
    directoryLoading.value = false;
  }

  function clearSearch() {
    searchResults.value = [];
    searchActive.value = false;
    searchError.value = null;
    searchLimitReached.value = false;
  }

  return {
    ehrs,
    total,
    offset,
    limit,
    loading,
    error,
    selectedEhr,
    directory,
    directoryLoading,
    directoryError,
    searchResults,
    searchActive,
    searchLoading,
    searchError,
    searchLimitReached,
    fetchEhrs,
    fetchEhrDetail,
    fetchDirectory,
    clearDirectory,
    searchEhrs,
    clearSearch,
    createEhr,
    updateEhrStatus,
    deleteEhr,
  };
});
