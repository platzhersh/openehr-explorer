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
  // Whether the EHR has a DIRECTORY (FOLDER structure) set. Applied as a
  // post-filter server-side (AQL has no path for it) — see build_ehr_search_aql
  // / filter_by_directory_presence in src-tauri/src/commands/ehr.rs. Unlike
  // has_compositions, both true and false are supported.
  has_directory?: boolean;
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
  // True once a fetch has *succeeded* (populated or legitimately empty) for
  // the current EHR/server — distinct from `!!directory`, since "no
  // directory set" is itself a successful, stable result that shouldn't
  // trigger a re-fetch on every later tab reselect. Left false after a
  // failure so the next reselect retries instead of silently no-op'ing.
  const directoryLoaded = ref(false);

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

  // Bumped by clearSearch() so a slow response for a since-superseded search
  // (the user applied different filters, or removed a chip, before the
  // previous request came back) can't land after a newer one and overwrite
  // results that no longer match what's shown as the active criteria.
  let searchRequestId = 0;

  async function searchEhrs(serverId: string, criteria: EhrSearchCriteria) {
    const requestId = ++searchRequestId;
    searchLoading.value = true;
    searchError.value = null;
    searchActive.value = true;
    try {
      const result = await invoke<EhrSearchResponse>("search_ehrs", {
        serverId,
        criteria,
      });
      if (requestId !== searchRequestId) return; // superseded by a newer search
      searchResults.value = result.results;
      searchLimitReached.value = result.limit_reached;
    } catch (e) {
      if (requestId !== searchRequestId) return;
      searchError.value = String(e);
    } finally {
      if (requestId === searchRequestId) searchLoading.value = false;
    }
  }

  // Bumped by clearDirectory() (called whenever the selected EHR changes —
  // see EhrBrowser.vue) so a slow response for a since-abandoned EHR/server
  // can't land after a newer request and overwrite what's on screen.
  let directoryRequestId = 0;

  async function fetchDirectory(serverId: string, ehrId: string, versionAtTime?: string) {
    const requestId = ++directoryRequestId;
    directoryLoading.value = true;
    directoryError.value = null;
    try {
      // A 404 (no DIRECTORY set for this EHR — common, not an error) comes
      // back from the backend as `null`, not a thrown rejection.
      const result = await invoke<Record<string, unknown> | null>("get_directory", {
        serverId,
        ehrId,
        versionAtTime: versionAtTime ?? null,
      });
      if (requestId !== directoryRequestId) return; // superseded by a newer request
      directory.value = result;
      directoryLoaded.value = true;
    } catch (e) {
      if (requestId !== directoryRequestId) return;
      directory.value = null;
      directoryError.value = String(e);
      // directoryLoaded stays false — a later tab reselect should retry.
    } finally {
      if (requestId === directoryRequestId) directoryLoading.value = false;
    }
  }

  function clearDirectory() {
    directoryRequestId++; // invalidate any in-flight fetchDirectory call
    directory.value = null;
    directoryError.value = null;
    directoryLoading.value = false;
    directoryLoaded.value = false;
  }

  /** Creates the DIRECTORY for an EHR that doesn't have one yet. `folder` is
   *  DIRECTORY FOLDER RM JSON (see `toWireFolder` in `src/lib/directoryEdit.ts`).
   *  The backend re-fetches after writing, so `directory` ends up holding the
   *  server's canonical stored representation rather than what was sent. */
  async function createDirectory(serverId: string, ehrId: string, folder: Record<string, unknown>) {
    directoryRequestId++; // invalidate any in-flight fetchDirectory call
    directoryLoading.value = true;
    directoryError.value = null;
    try {
      directory.value = await invoke<Record<string, unknown>>("create_directory", {
        serverId,
        ehrId,
        folder,
      });
      directoryLoaded.value = true;
    } catch (e) {
      directoryError.value = String(e);
      throw e;
    } finally {
      directoryLoading.value = false;
    }
  }

  /** Replaces the DIRECTORY's FOLDER hierarchy. `precedingVersionUid` must be
   *  the `uid.value` of the version currently loaded (sent as `If-Match` so a
   *  concurrent change elsewhere isn't silently clobbered). */
  async function updateDirectory(
    serverId: string,
    ehrId: string,
    folder: Record<string, unknown>,
    precedingVersionUid: string,
  ) {
    directoryRequestId++;
    directoryLoading.value = true;
    directoryError.value = null;
    try {
      directory.value = await invoke<Record<string, unknown>>("update_directory", {
        serverId,
        ehrId,
        folder,
        precedingVersionUid,
      });
      directoryLoaded.value = true;
    } catch (e) {
      directoryError.value = String(e);
      throw e;
    } finally {
      directoryLoading.value = false;
    }
  }

  /** Deletes the DIRECTORY entirely. Same `precedingVersionUid` guard as
   *  `updateDirectory`. */
  async function deleteDirectory(serverId: string, ehrId: string, precedingVersionUid: string) {
    directoryRequestId++;
    directoryLoading.value = true;
    directoryError.value = null;
    try {
      await invoke<string>("delete_directory", {
        serverId,
        ehrId,
        precedingVersionUid,
      });
      directory.value = null;
      directoryLoaded.value = true; // "no directory" is now the confirmed, stable state
    } catch (e) {
      directoryError.value = String(e);
      throw e;
    } finally {
      directoryLoading.value = false;
    }
  }

  function clearSearch() {
    searchRequestId++; // invalidate any in-flight searchEhrs call
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
    directoryLoaded,
    searchResults,
    searchActive,
    searchLoading,
    searchError,
    searchLimitReached,
    fetchEhrs,
    fetchEhrDetail,
    fetchDirectory,
    clearDirectory,
    createDirectory,
    updateDirectory,
    deleteDirectory,
    searchEhrs,
    clearSearch,
    createEhr,
    updateEhrStatus,
    deleteEhr,
  };
});
