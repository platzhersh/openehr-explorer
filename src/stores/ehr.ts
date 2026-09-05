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
  // False when the CDR rejected the requested sort (e.g. EHRBase doesn't
  // implement ORDER BY on EHR-level attributes) and list_ehrs fell back to
  // an unsorted query. See sort_field_path/is_order_by_unsupported in
  // src-tauri/src/commands/ehr.rs.
  sort_applied: boolean;
  // True if there is at least one more EHR beyond this page — computed
  // server-side by fetching one extra row past `limit` (see list_ehrs in
  // src-tauri/src/commands/ehr.rs). Drives whether "Next" is enabled.
  has_more: boolean;
}

// Whitelisted server-side (AQL ORDER BY) sort fields for the EHR list — must
// match the fields accepted by `sort_field_path` in src-tauri/src/commands/ehr.rs.
export type EhrSortField = "time_created" | "ehr_id" | "system_id";
export type SortDir = "asc" | "desc";

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

// DIRECTORY revision history (OEH-46) — mirrors the Rust `RevisionCommitAudit`/
// `RevisionHistoryEntry` structs in src-tauri/src/commands/ehr.rs (shared
// with the Status History tab's EHR_STATUS revision history, OEH-47), which
// parse the `versioned_directory/revision_history` response the same way
// `composition::get_composition_versions` parses a composition's.
export interface CommitAudit {
  change_type: string | null;
  committer_name: string | null;
  time_committed: string | null;
  description: string | null;
}

export interface DirectoryRevision {
  version_id: string;
  preceding_version_uid: string | null;
  commit_audit: CommitAudit | null;
  time_committed: string | null;
}

export const useEhrStore = defineStore("ehr", () => {
  const ehrs = ref<EhrSummary[]>([]);
  const total = ref(0);
  const offset = ref(0);
  const limit = ref(20);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const selectedEhr = ref<EhrDetail | null>(null);

  // Sort state for the paginated (non-search) EHR list — sorted server-side
  // via AQL ORDER BY (see list_ehrs in src-tauri/src/commands/ehr.rs).
  // Defaults match the app's historical default ordering: newest first.
  const sortBy = ref<EhrSortField>("time_created");
  const sortDir = ref<SortDir>("desc");
  // False when the server most recently rejected the requested sort (e.g.
  // EHRBase doesn't implement ORDER BY on EHR-level attributes) and the
  // list shown is actually unsorted. Starts true so no banner flashes
  // before the first fetch resolves.
  const sortApplied = ref(true);
  // Mirrors EhrListResponse.has_more from the most recent fetchEhrs — true
  // when another page exists after the one currently shown. Starts true so
  // "Next" isn't briefly shown as disabled before the first fetch resolves.
  const hasMore = ref(true);

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

  // DIRECTORY revision history + point-in-time/version preview (OEH-46).
  // `directory` above always tracks the *current* version (and is what
  // Edit/Create/Delete Directory act on); a preview is a separate, inert
  // snapshot the user asked to look at — it's never written back and never
  // clobbers `directory`, so leaving the preview open can't interfere with
  // editing the live directory.
  const directoryRevisionHistory = ref<DirectoryRevision[]>([]);
  const directoryRevisionHistoryLoading = ref(false);
  const directoryRevisionHistoryError = ref<string | null>(null);
  // Same "distinguish a real empty result from never-fetched" role as
  // `directoryLoaded` above, for the exact same reason: an EHR whose
  // DIRECTORY has no revision history at all is a normal, stable result —
  // without this, `toggleDirectoryHistoryPanel`'s "already have data, skip
  // fetching" gate (keyed off `.length === 0`) would refetch on every panel
  // reopen for such an EHR, since an empty array looks identical to one that
  // was never populated. Left false after a failure so the next reselect
  // retries, same as `directoryLoaded`.
  const directoryRevisionHistoryLoaded = ref(false);

  const directoryVersionPreview = ref<Record<string, unknown> | null>(null);
  const directoryVersionPreviewLoading = ref(false);
  const directoryVersionPreviewError = ref<string | null>(null);

  // Search state (PRD-0013)
  const searchResults = ref<EhrSearchResult[]>([]);
  const searchActive = ref(false);
  const searchLoading = ref(false);
  const searchError = ref<string | null>(null);
  const searchLimitReached = ref(false);

  // Bumped on every fetchEhrs call so a slow response for a since-superseded
  // request (the user changed page, sort field, or sort direction again
  // before the previous request came back) can't land after a newer one and
  // overwrite the list with stale — e.g. wrongly ordered — data. Same
  // pattern as searchRequestId/directoryRequestId below.
  let fetchRequestId = 0;

  async function fetchEhrs(serverId: string, page = 0) {
    const requestId = ++fetchRequestId;
    loading.value = true;
    error.value = null;
    try {
      const result = await invoke<EhrListResponse>("list_ehrs", {
        serverId,
        offset: page * limit.value,
        limit: limit.value,
        sortBy: sortBy.value,
        sortDir: sortDir.value,
      });
      if (requestId !== fetchRequestId) return; // superseded by a newer request
      ehrs.value = result.ehrs;
      total.value = result.total;
      offset.value = result.offset;
      sortApplied.value = result.sort_applied;
      hasMore.value = result.has_more;
    } catch (e) {
      if (requestId !== fetchRequestId) return;
      error.value = String(e);
    } finally {
      if (requestId === fetchRequestId) loading.value = false;
    }
  }

  /** Changes the sort field for the paginated EHR list and re-fetches page 0.
   *  Direction is left as-is — changing field doesn't guess a "natural"
   *  default direction per field, it just re-sorts the same way. */
  async function setSortBy(serverId: string, field: EhrSortField) {
    sortBy.value = field;
    await fetchEhrs(serverId, 0);
  }

  /** Flips asc/desc for the current sort field and re-fetches page 0. */
  async function toggleSortDir(serverId: string) {
    sortDir.value = sortDir.value === "asc" ? "desc" : "asc";
    await fetchEhrs(serverId, 0);
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

  /** Drops the cached revision history without touching anything else — for
   *  a mutation (save/delete) that happens while the "Version history" panel
   *  is closed. `fetchDirectoryRevisionHistory`'s own gate is "already
   *  loaded, skip fetching" (see `toggleDirectoryHistoryPanel` in
   *  EhrBrowser.vue), so leaving a stale list — and `directoryRevisionHistoryLoaded`
   *  still true — in place after a mutation would make the next panel-open
   *  silently show outdated revisions instead of fetching fresh ones. */
  function invalidateDirectoryRevisionHistory() {
    directoryHistoryRequestId++; // invalidate any in-flight fetch too
    directoryRevisionHistory.value = [];
    directoryRevisionHistoryLoaded.value = false;
  }

  function clearDirectory() {
    directoryRequestId++; // invalidate any in-flight fetchDirectory call
    directory.value = null;
    directoryError.value = null;
    directoryLoading.value = false;
    directoryLoaded.value = false;
    directoryHistoryRequestId++; // invalidate any in-flight fetchDirectoryRevisionHistory call
    directoryRevisionHistory.value = [];
    directoryRevisionHistoryLoading.value = false;
    directoryRevisionHistoryError.value = null;
    directoryRevisionHistoryLoaded.value = false;
    clearDirectoryVersionPreview();
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

  // Bumped by clearDirectory() so a slow response for a since-abandoned
  // EHR/server can't land after a newer request and overwrite the history
  // list — same pattern as directoryRequestId above.
  let directoryHistoryRequestId = 0;

  /** Fetches the DIRECTORY's full revision history (every version ever
   *  committed for this EHR), for the "Version history" panel. Independent
   *  of `fetchDirectory` — an empty history (never-created DIRECTORY) is a
   *  normal result, not an error, matching `get_directory_versions` on the
   *  Rust side (the same command the reconstructed Contributions tab uses,
   *  see EhrBrowser.vue — OEH-47). */
  async function fetchDirectoryRevisionHistory(serverId: string, ehrId: string) {
    const requestId = ++directoryHistoryRequestId;
    directoryRevisionHistoryLoading.value = true;
    directoryRevisionHistoryError.value = null;
    try {
      const result = await invoke<DirectoryRevision[]>("get_directory_versions", {
        serverId,
        ehrId,
      });
      if (requestId !== directoryHistoryRequestId) return; // superseded by a newer request
      directoryRevisionHistory.value = result;
      directoryRevisionHistoryLoaded.value = true;
    } catch (e) {
      if (requestId !== directoryHistoryRequestId) return;
      directoryRevisionHistory.value = [];
      directoryRevisionHistoryError.value = String(e);
      // directoryRevisionHistoryLoaded stays false — a later reselect retries.
    } finally {
      if (requestId === directoryHistoryRequestId) directoryRevisionHistoryLoading.value = false;
    }
  }

  // Shared by both preview functions below (rather than one counter each)
  // since they write to the same `directoryVersionPreview` state — whichever
  // of "preview this version" / "preview at this time" was asked for most
  // recently should win, regardless of which function it came through.
  let directoryPreviewRequestId = 0;

  /** Previews one historical DIRECTORY version by its version UID (from
   *  `directoryRevisionHistory`), without touching the live `directory`. */
  async function previewDirectoryVersion(serverId: string, ehrId: string, versionUid: string) {
    const requestId = ++directoryPreviewRequestId;
    directoryVersionPreviewLoading.value = true;
    directoryVersionPreviewError.value = null;
    try {
      const result = await invoke<Record<string, unknown> | null>("get_directory_version", {
        serverId,
        ehrId,
        versionUid,
      });
      if (requestId !== directoryPreviewRequestId) return; // superseded by a newer request
      directoryVersionPreview.value = result;
    } catch (e) {
      if (requestId !== directoryPreviewRequestId) return;
      directoryVersionPreview.value = null;
      directoryVersionPreviewError.value = String(e);
    } finally {
      if (requestId === directoryPreviewRequestId) directoryVersionPreviewLoading.value = false;
    }
  }

  /** Previews the DIRECTORY as it stood at a given instant (ISO 8601,
   *  interpreted as UTC by the server), without touching the live
   *  `directory`. */
  async function previewDirectoryAtTime(serverId: string, ehrId: string, versionAtTime: string) {
    const requestId = ++directoryPreviewRequestId;
    directoryVersionPreviewLoading.value = true;
    directoryVersionPreviewError.value = null;
    try {
      const result = await invoke<Record<string, unknown> | null>("get_directory", {
        serverId,
        ehrId,
        versionAtTime,
      });
      if (requestId !== directoryPreviewRequestId) return; // superseded by a newer request
      directoryVersionPreview.value = result;
    } catch (e) {
      if (requestId !== directoryPreviewRequestId) return;
      directoryVersionPreview.value = null;
      directoryVersionPreviewError.value = String(e);
    } finally {
      if (requestId === directoryPreviewRequestId) directoryVersionPreviewLoading.value = false;
    }
  }

  function clearDirectoryVersionPreview() {
    directoryPreviewRequestId++; // invalidate any in-flight preview call
    directoryVersionPreview.value = null;
    directoryVersionPreviewError.value = null;
    directoryVersionPreviewLoading.value = false;
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
    sortBy,
    sortDir,
    sortApplied,
    hasMore,
    loading,
    error,
    selectedEhr,
    directory,
    directoryLoading,
    directoryError,
    directoryLoaded,
    directoryRevisionHistory,
    directoryRevisionHistoryLoading,
    directoryRevisionHistoryError,
    directoryRevisionHistoryLoaded,
    directoryVersionPreview,
    directoryVersionPreviewLoading,
    directoryVersionPreviewError,
    searchResults,
    searchActive,
    searchLoading,
    searchError,
    searchLimitReached,
    fetchEhrs,
    setSortBy,
    toggleSortDir,
    fetchEhrDetail,
    fetchDirectory,
    clearDirectory,
    createDirectory,
    updateDirectory,
    deleteDirectory,
    fetchDirectoryRevisionHistory,
    invalidateDirectoryRevisionHistory,
    previewDirectoryVersion,
    previewDirectoryAtTime,
    clearDirectoryVersionPreview,
    searchEhrs,
    clearSearch,
    createEhr,
    updateEhrStatus,
    deleteEhr,
  };
});
