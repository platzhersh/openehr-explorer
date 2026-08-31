<script setup lang="ts">
import { ref, watch, computed, onMounted, provide } from "vue";
import { useRoute, useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { useServerStore } from "../stores/server";
import {
  useEhrStore,
  type CompositionSummary,
  type EhrSearchCriteria,
  type EhrSortField,
} from "../stores/ehr";
import { useAnalytics } from "../composables/useAnalytics";
import { useTourStore } from "../stores/tour";
import EhrCreateDialog from "../components/EhrCreateDialog.vue";
import EhrFilterModal from "../components/EhrFilterModal.vue";
import DirectoryTree from "../components/DirectoryTree.vue";
import DirectoryTreeEditor from "../components/DirectoryTreeEditor.vue";
import CompassIcon from "../components/CompassIcon.vue";
import FilterIcon from "../components/FilterIcon.vue";
import JsonViewer from "../components/JsonViewer.vue";
import CopyButton from "../components/CopyButton.vue";
import {
  addItem as addDirectoryItem,
  addSubfolder as addDirectorySubfolder,
  DIRECTORY_MUTATIONS_KEY,
  emptyFolder,
  fromWireFolder,
  getFolderAtPath,
  removeItem as removeDirectoryItem,
  removeSubfolder as removeDirectorySubfolder,
  toWireFolder,
  type EditableFolder,
} from "../lib/directoryEdit";

const route = useRoute();
const router = useRouter();
const serverStore = useServerStore();
const ehrStore = useEhrStore();
const analytics = useAnalytics();
const tourStore = useTourStore();

onMounted(() => {
  // Track that the user opened the EHR browser. No IDs, URLs, or counts — just
  // a coarse feature-adoption ping so we know the view is actually being used.
  void analytics.track("ehr_browsed");
});

function replayTour() {
  void analytics.track("tour_replayed", { tour_id: "ehrs" });
  tourStore.start("ehrs");
}
const searchQuery = ref("");
const currentPage = ref(0);
const showCreateDialog = ref(false);
const showDeleteDialog = ref(false);
const deleteConfirmText = ref("");
const deleting = ref(false);
const deleteError = ref<string | null>(null);
const activeTab = ref<
  "detail" | "directory" | "status" | "compositions" | "json" | "contributions"
>("detail");

// DIRECTORY create/update/delete (OEH-27 follow-up) — editing state for the
// Directory tab. `editableDirectory` is a plain-field working copy (see
// src/lib/directoryEdit.ts); it only replaces `ehrStore.directory` on save.
const directoryEditMode = ref(false);
const editableDirectory = ref<EditableFolder | null>(null);
const directorySaving = ref(false);
const directorySaveError = ref<string | null>(null);
const showDeleteDirectoryDialog = ref(false);
const deletingDirectory = ref(false);
const deleteDirectoryError = ref<string | null>(null);

// `DirectoryTreeEditor` (at any depth) injects this instead of mutating the
// `EditableFolder` it receives as a prop — every write is applied here, to
// the tree this view actually owns, addressed by `path` (see
// DIRECTORY_MUTATIONS_KEY in src/lib/directoryEdit.ts for the full "why").
provide(DIRECTORY_MUTATIONS_KEY, {
  renameFolder(path, name) {
    if (!editableDirectory.value) return;
    getFolderAtPath(editableDirectory.value, path).name = name;
  },
  renameItemId(path, key, id) {
    if (!editableDirectory.value) return;
    const item = getFolderAtPath(editableDirectory.value, path).items.find((it) => it.key === key);
    if (item) item.id = id;
  },
  addSubfolder(path) {
    if (!editableDirectory.value) return;
    addDirectorySubfolder(getFolderAtPath(editableDirectory.value, path));
  },
  addItem(path, compositionUid) {
    if (!editableDirectory.value) return;
    addDirectoryItem(getFolderAtPath(editableDirectory.value, path), compositionUid);
  },
  removeSubfolder(parentPath, key) {
    if (!editableDirectory.value) return;
    removeDirectorySubfolder(getFolderAtPath(editableDirectory.value, parentPath), key);
  },
  removeItem(parentPath, key) {
    if (!editableDirectory.value) return;
    removeDirectoryItem(getFolderAtPath(editableDirectory.value, parentPath), key);
  },
});

const contributionLookupUid = ref("");
const contributionLookupError = ref<string | null>(null);

// --- Status history tab (OEH-47) ---
// EHR_STATUS is a VERSIONED_OBJECT, same as compositions — this mirrors the
// composition Versions tab in CompositionViewer.vue: a revision-history
// table plus an "at a point in time" fetch and a version-document preview.
interface RevisionCommitAudit {
  change_type: string | null;
  committer_name: string | null;
  time_committed: string | null;
  description: string | null;
}
interface RevisionHistoryEntry {
  version_id: string;
  preceding_version_uid: string | null;
  commit_audit: RevisionCommitAudit | null;
  time_committed: string | null;
}
const statusVersions = ref<RevisionHistoryEntry[]>([]);
const statusVersionsLoading = ref(false);
const statusVersionsError = ref<string | null>(null);
// Same "confirmed vs. never attempted" distinction as directoryLoaded — an
// empty revision history is itself a stable, successful result.
const statusVersionsLoaded = ref(false);
const statusContributionError = ref<string | null>(null);

// The document currently shown in the preview panel — either a specific
// historical version (via "Open" on a revision row) or the version in
// effect "at a point in time" (via the date picker below).
const statusPreview = ref<Record<string, unknown> | null>(null);
const statusPreviewLabel = ref<string | null>(null);
const statusPreviewLoading = ref(false);
const statusPreviewError = ref<string | null>(null);
// `datetime-local` input value (no timezone) for the "at a point in time" picker.
const statusAtTimeInput = ref("");

function clearStatusHistory() {
  statusVersions.value = [];
  statusVersionsError.value = null;
  statusVersionsLoading.value = false;
  statusVersionsLoaded.value = false;
  statusContributionError.value = null;
  statusPreview.value = null;
  statusPreviewLabel.value = null;
  statusPreviewError.value = null;
}

async function fetchStatusVersions() {
  if (!serverStore.activeServerId || !ehrId.value) return;
  statusVersionsLoading.value = true;
  statusVersionsError.value = null;
  try {
    statusVersions.value = await invoke<RevisionHistoryEntry[]>("get_ehr_status_versions", {
      serverId: serverStore.activeServerId,
      ehrId: ehrId.value,
    });
    statusVersionsLoaded.value = true;
  } catch (e) {
    statusVersionsError.value = String(e);
  } finally {
    statusVersionsLoading.value = false;
  }
}

function selectStatusHistoryTab() {
  activeTab.value = "status";
  if (
    ehrId.value &&
    serverStore.activeServerId &&
    !statusVersionsLoaded.value &&
    !statusVersionsLoading.value
  ) {
    fetchStatusVersions();
  }
}

/** Loads a specific historical EHR_STATUS version into the preview panel. */
async function viewStatusVersion(versionId: string) {
  if (!serverStore.activeServerId || !ehrId.value) return;
  statusPreviewLoading.value = true;
  statusPreviewError.value = null;
  try {
    statusPreview.value = await invoke<Record<string, unknown> | null>("get_ehr_status_version", {
      serverId: serverStore.activeServerId,
      ehrId: ehrId.value,
      versionUid: versionId,
    });
    statusPreviewLabel.value = versionId;
  } catch (e) {
    statusPreviewError.value = String(e);
  } finally {
    statusPreviewLoading.value = false;
  }
}

/** Loads the EHR_STATUS version in effect at `statusAtTimeInput` into the
 *  preview panel — the "at a point in time" view. */
async function viewStatusAtTime() {
  if (!serverStore.activeServerId || !ehrId.value || !statusAtTimeInput.value) return;
  statusPreviewLoading.value = true;
  statusPreviewError.value = null;
  const isoTime = new Date(statusAtTimeInput.value).toISOString();
  try {
    statusPreview.value = await invoke<Record<string, unknown> | null>("get_ehr_status", {
      serverId: serverStore.activeServerId,
      ehrId: ehrId.value,
      versionAtTime: isoTime,
    });
    statusPreviewLabel.value = `At ${isoTime}`;
    if (statusPreview.value === null) {
      statusPreviewError.value = "No EHR_STATUS version was in effect at that time.";
    }
  } catch (e) {
    statusPreviewError.value = String(e);
  } finally {
    statusPreviewLoading.value = false;
  }
}

/** Resolves the CONTRIBUTION an EHR_STATUS version was committed as part of,
 *  then navigates to the Contribution viewer — same pattern as the
 *  composition Versions tab's "View Contribution". */
async function viewStatusContribution(versionId: string) {
  if (!serverStore.activeServerId || !ehrId.value) return;
  statusContributionError.value = null;
  try {
    const contributionUid = await invoke<string | null>("get_ehr_status_version_contribution", {
      serverId: serverStore.activeServerId,
      ehrId: ehrId.value,
      versionUid: versionId,
    });
    if (!contributionUid) {
      statusContributionError.value =
        "This server did not report a contribution reference for this version.";
      return;
    }
    router.push({
      name: "contribution",
      params: { ehrId: ehrId.value, contributionUid },
    });
  } catch (e) {
    statusContributionError.value = String(e);
  }
}

// --- Contributions tab reconstruction (OEH-47) ---
// openEHR has no "list contributions for an EHR" endpoint — only GET-by-UID
// (see the manual lookup form below) — so this reconstructs the table by
// unioning the revision histories of every VERSIONED_OBJECT under the EHR:
// every composition, EHR_STATUS, and DIRECTORY. Each entry carries a
// committer and timestamp; the CONTRIBUTION UID itself is resolved lazily,
// only when "View" is clicked, to avoid one extra request per row up front.
type ContributionSource = "Composition" | "EHR_STATUS" | "DIRECTORY";
interface ContributionRow {
  source: ContributionSource;
  sourceDetail: string | null;
  version_id: string;
  change_type: string | null;
  committer_name: string | null;
  time_committed: string | null;
}
const contributionRows = ref<ContributionRow[]>([]);
const contributionRowsLoading = ref(false);
const contributionRowsError = ref<string | null>(null);
const contributionRowsLoaded = ref(false);
// Set when at least one (but not all) of the revision-history sources below
// failed to load — the table still shows whatever succeeded, but the
// missing source(s) are named so it's clear the table is incomplete rather
// than genuinely empty for those sources.
const contributionRowsPartialWarning = ref<string | null>(null);

function clearContributions() {
  contributionRows.value = [];
  contributionRowsError.value = null;
  contributionRowsPartialWarning.value = null;
  contributionRowsLoading.value = false;
  contributionRowsLoaded.value = false;
  contributionLookupError.value = null;
}

function toContributionRow(
  source: ContributionSource,
  sourceDetail: string | null,
  entry: RevisionHistoryEntry,
): ContributionRow {
  return {
    source,
    sourceDetail,
    version_id: entry.version_id,
    change_type: entry.commit_audit?.change_type ?? null,
    committer_name: entry.commit_audit?.committer_name ?? null,
    time_committed: entry.time_committed,
  };
}

interface RevisionHistorySource {
  /** Human-readable name for the partial-failure warning, e.g. "DIRECTORY"
   *  or "Composition: Vital Signs". */
  label: string;
  source: ContributionSource;
  sourceDetail: string | null;
  promise: Promise<RevisionHistoryEntry[]>;
}

async function loadContributions() {
  if (!serverStore.activeServerId || !ehrId.value || !ehrStore.selectedEhr) return;
  const serverId = serverStore.activeServerId;
  const id = ehrId.value;
  const compositions = ehrStore.selectedEhr.compositions;

  contributionRowsLoading.value = true;
  contributionRowsError.value = null;
  contributionRowsPartialWarning.value = null;

  // Each source is fetched independently (Promise.allSettled, not
  // Promise.all) — a composition, EHR_STATUS, or DIRECTORY revision-history
  // request failing shouldn't discard every other history that succeeded.
  const sources: RevisionHistorySource[] = [
    {
      label: "EHR_STATUS",
      source: "EHR_STATUS",
      sourceDetail: null,
      promise: invoke<RevisionHistoryEntry[]>("get_ehr_status_versions", { serverId, ehrId: id }),
    },
    {
      label: "DIRECTORY",
      source: "DIRECTORY",
      sourceDetail: null,
      promise: invoke<RevisionHistoryEntry[]>("get_directory_versions", { serverId, ehrId: id }),
    },
    ...compositions.map((comp): RevisionHistorySource => {
      const detail = comp.template_id ?? comp.name ?? comp.uid;
      return {
        label: `Composition: ${detail}`,
        source: "Composition",
        sourceDetail: detail,
        promise: invoke<RevisionHistoryEntry[]>("get_composition_versions", {
          serverId,
          ehrId: id,
          versionedObjectUid: comp.uid.split("::")[0],
        }),
      };
    }),
  ];

  const settled = await Promise.allSettled(sources.map((s) => s.promise));

  const rows: ContributionRow[] = [];
  const failed: { label: string; reason: unknown }[] = [];
  settled.forEach((result, i) => {
    const { source, sourceDetail, label } = sources[i];
    if (result.status === "fulfilled") {
      for (const v of result.value) rows.push(toContributionRow(source, sourceDetail, v));
    } else {
      failed.push({ label, reason: result.reason });
    }
  });
  // Newest first — entries without a committed timestamp (shouldn't
  // normally happen) sort to the end rather than the top.
  rows.sort((a, b) => (b.time_committed ?? "").localeCompare(a.time_committed ?? ""));

  contributionRowsLoading.value = false;

  if (failed.length === sources.length) {
    // Every source failed — this isn't "no contributions", it's a real
    // failure, so report it as one (and leave contributionRowsLoaded false
    // so reselecting the tab retries, matching directoryLoaded's convention).
    contributionRowsError.value = String(failed[0].reason);
    return;
  }

  contributionRows.value = rows;
  contributionRowsLoaded.value = true;
  if (failed.length > 0) {
    contributionRowsPartialWarning.value = `Some sources failed to load and are missing from this table: ${failed.map((f) => f.label).join(", ")}.`;
  }
}

function selectContributionsTab() {
  activeTab.value = "contributions";
  if (
    ehrId.value &&
    serverStore.activeServerId &&
    !contributionRowsLoaded.value &&
    !contributionRowsLoading.value
  ) {
    loadContributions();
  }
}

/** Resolves and opens the CONTRIBUTION for one reconstructed row, using the
 *  version-contribution resolver that matches its source VERSIONED_OBJECT. */
async function viewContributionForRow(row: ContributionRow) {
  if (!serverStore.activeServerId || !ehrId.value) return;
  contributionRowsError.value = null;
  try {
    let contributionUid: string | null;
    if (row.source === "EHR_STATUS") {
      contributionUid = await invoke<string | null>("get_ehr_status_version_contribution", {
        serverId: serverStore.activeServerId,
        ehrId: ehrId.value,
        versionUid: row.version_id,
      });
    } else if (row.source === "DIRECTORY") {
      contributionUid = await invoke<string | null>("get_directory_version_contribution", {
        serverId: serverStore.activeServerId,
        ehrId: ehrId.value,
        versionUid: row.version_id,
      });
    } else {
      contributionUid = await invoke<string | null>("get_composition_version_contribution", {
        serverId: serverStore.activeServerId,
        ehrId: ehrId.value,
        versionedObjectUid: row.version_id.split("::")[0],
        versionUid: row.version_id,
      });
    }
    if (!contributionUid) {
      contributionRowsError.value =
        "This server did not report a contribution reference for this version.";
      return;
    }
    router.push({
      name: "contribution",
      params: { ehrId: ehrId.value, contributionUid },
    });
  } catch (e) {
    contributionRowsError.value = String(e);
  }
}

/** Rows bucketed by calendar day (UTC date portion of the ISO timestamp) for
 *  the lightweight activity chart — a simple count-per-day view of when
 *  contributions landed, not a replacement for the table above. */
const activityByDay = computed(() => {
  const counts = new Map<string, number>();
  for (const row of contributionRows.value) {
    if (!row.time_committed) continue;
    const day = row.time_committed.slice(0, 10);
    counts.set(day, (counts.get(day) ?? 0) + 1);
  }
  const days = [...counts.entries()].sort(([a], [b]) => a.localeCompare(b));
  const max = Math.max(1, ...days.map(([, count]) => count));
  return days.map(([day, count]) => ({ day, count, pct: Math.round((count / max) * 100) }));
});

const showFilterModal = ref(false);
const validationError = ref<string | null>(null);
const searchHistory = ref<string[]>([]);
const showHistory = ref(false);

// The single source of truth for "what filters are currently applied" —
// whether they came from typing colon-syntax into the search box or from
// building them in the Filters modal. Drives the removable filter chips
// below the search bar so either entry point stays visible/editable the
// same way.
const activeCriteria = ref<EhrSearchCriteria | null>(null);

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

const ehrId = computed(() => route.params.ehrId as string | undefined);

watch(
  () => serverStore.activeServerId,
  (id) => {
    if (id) {
      ehrStore.fetchEhrs(id, 0);
      currentPage.value = 0;
    }

    // The route's :ehrId doesn't change on a server switch, so without this
    // the DIRECTORY tab would keep showing data fetched from the
    // now-abandoned server for whatever EHR ID happens to still be selected.
    ehrStore.clearDirectory();
    cancelEditDirectory();
    if (activeTab.value === "directory" && ehrId.value && id) {
      ehrStore.fetchDirectory(id, ehrId.value);
    }

    // Same reasoning for the Status history and Contributions tabs — both
    // are EHR-scoped. Contributions isn't re-fetched here even if active:
    // it depends on the compositions list from fetchEhrDetail below, which
    // this watcher doesn't await, so re-fetching now would race and read
    // the still-stale list. The selectedEhr watcher further down handles it
    // once that list actually lands.
    clearStatusHistory();
    clearContributions();
    if (activeTab.value === "status" && ehrId.value && id) {
      fetchStatusVersions();
    }
  },
  { immediate: true },
);

watch(ehrId, (id) => {
  if (id && serverStore.activeServerId) {
    // If we have search results and the selected EHR is in them, pre-populate from search data.
    // Either way we still fetch full detail for compositions.
    ehrStore.fetchEhrDetail(serverStore.activeServerId, id);
  }

  // The DIRECTORY tab's data is EHR-scoped — reset it whenever the selected
  // EHR changes, and re-fetch for the new EHR if that tab is currently open.
  ehrStore.clearDirectory();
  cancelEditDirectory();
  if (activeTab.value === "directory" && id && serverStore.activeServerId) {
    ehrStore.fetchDirectory(serverStore.activeServerId, id);
  }

  // Status history / Contributions — see the server-switch watcher above for
  // why Contributions isn't re-fetched directly here.
  clearStatusHistory();
  clearContributions();
  if (activeTab.value === "status" && id && serverStore.activeServerId) {
    fetchStatusVersions();
  }
});

// Contributions reconstruction depends on the compositions list, which
// arrives asynchronously via fetchEhrDetail (triggered above) — this
// re-fetches once that list actually lands, if the Contributions tab is the
// one currently open.
watch(
  () => ehrStore.selectedEhr,
  () => {
    if (
      activeTab.value === "contributions" &&
      !contributionRowsLoaded.value &&
      !contributionRowsLoading.value
    ) {
      loadContributions();
    }
  },
);

const DATE_RE = /^\d{4}-\d{2}-\d{2}$/;

/** Parses a `true`/`false` token value for a boolean filter (modifiable:,
 *  hasCompositions:, hasDirectory:), assigning it into `criteria[field]` on
 *  success. Returns an error message on an invalid value, or null. Pulled
 *  out of `applyToken`'s switch so each boolean case there is a single
 *  return-call rather than its own nested if — see applyToken. */
function applyBooleanToken(
  criteria: EhrSearchCriteria,
  field: "modifiable" | "has_compositions" | "has_directory",
  prefix: string,
  value: string,
): string | null {
  if (value !== "true" && value !== "false") {
    return `${prefix}: expects 'true' or 'false'.`;
  }
  criteria[field] = value === "true";
  return null;
}

/** Same shape as applyBooleanToken, for the YYYY-MM-DD date filters. */
function applyDateToken(
  criteria: EhrSearchCriteria,
  field: "created_on" | "created_before" | "created_after",
  prefix: string,
  value: string,
): string | null {
  if (!DATE_RE.test(value)) {
    return `${prefix}: expects a date in YYYY-MM-DD format (e.g. 2026-03-12).`;
  }
  criteria[field] = value;
  return null;
}

/** Applies one whitespace-separated token to `criteria`, mutating it in
 *  place. Returns an error message on failure, or null on success —
 *  including for a bare token (no colon) or an unrecognized prefix, both of
 *  which fall back to extending `ehr_id_prefix`. */
function applyToken(criteria: EhrSearchCriteria, token: string): string | null {
  const colonIdx = token.indexOf(":");
  if (colonIdx === -1) {
    criteria.ehr_id_prefix = (criteria.ehr_id_prefix ?? "") + token;
    return null;
  }

  const prefix = token.substring(0, colonIdx);
  const value = token.substring(colonIdx + 1);
  if (!value) return `${prefix}: value cannot be empty.`;

  switch (prefix) {
    case "subject":
      criteria.subject_id = value;
      return null;
    case "namespace":
      criteria.subject_namespace = value;
      return null;
    case "system":
      criteria.system_id = value;
      return null;
    case "modifiable":
      return applyBooleanToken(criteria, "modifiable", "modifiable", value);
    case "hasCompositions":
      return applyBooleanToken(criteria, "has_compositions", "hasCompositions", value);
    case "hasDirectory":
      return applyBooleanToken(criteria, "has_directory", "hasDirectory", value);
    case "created-on":
      return applyDateToken(criteria, "created_on", "created-on", value);
    case "created-before":
      return applyDateToken(criteria, "created_before", "created-before", value);
    case "created-after":
      return applyDateToken(criteria, "created_after", "created-after", value);
    default:
      // Unknown prefix — treat as EHR ID prefix (safe fallback)
      criteria.ehr_id_prefix = (criteria.ehr_id_prefix ?? "") + token;
      return null;
  }
}

function parseSearchInput(raw: string): {
  criteria: EhrSearchCriteria | null;
  error: string | null;
  warning: string | null;
} {
  const trimmed = raw.trim();
  if (!trimmed) return { criteria: null, error: null, warning: null };

  const criteria: EhrSearchCriteria = {};
  for (const token of trimmed.split(/\s+/)) {
    const error = applyToken(criteria, token);
    if (error) return { criteria: null, error, warning: null };
  }

  // Conflict resolution: created_on overrides created_before/created_after
  let warning: string | null = null;
  if (criteria.created_on && (criteria.created_before || criteria.created_after)) {
    warning = "created-on overrides created-before/created-after. Only created-on is used.";
    delete criteria.created_before;
    delete criteria.created_after;
  }

  return { criteria, error: null, warning };
}

function executeSearch() {
  validationError.value = null;
  const raw = searchQuery.value.trim();

  if (!raw) {
    clearSearch();
    return;
  }

  const { criteria, error } = parseSearchInput(raw);

  if (error) {
    validationError.value = error;
    return;
  }

  if (!criteria) {
    clearSearch();
    return;
  }

  if (!serverStore.activeServerId) return;

  // Add to history
  if (!searchHistory.value.includes(raw)) {
    searchHistory.value.unshift(raw);
    if (searchHistory.value.length > 10) searchHistory.value.pop();
  }

  showHistory.value = false;
  runSearch(criteria);
}

/** Shared tail of every search path (text box, Filters modal, chip removal):
 *  records the applied criteria and fires the backend query. */
function runSearch(criteria: EhrSearchCriteria) {
  if (!serverStore.activeServerId) return;
  activeCriteria.value = criteria;
  // Feature-adoption ping only — never the query text itself or the raw
  // criteria. Users construct search queries with patient identifiers
  // encoded in the input, so the text is treated as PII and stays local.
  void analytics.track("ehr_searched");
  ehrStore.searchEhrs(serverStore.activeServerId, criteria);
}

/** Applies the structured criteria built in the Filters modal. The text box
 *  is cleared so it doesn't sit there showing a stale, out-of-sync string. */
function handleFilterModalApply(criteria: EhrSearchCriteria) {
  searchQuery.value = "";
  validationError.value = null;
  showFilterModal.value = false;
  runSearch(criteria);
}

/** Removes a single active filter chip and re-runs the search with what's
 *  left — or clears the search entirely if that was the last filter. */
function removeFilterChip(key: keyof EhrSearchCriteria) {
  if (!activeCriteria.value) return;
  const next = { ...activeCriteria.value };
  delete next[key];
  searchQuery.value = "";
  if (Object.keys(next).length === 0) {
    clearSearch();
  } else {
    runSearch(next);
  }
}

interface FilterChip {
  key: keyof EhrSearchCriteria;
  label: string;
}

/** `value ? "Yes" : "No"`, pulled out to a one-liner so filterChips' string
 *  templates can call it instead of embedding the ternary — that keeps each
 *  of filterChips' many `if` branches to a flat structural cost instead of
 *  a nested one, which otherwise pushes its Cognitive Complexity over the
 *  usual gate (SonarCloud flagged this at 17 for the original version). */
function yesNo(value: boolean): string {
  return value ? "Yes" : "No";
}

const filterChips = computed<FilterChip[]>(() => {
  const c = activeCriteria.value;
  if (!c) return [];
  const chips: FilterChip[] = [];
  if (c.ehr_id_prefix)
    chips.push({ key: "ehr_id_prefix", label: `ID starts with "${c.ehr_id_prefix}"` });
  if (c.subject_id) chips.push({ key: "subject_id", label: `Subject contains "${c.subject_id}"` });
  if (c.subject_namespace)
    chips.push({ key: "subject_namespace", label: `Namespace: ${c.subject_namespace}` });
  if (c.system_id) chips.push({ key: "system_id", label: `System: ${c.system_id}` });
  if (c.modifiable !== undefined)
    chips.push({ key: "modifiable", label: `Modifiable: ${yesNo(c.modifiable)}` });
  if (c.has_compositions !== undefined)
    chips.push({
      key: "has_compositions",
      label: `Has compositions: ${yesNo(c.has_compositions)}`,
    });
  if (c.has_directory !== undefined)
    chips.push({
      key: "has_directory",
      label: `Has directory entries: ${yesNo(c.has_directory)}`,
    });
  if (c.created_on) chips.push({ key: "created_on", label: `Created on ${c.created_on}` });
  if (c.created_before)
    chips.push({ key: "created_before", label: `Created before ${c.created_before}` });
  if (c.created_after)
    chips.push({ key: "created_after", label: `Created after ${c.created_after}` });
  return chips;
});

function onSearchKeydown(e: KeyboardEvent) {
  if (e.key === "Enter") {
    if (debounceTimer) clearTimeout(debounceTimer);
    executeSearch();
  }
}

function onSearchInput() {
  validationError.value = null;
  if (debounceTimer) clearTimeout(debounceTimer);
  if (!searchQuery.value.trim()) {
    clearSearch();
    return;
  }
  debounceTimer = setTimeout(() => {
    executeSearch();
  }, 600);
}

function clearSearch() {
  searchQuery.value = "";
  validationError.value = null;
  activeCriteria.value = null;
  ehrStore.clearSearch();
  if (serverStore.activeServerId) {
    ehrStore.fetchEhrs(serverStore.activeServerId, currentPage.value);
  }
}

function hideHistoryDelayed() {
  setTimeout(() => (showHistory.value = false), 200);
}

function selectHistoryItem(item: string) {
  searchQuery.value = item;
  showHistory.value = false;
  executeSearch();
}

function selectEhr(id: string) {
  router.push({ name: "ehr-detail", params: { ehrId: id } });
}

function openComposition(comp: CompositionSummary) {
  if (ehrId.value) {
    router.push({
      name: "composition",
      params: { ehrId: ehrId.value, compositionUid: comp.uid },
    });
  }
}

function selectDirectoryTab() {
  activeTab.value = "directory";
  // `directoryLoaded` (not `!!directory`) is what guards re-fetching: a
  // legitimate empty result ("no directory set") is a successful, stable
  // outcome that shouldn't be re-requested on every reselect, but a prior
  // failure leaves `directoryLoaded` false so the next reselect retries.
  if (
    ehrId.value &&
    serverStore.activeServerId &&
    !ehrStore.directoryLoaded &&
    !ehrStore.directoryLoading
  ) {
    ehrStore.fetchDirectory(serverStore.activeServerId, ehrId.value);
  }
}

function openCompositionRef(objectRef: { id?: { value?: string } }) {
  if (ehrId.value && objectRef.id?.value) {
    router.push({
      name: "composition",
      params: { ehrId: ehrId.value, compositionUid: objectRef.id.value },
    });
  }
}

// Compositions available to pick from in the "add item" dropdown while
// editing the DIRECTORY — sourced from the same list shown on the Detail tab.
const availableCompositionOptions = computed(() => {
  if (!ehrStore.selectedEhr) return [];
  return ehrStore.selectedEhr.compositions.map((comp) => ({
    uid: comp.uid,
    label: comp.name ?? comp.template_id ?? comp.uid,
  }));
});

function directoryVersionUid(): string | undefined {
  return (ehrStore.directory as { uid?: { value?: string } } | null)?.uid?.value;
}

function startCreateDirectory() {
  editableDirectory.value = emptyFolder("Root");
  directorySaveError.value = null;
  directoryEditMode.value = true;
}

function startEditDirectory() {
  if (!ehrStore.directory) return;
  editableDirectory.value = fromWireFolder(ehrStore.directory);
  directorySaveError.value = null;
  directoryEditMode.value = true;
}

function cancelEditDirectory() {
  directoryEditMode.value = false;
  editableDirectory.value = null;
  directorySaveError.value = null;
}

async function saveDirectory() {
  if (!editableDirectory.value || !serverStore.activeServerId || !ehrId.value) return;

  directorySaving.value = true;
  directorySaveError.value = null;
  const wireFolder = toWireFolder(editableDirectory.value);
  const existingVersionUid = directoryVersionUid();
  try {
    if (existingVersionUid) {
      await ehrStore.updateDirectory(
        serverStore.activeServerId,
        ehrId.value,
        wireFolder,
        existingVersionUid,
      );
      void analytics.track("directory_updated");
    } else {
      await ehrStore.createDirectory(serverStore.activeServerId, ehrId.value, wireFolder);
      void analytics.track("directory_created");
    }
    directoryEditMode.value = false;
    editableDirectory.value = null;
  } catch (e) {
    directorySaveError.value = String(e);
  } finally {
    directorySaving.value = false;
  }
}

function openDeleteDirectoryDialog() {
  deleteDirectoryError.value = null;
  showDeleteDirectoryDialog.value = true;
}

async function handleDeleteDirectory() {
  const existingVersionUid = directoryVersionUid();
  if (!serverStore.activeServerId || !ehrId.value || !existingVersionUid) return;

  deletingDirectory.value = true;
  deleteDirectoryError.value = null;
  try {
    await ehrStore.deleteDirectory(serverStore.activeServerId, ehrId.value, existingVersionUid);
    void analytics.track("directory_deleted");
    showDeleteDirectoryDialog.value = false;
  } catch (e) {
    deleteDirectoryError.value = String(e);
  } finally {
    deletingDirectory.value = false;
  }
}

function prevPage() {
  if (currentPage.value > 0 && serverStore.activeServerId) {
    currentPage.value--;
    ehrStore.fetchEhrs(serverStore.activeServerId, currentPage.value);
  }
}

function nextPage() {
  if (serverStore.activeServerId) {
    currentPage.value++;
    ehrStore.fetchEhrs(serverStore.activeServerId, currentPage.value);
  }
}

function refresh() {
  if (serverStore.activeServerId) {
    if (ehrStore.searchActive) {
      executeSearch();
    } else {
      ehrStore.fetchEhrs(serverStore.activeServerId, currentPage.value);
    }
  }
}

// Sort options for the paginated (non-search) EHR list — the AQL ORDER BY
// clause built server-side in list_ehrs handles the actual sorting.
const sortFieldOptions: { value: EhrSortField; label: string }[] = [
  { value: "time_created", label: "Created date" },
  { value: "ehr_id", label: "EHR ID" },
  { value: "system_id", label: "System ID" },
];

// Shown as a tooltip on the (disabled) sort controls when the active
// server rejected ORDER BY and list_ehrs fell back to an unsorted query
// (see ehrStore.sortApplied / sort_applied in src-tauri/src/commands/ehr.rs).
// All three fields hit the same EHR-level-attribute limitation, so there's
// no field left to offer instead — disabling avoids the user repeatedly
// triggering the same failed-request-then-fallback round trip.
const sortUnsupportedTitle = "Sorting isn't supported by this server";

function onSortFieldChange(field: string) {
  if (!serverStore.activeServerId) return;
  currentPage.value = 0;
  void ehrStore.setSortBy(serverStore.activeServerId, field as EhrSortField);
}

function onToggleSortDir() {
  if (!serverStore.activeServerId) return;
  currentPage.value = 0;
  void ehrStore.toggleSortDir(serverStore.activeServerId);
}

// --- Compositions tab (OEH-47) ---
// FerroEHR gives Compositions its own tab, filterable by template/composer/
// date range, separate from the EHR-metadata "Status" (our "Detail") tab.
// All filtering is client-side over the composition list get_ehr_detail
// already fetches in full — no new backend command, since that AQL query
// has no LIMIT to page around in the first place.
const compositionFilterTemplate = ref("");
const compositionFilterComposer = ref("");
const compositionFilterFrom = ref(""); // date input value, "YYYY-MM-DD"
const compositionFilterTo = ref("");

const compositionFiltersActive = computed(
  () =>
    !!(
      compositionFilterTemplate.value ||
      compositionFilterComposer.value ||
      compositionFilterFrom.value ||
      compositionFilterTo.value
    ),
);

function clearCompositionFilters() {
  compositionFilterTemplate.value = "";
  compositionFilterComposer.value = "";
  compositionFilterFrom.value = "";
  compositionFilterTo.value = "";
}

// Filtered compositions, grouped by template_id — the Compositions tab's
// list. Date filtering compares just the YYYY-MM-DD portion of
// time_committed against the (inclusive) from/to bounds, so a composition
// committed at any time during the "to" day is still included.
/** Whether `timeCommitted`'s date portion falls within the inclusive
 *  [from, to] bound — either side empty means unbounded on that side. A
 *  composition with no time_committed can never match a non-empty range. */
function compositionMatchesDateRange(
  timeCommitted: string | null,
  from: string,
  to: string,
): boolean {
  if (!from && !to) return true;
  const day = timeCommitted?.slice(0, 10);
  if (!day) return false;
  if (from && day < from) return false;
  return !(to && day > to);
}

/** All three Compositions-tab filters as a single predicate, pulled out of
 *  compositionsByTemplate's loop so that computed stays a flat filter+group
 *  instead of a nest of per-field conditionals (SonarCloud flagged the
 *  latter's Cognitive Complexity). */
function compositionMatchesFilters(
  comp: CompositionSummary,
  templateQuery: string,
  composerQuery: string,
  from: string,
  to: string,
): boolean {
  if (templateQuery && !(comp.template_id ?? "").toLowerCase().includes(templateQuery)) {
    return false;
  }
  if (composerQuery && !(comp.composer ?? "").toLowerCase().includes(composerQuery)) {
    return false;
  }
  return compositionMatchesDateRange(comp.time_committed, from, to);
}

const compositionsByTemplate = computed(() => {
  if (!ehrStore.selectedEhr) return {};
  const templateQuery = compositionFilterTemplate.value.trim().toLowerCase();
  const composerQuery = compositionFilterComposer.value.trim().toLowerCase();
  const from = compositionFilterFrom.value;
  const to = compositionFilterTo.value;

  const groups: Record<string, CompositionSummary[]> = {};
  for (const comp of ehrStore.selectedEhr.compositions) {
    if (!compositionMatchesFilters(comp, templateQuery, composerQuery, from, to)) continue;
    const key = comp.template_id ?? "(no template)";
    if (!groups[key]) groups[key] = [];
    groups[key].push(comp);
  }
  return groups;
});

// Total row count across all template groups — separate from
// compositionsByTemplate's own per-group counts, for the "Compositions (N)"
// header, which shows the filtered count (not the EHR's unfiltered total)
// once a filter narrows the list.
const filteredCompositionCount = computed(() =>
  Object.values(compositionsByTemplate.value).reduce((sum, comps) => sum + comps.length, 0),
);

function handleEhrCreated(newEhrId: string) {
  showCreateDialog.value = false;
  void analytics.track("ehr_created");
  refresh();
  // Navigate to the new EHR
  router.push({ name: "ehr-detail", params: { ehrId: newEhrId } });
}

async function handleDeleteEhr() {
  if (!serverStore.activeServerId || !ehrId.value) return;

  // Validate confirmation text
  if (deleteConfirmText.value !== ehrId.value) {
    return;
  }

  deleting.value = true;
  deleteError.value = null;
  try {
    await ehrStore.deleteEhr(serverStore.activeServerId, ehrId.value);
    showDeleteDialog.value = false;
    deleteConfirmText.value = "";
    void analytics.track("ehr_deleted");
    // Clear detail pane and navigate back to EHR list
    ehrStore.selectedEhr = null;
    router.push({ name: "ehrs" });
    // Refresh list
    refresh();
  } catch (e) {
    deleteError.value = String(e);
  } finally {
    deleting.value = false;
  }
}

function openDeleteDialog() {
  deleteConfirmText.value = "";
  deleteError.value = null;
  showDeleteDialog.value = true;
}

const canDelete = computed(() => {
  return deleteConfirmText.value === ehrId.value;
});

// CONTRIBUTION lookup (OEH-28). openEHR has no "list contributions for an
// EHR" endpoint — only GET-by-UID — so this is a manual lookup form. The
// composition Versions tab is the other, more common entry point: it
// resolves a version's contribution UID for you and jumps straight here.
function lookupContribution() {
  contributionLookupError.value = null;
  const uid = contributionLookupUid.value.trim();
  if (!uid) {
    contributionLookupError.value = "Enter a contribution UID.";
    return;
  }
  if (!ehrId.value) return;
  router.push({
    name: "contribution",
    params: { ehrId: ehrId.value, contributionUid: uid },
  });
}
</script>

<template>
  <div class="ehr-browser">
    <div class="panel-left">
      <div class="panel-header">
        <h2 v-if="ehrStore.searchActive">
          EHRs — Search Results ({{ ehrStore.searchResults.length }})
        </h2>
        <h2 v-else>EHRs</h2>
        <div class="header-actions">
          <button
            type="button"
            class="tour-trigger-btn"
            title="Take a tour of the EHR Browser"
            @click="replayTour"
          >
            <CompassIcon />
          </button>
          <button
            type="button"
            class="btn btn-sm btn-primary"
            data-tour="ehr-create"
            @click="showCreateDialog = true"
          >
            + New EHR
          </button>
          <button type="button" class="btn btn-sm" @click="refresh">Refresh</button>
        </div>
      </div>

      <div class="search-bar">
        <div class="search-input-wrapper">
          <input
            class="input search-input"
            data-tour="ehr-search"
            v-model="searchQuery"
            placeholder="Search by EHR ID, or click Filters for more options"
            autocapitalize="off"
            autocorrect="off"
            spellcheck="false"
            @keydown="onSearchKeydown"
            @input="onSearchInput"
            @focus="showHistory = searchHistory.length > 0 && !searchQuery"
            @blur="hideHistoryDelayed"
          />
          <button
            v-if="searchQuery"
            type="button"
            class="clear-btn"
            @click="clearSearch"
            title="Clear search"
          >
            &times;
          </button>
          <button
            type="button"
            class="filter-btn"
            data-tour="ehr-search-filters"
            @click="showFilterModal = true"
            title="Build filters — includes a shortcut-syntax reference"
          >
            <FilterIcon />
            <span v-if="filterChips.length" class="filter-count-badge">{{
              filterChips.length
            }}</span>
          </button>
        </div>

        <!-- Validation error -->
        <div v-if="validationError" class="search-validation-error">
          {{ validationError }}
        </div>

        <!-- Search error from backend -->
        <div v-if="ehrStore.searchError" class="search-validation-error">
          Search failed: {{ ehrStore.searchError }}
        </div>

        <!-- Active filter chips — works whether the filters came from the
             Filters modal or from typing colon-syntax into the box, since
             both write into the same `activeCriteria` state. -->
        <div v-if="filterChips.length" class="filter-chips">
          <span v-for="chip in filterChips" :key="chip.key" class="filter-chip">
            {{ chip.label }}
            <button
              type="button"
              class="chip-remove"
              @click="removeFilterChip(chip.key)"
              :title="`Remove filter: ${chip.label}`"
            >
              &times;
            </button>
          </span>
          <button type="button" class="chip-clear-all" @click="clearSearch">Clear all</button>
        </div>

        <!-- Search history dropdown -->
        <div v-if="showHistory && searchHistory.length > 0" class="history-dropdown">
          <div
            v-for="item in searchHistory"
            :key="item"
            class="history-item"
            @mousedown.prevent="selectHistoryItem(item)"
          >
            {{ item }}
          </div>
        </div>
      </div>

      <!-- Back to list link when search is active -->
      <div v-if="ehrStore.searchActive" class="back-to-list">
        <a href="#" @click.prevent="clearSearch">&larr; Back to list</a>
      </div>

      <!-- Limit reached banner -->
      <div v-if="ehrStore.searchActive && ehrStore.searchLimitReached" class="limit-banner">
        Showing first 200 results — refine your search to narrow down.
      </div>

      <!-- Loading state -->
      <div v-if="ehrStore.searchLoading || ehrStore.loading" class="loading">
        <span class="spinner"></span> Loading...
      </div>
      <div v-else-if="!ehrStore.searchActive && ehrStore.error" class="error-msg">
        {{ ehrStore.error }}
      </div>
      <div v-else-if="!serverStore.activeServerId" class="empty-state">
        <h3>No server selected</h3>
        <p>Configure a server in the Servers tab.</p>
      </div>

      <!-- Search results -->
      <div v-else-if="ehrStore.searchActive">
        <div
          v-if="ehrStore.searchResults.length === 0 && !ehrStore.searchLoading"
          class="empty-state"
        >
          <h3>No EHRs match your search.</h3>
          <p><a href="#" @click.prevent="clearSearch">Clear search</a></p>
        </div>
        <div v-else class="ehr-list">
          <div
            v-for="ehr in ehrStore.searchResults"
            :key="ehr.ehr_id"
            class="ehr-item"
            :class="{ active: ehr.ehr_id === ehrId }"
            @click="selectEhr(ehr.ehr_id)"
          >
            <div class="ehr-id">
              <span class="id-text">{{ ehr.ehr_id.substring(0, 8) }}...</span>
              <CopyButton :text="ehr.ehr_id" title="Copy full ID" @click.stop />
            </div>
            <div class="ehr-meta">
              <span v-if="ehr.time_created" class="meta-item">{{ ehr.time_created }}</span>
              <span v-if="ehr.subject_id" class="meta-item">Subject: {{ ehr.subject_id }}</span>
              <span v-if="ehr.subject_namespace" class="meta-item"
                >NS: {{ ehr.subject_namespace }}</span
              >
            </div>
          </div>
        </div>
      </div>

      <!-- Normal paginated list -->
      <div v-else>
        <div class="sort-bar">
          <label class="sort-label" for="ehr-sort-field">Sort by</label>
          <select
            id="ehr-sort-field"
            class="input sort-select"
            :value="ehrStore.sortBy"
            :disabled="!ehrStore.sortApplied"
            :title="!ehrStore.sortApplied ? sortUnsupportedTitle : undefined"
            @change="onSortFieldChange(($event.target as HTMLSelectElement).value)"
          >
            <option v-for="opt in sortFieldOptions" :key="opt.value" :value="opt.value">
              {{ opt.label }}
            </option>
          </select>
          <button
            type="button"
            class="btn btn-sm sort-dir-btn"
            :disabled="!ehrStore.sortApplied"
            :title="
              !ehrStore.sortApplied
                ? sortUnsupportedTitle
                : ehrStore.sortDir === 'asc'
                  ? 'Ascending — click for descending'
                  : 'Descending — click for ascending'
            "
            @click="onToggleSortDir"
          >
            {{ ehrStore.sortDir === "asc" ? "↑ Asc" : "↓ Desc" }}
          </button>
        </div>

        <div v-if="!ehrStore.sortApplied" class="limit-banner">
          Sorting isn't supported by this server — showing default order.
        </div>

        <div class="ehr-list">
          <div
            v-for="ehr in ehrStore.ehrs"
            :key="ehr.ehr_id"
            class="ehr-item"
            :class="{ active: ehr.ehr_id === ehrId }"
            @click="selectEhr(ehr.ehr_id)"
          >
            <div class="ehr-id">
              <span class="id-text">{{ ehr.ehr_id.substring(0, 8) }}...</span>
              <CopyButton :text="ehr.ehr_id" title="Copy full ID" @click.stop />
            </div>
            <div class="ehr-meta">
              <span v-if="ehr.time_created" class="meta-item">{{ ehr.time_created }}</span>
              <span v-if="ehr.subject_id" class="meta-item">Subject: {{ ehr.subject_id }}</span>
            </div>
          </div>
        </div>

        <div class="pagination">
          <button type="button" class="btn btn-sm" :disabled="currentPage === 0" @click="prevPage">
            Previous
          </button>
          <span class="page-info">Page {{ currentPage + 1 }}</span>
          <button type="button" class="btn btn-sm" @click="nextPage">Next</button>
        </div>
      </div>
    </div>

    <!-- Detail panel -->
    <div class="panel-right">
      <template v-if="ehrStore.selectedEhr">
        <div class="panel-header">
          <h2>EHR Detail</h2>
          <div class="header-actions">
            <div class="tab-bar">
              <button
                type="button"
                class="tab"
                :class="{ active: activeTab === 'detail' }"
                @click="activeTab = 'detail'"
              >
                Detail
              </button>
              <button
                type="button"
                class="tab"
                data-tour="ehr-directory-tab"
                :class="{ active: activeTab === 'directory' }"
                @click="selectDirectoryTab"
              >
                Directory
              </button>
              <button
                type="button"
                class="tab"
                :class="{ active: activeTab === 'status' }"
                @click="selectStatusHistoryTab"
              >
                Status History
              </button>
              <button
                type="button"
                class="tab"
                :class="{ active: activeTab === 'compositions' }"
                @click="activeTab = 'compositions'"
              >
                Compositions
              </button>
              <button
                type="button"
                class="tab"
                :class="{ active: activeTab === 'json' }"
                @click="activeTab = 'json'"
              >
                JSON
              </button>
              <button
                type="button"
                class="tab"
                :class="{ active: activeTab === 'contributions' }"
                @click="selectContributionsTab"
              >
                Contributions
              </button>
            </div>
            <button type="button" class="btn btn-sm btn-danger" @click="openDeleteDialog">
              Delete EHR
            </button>
          </div>
        </div>

        <div v-if="activeTab === 'detail'" class="detail-section">
          <div class="detail-row">
            <span class="detail-label">EHR ID</span>
            <span class="detail-value mono">
              {{ ehrStore.selectedEhr.ehr_id }}
              <CopyButton :text="ehrStore.selectedEhr!.ehr_id" title="Copy EHR ID" />
            </span>
          </div>
          <div class="detail-row" v-if="ehrStore.selectedEhr.time_created">
            <span class="detail-label">Created</span>
            <span class="detail-value">{{ ehrStore.selectedEhr.time_created }}</span>
          </div>
          <div class="detail-row" v-if="ehrStore.selectedEhr.system_id">
            <span class="detail-label">System ID</span>
            <span class="detail-value mono">{{ ehrStore.selectedEhr.system_id }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">Modifiable</span>
            <span class="detail-value">{{ ehrStore.selectedEhr.is_modifiable ?? "unknown" }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">Queryable</span>
            <span class="detail-value">{{ ehrStore.selectedEhr.is_queryable ?? "unknown" }}</span>
          </div>
          <div class="detail-row" v-if="ehrStore.selectedEhr.subject_id">
            <span class="detail-label">Subject ID</span>
            <span class="detail-value">{{ ehrStore.selectedEhr.subject_id }}</span>
          </div>
          <div class="detail-row" v-if="ehrStore.selectedEhr.subject_namespace">
            <span class="detail-label">Subject Namespace</span>
            <span class="detail-value">{{ ehrStore.selectedEhr.subject_namespace }}</span>
          </div>
        </div>

        <!-- JSON View -->
        <div v-if="activeTab === 'json'" class="json-view">
          <JsonViewer v-if="ehrStore.selectedEhr" :value="ehrStore.selectedEhr" />
        </div>

        <!-- Directory View (OEH-27) -->
        <div v-if="activeTab === 'directory'" class="directory-view">
          <template v-if="directoryEditMode && editableDirectory">
            <div v-if="directorySaveError" class="delete-error">{{ directorySaveError }}</div>
            <DirectoryTreeEditor
              :folder="editableDirectory"
              :path="[]"
              :depth="0"
              :is-root="true"
              :available-compositions="availableCompositionOptions"
            />
            <div class="directory-edit-actions">
              <button
                type="button"
                class="btn btn-sm"
                :disabled="directorySaving"
                @click="cancelEditDirectory"
              >
                Cancel
              </button>
              <button
                type="button"
                class="btn btn-sm btn-primary"
                :disabled="directorySaving"
                @click="saveDirectory"
              >
                {{ directorySaving ? "Saving..." : "Save Directory" }}
              </button>
            </div>
          </template>

          <template v-else>
            <div v-if="!ehrStore.directoryLoading" class="directory-toolbar">
              <button
                v-if="ehrStore.directory"
                type="button"
                class="btn btn-sm"
                @click="startEditDirectory"
              >
                Edit Directory
              </button>
              <button v-else type="button" class="btn btn-sm" @click="startCreateDirectory">
                Create Directory
              </button>
              <button
                v-if="ehrStore.directory"
                type="button"
                class="btn btn-sm btn-danger"
                @click="openDeleteDirectoryDialog"
              >
                Delete Directory
              </button>
            </div>

            <div v-if="ehrStore.directoryLoading" class="loading">
              <span class="spinner"></span> Loading directory...
            </div>
            <div v-else-if="ehrStore.directoryError" class="empty-state">
              <h3>Failed to load directory</h3>
              <p class="error-detail">{{ ehrStore.directoryError }}</p>
            </div>
            <DirectoryTree
              v-else-if="ehrStore.directory"
              :folder="ehrStore.directory"
              :depth="0"
              @open-item="openCompositionRef"
            />
            <div v-else class="empty-state">
              <h3>No directory set</h3>
              <p>This EHR doesn't have a DIRECTORY folder structure.</p>
            </div>
          </template>
        </div>

        <!-- Status History View (OEH-47) -->
        <div v-if="activeTab === 'status'" class="status-history-view">
          <p class="contributions-hint">
            EHR_STATUS is versioned the same way compositions are. Browse its revision history
            below, or jump straight to the version in effect at a specific point in time.
          </p>

          <div class="at-time-picker">
            <label for="status-at-time" class="visually-hidden">At a point in time</label>
            <input
              id="status-at-time"
              type="datetime-local"
              class="input"
              v-model="statusAtTimeInput"
            />
            <button
              type="button"
              class="btn btn-sm btn-primary"
              :disabled="!statusAtTimeInput"
              @click="viewStatusAtTime"
            >
              View at time
            </button>
          </div>

          <div v-if="statusContributionError" class="search-validation-error">
            {{ statusContributionError }}
          </div>

          <div v-if="statusVersionsLoading" class="loading">
            <span class="spinner"></span> Loading status history...
          </div>
          <div v-else-if="statusVersionsError" class="empty-state">
            <h3>Failed to load status history</h3>
            <p class="error-detail">{{ statusVersionsError }}</p>
          </div>
          <div v-else-if="statusVersions.length === 0" class="empty-state">
            <h3>No version history available</h3>
            <p>This server did not report a revision history for the EHR_STATUS.</p>
          </div>
          <div v-else class="version-list">
            <div v-for="v in statusVersions" :key="v.version_id" class="version-row">
              <div class="version-row-main">
                <div class="version-row-id mono">{{ v.version_id }}</div>
                <div class="version-row-meta">
                  <span v-if="v.commit_audit?.change_type" class="badge">{{
                    v.commit_audit.change_type
                  }}</span>
                  <span v-if="v.commit_audit?.committer_name">{{
                    v.commit_audit.committer_name
                  }}</span>
                  <span v-if="v.time_committed">{{ v.time_committed }}</span>
                </div>
              </div>
              <div class="version-row-actions">
                <button type="button" class="btn btn-sm" @click="viewStatusVersion(v.version_id)">
                  Open
                </button>
                <button
                  type="button"
                  class="btn btn-sm"
                  @click="viewStatusContribution(v.version_id)"
                >
                  View Contribution
                </button>
              </div>
            </div>
          </div>

          <!-- Version-document preview panel -->
          <div
            v-if="statusPreviewLoading || statusPreview || statusPreviewError"
            class="status-preview"
          >
            <h3 class="section-title">
              Preview<span v-if="statusPreviewLabel"> — {{ statusPreviewLabel }}</span>
            </h3>
            <div v-if="statusPreviewLoading" class="loading">
              <span class="spinner"></span> Loading version...
            </div>
            <div v-else-if="statusPreviewError" class="empty-state">
              <p class="error-detail">{{ statusPreviewError }}</p>
            </div>
            <JsonViewer v-else-if="statusPreview" :value="statusPreview" />
          </div>
        </div>

        <!-- Contributions View (OEH-28 / OEH-47) -->
        <div v-if="activeTab === 'contributions'" class="contributions-view">
          <p class="contributions-hint">
            Reconstructed by unioning the revision histories of every composition, EHR_STATUS, and
            DIRECTORY under this EHR — openEHR has no single "list contributions for an EHR"
            endpoint. A contribution whose component versions were all deleted or purged won't show
            up here; look it up by UID below as a fallback.
          </p>

          <div v-if="contributionRowsLoading" class="loading">
            <span class="spinner"></span> Loading contributions...
          </div>
          <div v-else-if="contributionRowsError" class="empty-state">
            <h3>Failed to load contributions</h3>
            <p class="error-detail">{{ contributionRowsError }}</p>
          </div>
          <template v-else>
            <div v-if="contributionRowsPartialWarning" class="limit-banner">
              {{ contributionRowsPartialWarning }}
            </div>
            <template v-if="contributionRows.length > 0">
              <div class="activity-chart" v-if="activityByDay.length > 1">
                <div
                  v-for="bucket in activityByDay"
                  :key="bucket.day"
                  class="activity-bar"
                  :title="`${bucket.day}: ${bucket.count} commit(s)`"
                >
                  <div class="activity-bar-fill" :style="{ height: bucket.pct + '%' }"></div>
                </div>
              </div>
              <div class="version-list">
                <div
                  v-for="row in contributionRows"
                  :key="`${row.source}-${row.version_id}`"
                  class="version-row"
                >
                  <div class="version-row-main">
                    <div class="version-row-meta">
                      <span class="badge badge-source">{{ row.source }}</span>
                      <span v-if="row.sourceDetail">{{ row.sourceDetail }}</span>
                    </div>
                    <div class="version-row-id mono">{{ row.version_id }}</div>
                    <div class="version-row-meta">
                      <span v-if="row.change_type" class="badge">{{ row.change_type }}</span>
                      <span v-if="row.committer_name">{{ row.committer_name }}</span>
                      <span v-if="row.time_committed">{{ row.time_committed }}</span>
                    </div>
                  </div>
                  <button type="button" class="btn btn-sm" @click="viewContributionForRow(row)">
                    View Contribution
                  </button>
                </div>
              </div>
            </template>
            <div v-else class="empty-state">
              <h3>No contributions found</h3>
              <p>
                No revision history was reported for this EHR's compositions, EHR_STATUS, or
                DIRECTORY.
              </p>
            </div>
          </template>

          <h3 class="section-title contribution-lookup-title">Look up by UID</h3>
          <div class="contribution-lookup">
            <label for="contribution-lookup-uid" class="visually-hidden">Contribution UID</label>
            <input
              id="contribution-lookup-uid"
              class="input"
              v-model="contributionLookupUid"
              placeholder="Contribution UID"
              autocapitalize="off"
              autocorrect="off"
              spellcheck="false"
              @keydown.enter="lookupContribution"
            />
            <button type="button" class="btn btn-sm btn-primary" @click="lookupContribution">
              View
            </button>
          </div>
          <div v-if="contributionLookupError" class="search-validation-error">
            {{ contributionLookupError }}
          </div>
        </div>

        <!-- Compositions View (OEH-47) -->
        <div v-if="activeTab === 'compositions'" class="compositions-tab-view">
          <h3 class="section-title">
            Compositions ({{
              compositionFiltersActive
                ? `${filteredCompositionCount} of ${ehrStore.selectedEhr.compositions.length}`
                : ehrStore.selectedEhr.compositions.length
            }})
          </h3>

          <div class="composition-filters">
            <label for="comp-filter-template" class="visually-hidden">Template</label>
            <input
              id="comp-filter-template"
              class="input"
              v-model="compositionFilterTemplate"
              placeholder="Template"
              autocapitalize="off"
              autocorrect="off"
              spellcheck="false"
            />
            <label for="comp-filter-composer" class="visually-hidden">Composer</label>
            <input
              id="comp-filter-composer"
              class="input"
              v-model="compositionFilterComposer"
              placeholder="Composer"
              autocapitalize="off"
              autocorrect="off"
              spellcheck="false"
            />
            <label for="comp-filter-from" class="visually-hidden">From date</label>
            <input
              id="comp-filter-from"
              type="date"
              class="input"
              v-model="compositionFilterFrom"
            />
            <label for="comp-filter-to" class="visually-hidden">To date</label>
            <input id="comp-filter-to" type="date" class="input" v-model="compositionFilterTo" />
            <button
              type="button"
              class="btn btn-sm"
              :disabled="!compositionFiltersActive"
              @click="clearCompositionFilters"
            >
              Clear
            </button>
          </div>

          <div
            v-for="(comps, templateId) in compositionsByTemplate"
            :key="templateId"
            class="template-group"
          >
            <div class="template-group-header">
              <span class="template-name">{{ templateId }}</span>
              <span class="badge">{{ comps.length }}</span>
            </div>
            <div
              v-for="comp in comps"
              :key="comp.uid"
              class="composition-item"
              @click="openComposition(comp)"
            >
              <div class="comp-name">{{ comp.name ?? comp.uid.substring(0, 8) }}</div>
              <div class="comp-meta">
                <span v-if="comp.composer">{{ comp.composer }}</span>
                <span v-if="comp.time_committed">{{ comp.time_committed }}</span>
              </div>
            </div>
          </div>

          <div
            v-if="Object.keys(compositionsByTemplate).length === 0 && !compositionFiltersActive"
            class="empty-state"
          >
            <p>No compositions found for this EHR.</p>
          </div>
          <div v-else-if="Object.keys(compositionsByTemplate).length === 0" class="empty-state">
            <p>No compositions match these filters.</p>
          </div>
        </div>
      </template>

      <div v-else class="empty-state">
        <h3>Select an EHR</h3>
        <p>Click on an EHR from the list to view its details and compositions.</p>
      </div>
    </div>

    <!-- EHR Create Dialog -->
    <EhrCreateDialog
      :open="showCreateDialog"
      @close="showCreateDialog = false"
      @created="handleEhrCreated"
    />

    <!-- Filters Modal — structured filter builder, no syntax to remember -->
    <EhrFilterModal
      :open="showFilterModal"
      :criteria="activeCriteria ?? {}"
      @close="showFilterModal = false"
      @apply="handleFilterModalApply"
    />

    <!-- EHR Delete Confirmation Dialog -->
    <div v-if="showDeleteDialog" class="dialog-overlay" @click="showDeleteDialog = false">
      <div class="dialog" @click.stop>
        <h3>Delete EHR</h3>
        <p>
          This action cannot be undone. This will permanently delete the EHR and all its
          compositions.
        </p>
        <p>Please type the EHR ID to confirm:</p>
        <div class="confirm-text">
          <code>{{ ehrId }}</code>
        </div>
        <input
          v-model="deleteConfirmText"
          type="text"
          class="input"
          placeholder="Enter EHR ID to confirm"
          :disabled="deleting"
        />
        <div v-if="deleteError" class="delete-error">
          <strong>Failed to delete EHR</strong>
          <p class="error-detail">{{ deleteError }}</p>
          <div v-if="deleteError.includes('HTTP 403')" class="delete-hint">
            EHR deletion requires admin credentials. Check the
            <strong>Admin Credentials</strong> setting in your server profile (Servers &rarr; Edit).
          </div>
        </div>
        <div class="dialog-actions">
          <button
            type="button"
            class="btn btn-sm"
            @click="showDeleteDialog = false"
            :disabled="deleting"
          >
            Cancel
          </button>
          <button
            type="button"
            class="btn btn-sm btn-danger"
            @click="handleDeleteEhr"
            :disabled="!canDelete || deleting"
          >
            {{ deleting ? "Deleting..." : "Delete EHR" }}
          </button>
        </div>
      </div>
    </div>

    <!-- Directory Delete Confirmation Dialog -->
    <div
      v-if="showDeleteDirectoryDialog"
      class="dialog-overlay"
      @click="showDeleteDirectoryDialog = false"
    >
      <div class="dialog" @click.stop>
        <h3>Delete Directory</h3>
        <p>
          This removes the entire DIRECTORY folder structure for this EHR. This action cannot be
          undone. The compositions it references are not deleted.
        </p>
        <div v-if="deleteDirectoryError" class="delete-error">
          <strong>Failed to delete directory</strong>
          <p class="error-detail">{{ deleteDirectoryError }}</p>
        </div>
        <div class="dialog-actions">
          <button
            type="button"
            class="btn btn-sm"
            @click="showDeleteDirectoryDialog = false"
            :disabled="deletingDirectory"
          >
            Cancel
          </button>
          <button
            type="button"
            class="btn btn-sm btn-danger"
            @click="handleDeleteDirectory"
            :disabled="deletingDirectory"
          >
            {{ deletingDirectory ? "Deleting..." : "Delete Directory" }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ehr-browser {
  display: flex;
  height: 100%;
}

.panel-left {
  width: 380px;
  min-width: 380px;
  border-right: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}

.panel-right {
  flex: 1;
  overflow-y: auto;
  padding: 0 24px 24px;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  border-bottom: 1px solid var(--color-border);
  position: sticky;
  top: 0;
  background: var(--color-bg);
  z-index: 1;
}
.panel-right .panel-header {
  padding: 16px 0;
}
.panel-header h2 {
  font-size: 16px;
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.tab-bar {
  display: flex;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  overflow: hidden;
}
.tab {
  padding: 4px 12px;
  background: var(--color-surface);
  color: var(--color-text-secondary);
  border: none;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
}
.tab:not(:last-child) {
  border-right: 1px solid var(--color-border);
}
.tab.active {
  background: var(--color-primary-dim);
  color: #fff;
}

.json-view,
.directory-view {
  margin-top: 16px;
  overflow: auto;
}

.directory-toolbar {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}

.directory-edit-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--color-border);
}

.search-bar {
  padding: 8px 16px;
  position: relative;
}
.search-input-wrapper {
  display: flex;
  align-items: center;
  gap: 4px;
}
.search-input {
  flex: 1;
  font-size: 12px;
}
.clear-btn {
  width: 24px;
  height: 24px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-surface);
  color: var(--color-text-secondary);
  font-size: 14px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: all 0.15s;
}
.clear-btn:hover {
  background: var(--color-border);
  color: var(--color-text);
}

.filter-btn {
  position: relative;
  width: 24px;
  height: 24px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-surface);
  color: var(--color-text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: all 0.15s;
}
.filter-btn:hover {
  background: var(--color-border);
  color: var(--color-text);
}
.filter-count-badge {
  position: absolute;
  top: -6px;
  right: -6px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  border-radius: 8px;
  background: var(--color-primary);
  color: #fff;
  font-size: 10px;
  font-weight: 600;
  line-height: 1;
  box-shadow: 0 0 0 2px var(--color-bg);
}

.search-validation-error {
  margin-top: 6px;
  padding: 6px 10px;
  background: rgba(255, 90, 90, 0.1);
  border: 1px solid rgba(255, 90, 90, 0.3);
  border-radius: var(--radius);
  color: var(--color-error);
  font-size: 12px;
}

.filter-chips {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  margin-top: 8px;
}
.filter-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 6px 3px 10px;
  border-radius: 999px;
  background: var(--color-primary-dim);
  color: #fff;
  font-size: 11px;
  line-height: 1.4;
}
.chip-remove {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border: none;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.15);
  color: inherit;
  font-size: 12px;
  line-height: 1;
  cursor: pointer;
  padding: 0;
}
.chip-remove:hover {
  background: rgba(255, 255, 255, 0.3);
}
.chip-clear-all {
  background: none;
  border: none;
  padding: 0;
  font-size: 11px;
  color: var(--color-text-muted);
  text-decoration: underline;
  cursor: pointer;
}
.chip-clear-all:hover {
  color: var(--color-text);
}

.history-dropdown {
  position: absolute;
  top: 100%;
  left: 16px;
  right: 16px;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  z-index: 100;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  max-height: 200px;
  overflow-y: auto;
}
.history-item {
  padding: 8px 12px;
  font-size: 12px;
  font-family: var(--font-mono);
  cursor: pointer;
  border-bottom: 1px solid var(--color-border);
}
.history-item:hover {
  background: var(--color-surface);
}
.history-item:last-child {
  border-bottom: none;
}

.back-to-list {
  padding: 4px 16px 8px;
}
.back-to-list a {
  font-size: 12px;
  color: var(--color-primary);
  text-decoration: none;
}
.back-to-list a:hover {
  text-decoration: underline;
}

.limit-banner {
  margin: 0 16px 8px;
  padding: 6px 10px;
  background: rgba(255, 200, 50, 0.1);
  border: 1px solid rgba(255, 200, 50, 0.3);
  border-radius: var(--radius);
  font-size: 12px;
  color: var(--color-text-secondary);
}

.spinner {
  display: inline-block;
  width: 12px;
  height: 12px;
  border: 2px solid var(--color-border);
  border-top-color: var(--color-primary);
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.ehr-list {
  flex: 1;
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 48px 24px;
  color: var(--color-text-muted);
}
.empty-state h3 {
  font-size: 14px;
  font-weight: 500;
  margin-bottom: 8px;
}
.empty-state p {
  font-size: 13px;
}
.empty-state a {
  color: var(--color-primary);
  text-decoration: none;
}
.empty-state a:hover {
  text-decoration: underline;
}

.ehr-item {
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border);
  cursor: pointer;
  transition: background 0.15s;
}
.ehr-item:hover {
  background: var(--color-surface);
}
.ehr-item.active {
  background: var(--color-surface);
  border-left: 3px solid var(--color-primary);
}

.ehr-id {
  display: flex;
  align-items: center;
  gap: 8px;
}
.id-text {
  font-family: var(--font-mono);
  font-size: 13px;
}
.ehr-meta {
  margin-top: 4px;
  display: flex;
  gap: 12px;
}
.meta-item {
  font-size: 11px;
  color: var(--color-text-muted);
}

.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 12px;
  border-top: 1px solid var(--color-border);
}
.page-info {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.sort-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 16px 8px;
}
.sort-label {
  font-size: 12px;
  color: var(--color-text-secondary);
}
.sort-select {
  font-size: 12px;
  padding: 4px 8px;
  width: auto;
}
.sort-dir-btn {
  white-space: nowrap;
}

.detail-section {
  margin-bottom: 24px;
}
.detail-row {
  display: flex;
  padding: 8px 0;
  border-bottom: 1px solid var(--color-border);
}
.detail-label {
  width: 120px;
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-secondary);
}
.detail-value {
  flex: 1;
  font-size: 13px;
}
.detail-value.mono {
  font-family: var(--font-mono);
  font-size: 12px;
  word-break: break-all;
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 12px;
  color: var(--color-text-secondary);
}

.contributions-view {
  margin-bottom: 24px;
}
.contributions-hint {
  font-size: 12px;
  line-height: 1.6;
  color: var(--color-text-secondary);
  margin-bottom: 16px;
}
.contribution-lookup {
  display: flex;
  gap: 8px;
}
.contribution-lookup .input {
  flex: 1;
  font-family: var(--font-mono);
}
.contribution-lookup-title {
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid var(--color-border);
}

/* Status history (OEH-47) */
.status-history-view {
  margin-bottom: 24px;
}
.at-time-picker {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}
.at-time-picker .input {
  flex: 1;
}
.status-preview {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid var(--color-border);
}

/* Revision-history / reconstructed-contribution rows — same shape as
   CompositionViewer.vue's Versions tab. */
.version-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.version-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-surface);
}
.version-row-main {
  min-width: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.version-row-id {
  font-size: 12px;
  word-break: break-all;
}
.version-row-meta {
  display: flex;
  gap: 10px;
  align-items: center;
  flex-wrap: wrap;
  font-size: 12px;
  color: var(--color-text-secondary);
}
.version-row-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}
.badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: var(--radius);
  background: var(--color-primary-dim);
  color: #fff;
  font-size: 11px;
  font-weight: 600;
}
.badge-source {
  background: var(--color-surface-hover, var(--color-border));
  color: var(--color-text-secondary);
}

/* Simple per-day commit-count bars above the Contributions table — not a
   substitute for the table, just a quick sense of activity over time. */
.activity-chart {
  display: flex;
  align-items: flex-end;
  gap: 2px;
  height: 48px;
  margin-bottom: 16px;
  padding: 4px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
}
.activity-bar {
  flex: 1;
  height: 100%;
  display: flex;
  align-items: flex-end;
  min-width: 2px;
}
.activity-bar-fill {
  width: 100%;
  min-height: 2px;
  background: var(--color-primary-dim);
  border-radius: 1px;
}

/* Visually hidden but still reachable by screen readers — pairs the
   contribution-UID input with an accessible label without duplicating the
   visible "Contribution UID" placeholder text on screen. */
.visually-hidden {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

.compositions-tab-view {
  margin-bottom: 24px;
}

.composition-filters {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}
.composition-filters .input {
  flex: 1;
  min-width: 0;
}
.composition-filters input[type="date"] {
  flex: 0 1 160px;
}

.template-group {
  margin-bottom: 16px;
}
.template-group-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 0;
  font-size: 13px;
  font-weight: 600;
  color: var(--color-primary);
}

.composition-item {
  padding: 8px 12px;
  margin-left: 12px;
  border-left: 2px solid var(--color-border);
  cursor: pointer;
  transition: all 0.15s;
}
.composition-item:hover {
  background: var(--color-surface);
  border-left-color: var(--color-primary);
}

.comp-name {
  font-size: 13px;
}
.comp-meta {
  display: flex;
  gap: 12px;
  margin-top: 2px;
  font-size: 11px;
  color: var(--color-text-muted);
}

.dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.dialog {
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 24px;
  min-width: 400px;
  max-width: 500px;
}

.dialog h3 {
  margin: 0 0 16px 0;
  font-size: 18px;
  font-weight: 600;
}

.dialog p {
  margin: 0 0 12px 0;
  color: var(--color-text-secondary);
  line-height: 1.5;
}

.confirm-text {
  background: var(--color-surface);
  padding: 8px 12px;
  border-radius: var(--radius);
  margin-bottom: 12px;
  font-family: var(--font-mono);
  font-size: 12px;
  word-break: break-all;
}

.dialog .input {
  width: 100%;
  margin-bottom: 24px;
}

.delete-error {
  margin-top: 12px;
  padding: 10px 14px;
  background: rgba(255, 90, 90, 0.1);
  border: 1px solid rgba(255, 90, 90, 0.3);
  border-radius: var(--radius);
  color: var(--color-error);
  font-size: 13px;
  line-height: 1.5;
  overflow-wrap: break-word;
  word-break: break-word;
}
.delete-error > strong {
  display: block;
  margin-bottom: 4px;
}
.error-detail {
  margin: 0;
  color: var(--color-text-secondary);
  font-size: 12px;
}
.delete-hint {
  margin-top: 8px;
  padding: 8px 10px;
  background: rgba(255, 200, 50, 0.1);
  border: 1px solid rgba(255, 200, 50, 0.3);
  border-radius: var(--radius);
  color: var(--color-text);
  font-size: 13px;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

.btn-danger {
  background: rgba(255, 90, 90, 0.1);
  color: var(--color-error);
  border: 1px solid rgba(255, 90, 90, 0.3);
}

.btn-danger:hover:not(:disabled) {
  background: rgba(255, 90, 90, 0.2);
}
</style>
