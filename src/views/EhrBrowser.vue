<script setup lang="ts">
import { ref, watch, computed, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useServerStore } from "../stores/server";
import { useEhrStore, type CompositionSummary, type EhrSearchCriteria } from "../stores/ehr";
import { useAnalytics } from "../composables/useAnalytics";
import { useTourStore } from "../stores/tour";
import EhrCreateDialog from "../components/EhrCreateDialog.vue";
import EhrFilterModal from "../components/EhrFilterModal.vue";
import DirectoryTree from "../components/DirectoryTree.vue";
import CompassIcon from "../components/CompassIcon.vue";
import JsonViewer from "../components/JsonViewer.vue";
import CopyButton from "../components/CopyButton.vue";

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
const activeTab = ref<"detail" | "directory" | "json" | "contributions">("detail");
const contributionLookupUid = ref("");
const contributionLookupError = ref<string | null>(null);
const showHelpModal = ref(false);
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
    if (activeTab.value === "directory" && ehrId.value && id) {
      ehrStore.fetchDirectory(id, ehrId.value);
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
  if (activeTab.value === "directory" && id && serverStore.activeServerId) {
    ehrStore.fetchDirectory(serverStore.activeServerId, id);
  }
});

const DATE_RE = /^\d{4}-\d{2}-\d{2}$/;

function parseSearchInput(raw: string): {
  criteria: EhrSearchCriteria | null;
  error: string | null;
  warning: string | null;
} {
  const trimmed = raw.trim();
  if (!trimmed) return { criteria: null, error: null, warning: null };

  const criteria: EhrSearchCriteria = {};
  let warning: string | null = null;
  const tokens = trimmed.split(/\s+/);

  for (const token of tokens) {
    const colonIdx = token.indexOf(":");
    if (colonIdx === -1) {
      // No colon — treat as EHR ID prefix
      if (criteria.ehr_id_prefix) {
        criteria.ehr_id_prefix += token; // append if multiple bare tokens
      } else {
        criteria.ehr_id_prefix = token;
      }
      continue;
    }

    const prefix = token.substring(0, colonIdx);
    const value = token.substring(colonIdx + 1);

    if (!value) {
      return { criteria: null, error: `${prefix}: value cannot be empty.`, warning: null };
    }

    switch (prefix) {
      case "subject":
        criteria.subject_id = value;
        break;
      case "namespace":
        criteria.subject_namespace = value;
        break;
      case "system":
        criteria.system_id = value;
        break;
      case "modifiable":
        if (value !== "true" && value !== "false") {
          return { criteria: null, error: "modifiable: expects 'true' or 'false'.", warning: null };
        }
        criteria.modifiable = value === "true";
        break;
      case "hasCompositions":
        if (value !== "true" && value !== "false") {
          return {
            criteria: null,
            error: "hasCompositions: expects 'true' or 'false'.",
            warning: null,
          };
        }
        criteria.has_compositions = value === "true";
        break;
      case "hasDirectory":
        if (value !== "true" && value !== "false") {
          return {
            criteria: null,
            error: "hasDirectory: expects 'true' or 'false'.",
            warning: null,
          };
        }
        criteria.has_directory = value === "true";
        break;
      case "created-on":
        if (!DATE_RE.test(value)) {
          return {
            criteria: null,
            error: "created-on: expects a date in YYYY-MM-DD format (e.g. 2026-03-12).",
            warning: null,
          };
        }
        criteria.created_on = value;
        break;
      case "created-before":
        if (!DATE_RE.test(value)) {
          return {
            criteria: null,
            error: "created-before: expects a date in YYYY-MM-DD format (e.g. 2026-03-12).",
            warning: null,
          };
        }
        criteria.created_before = value;
        break;
      case "created-after":
        if (!DATE_RE.test(value)) {
          return {
            criteria: null,
            error: "created-after: expects a date in YYYY-MM-DD format (e.g. 2026-03-12).",
            warning: null,
          };
        }
        criteria.created_after = value;
        break;
      default:
        // Unknown prefix — treat as EHR ID prefix (safe fallback)
        if (criteria.ehr_id_prefix) {
          criteria.ehr_id_prefix += token;
        } else {
          criteria.ehr_id_prefix = token;
        }
        break;
    }
  }

  // Conflict resolution: created_on overrides created_before/created_after
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
    chips.push({ key: "modifiable", label: `Modifiable: ${c.modifiable ? "Yes" : "No"}` });
  if (c.has_compositions !== undefined)
    chips.push({
      key: "has_compositions",
      label: `Has compositions: ${c.has_compositions ? "Yes" : "No"}`,
    });
  if (c.has_directory !== undefined)
    chips.push({
      key: "has_directory",
      label: `Has directory entries: ${c.has_directory ? "Yes" : "No"}`,
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

// Sort paginated EHRs by time_created descending (only when not searching)
const sortedEhrs = computed(() => {
  return [...ehrStore.ehrs].sort((a, b) => {
    if (!a.time_created && !b.time_created) return 0;
    if (!a.time_created) return 1;
    if (!b.time_created) return -1;
    return b.time_created.localeCompare(a.time_created);
  });
});

// Group compositions by template_id
const compositionsByTemplate = computed(() => {
  if (!ehrStore.selectedEhr) return {};
  const groups: Record<string, CompositionSummary[]> = {};
  for (const comp of ehrStore.selectedEhr.compositions) {
    const key = comp.template_id ?? "(no template)";
    if (!groups[key]) groups[key] = [];
    groups[key].push(comp);
  }
  return groups;
});

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
            title="Build filters without typing"
          >
            Filters
            <span v-if="filterChips.length" class="filter-count-badge">{{
              filterChips.length
            }}</span>
          </button>
          <button
            type="button"
            class="help-btn"
            data-tour="ehr-search-help"
            @click="showHelpModal = true"
            title="Search syntax help"
          >
            ?
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
        <div class="ehr-list">
          <div
            v-for="ehr in sortedEhrs"
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
                :class="{ active: activeTab === 'json' }"
                @click="activeTab = 'json'"
              >
                JSON
              </button>
              <button
                type="button"
                class="tab"
                :class="{ active: activeTab === 'contributions' }"
                @click="activeTab = 'contributions'"
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
        </div>

        <!-- Contributions View (OEH-28) -->
        <div v-if="activeTab === 'contributions'" class="contributions-view">
          <p class="contributions-hint">
            openEHR doesn't provide a way to list all contributions for an EHR — only to fetch one
            by UID. Enter a known contribution UID below, or open a composition's
            <strong>Versions</strong> tab and click <strong>View Contribution</strong> to jump
            straight to the commit that created a specific version.
          </p>
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

        <h3 class="section-title" v-if="activeTab === 'detail'">
          Compositions ({{ ehrStore.selectedEhr.compositions.length }})
        </h3>

        <template v-if="activeTab === 'detail'">
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

          <div v-if="ehrStore.selectedEhr.compositions.length === 0" class="empty-state">
            <p>No compositions found for this EHR.</p>
          </div>
        </template>
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

    <!-- Search Syntax Help Modal (for the quick-search box's colon syntax) -->
    <div v-if="showHelpModal" class="dialog-overlay" @click="showHelpModal = false">
      <div class="dialog" @click.stop>
        <h3>Search Syntax</h3>
        <p>
          Prefer building filters visually? Use the <strong>Filters</strong> button instead — this
          syntax is a shortcut for the quick search box.
        </p>
        <table class="help-table">
          <tbody>
            <tr>
              <td class="help-example">fde80e0e...</td>
              <td>EHR ID prefix match</td>
            </tr>
            <tr>
              <td class="help-example">subject:value</td>
              <td>Subject ID contains match</td>
            </tr>
            <tr>
              <td class="help-example">namespace:value</td>
              <td>Subject namespace exact match</td>
            </tr>
            <tr>
              <td class="help-example">system:value</td>
              <td>System ID exact match</td>
            </tr>
            <tr>
              <td class="help-example">modifiable:true|false</td>
              <td>EHR status is_modifiable</td>
            </tr>
            <tr>
              <td class="help-example">hasCompositions:true</td>
              <td>Has compositions (false not supported)</td>
            </tr>
            <tr>
              <td class="help-example">hasDirectory:true|false</td>
              <td>Has a DIRECTORY (checked per matching EHR)</td>
            </tr>
          </tbody>
        </table>
        <p class="help-note">Combine terms with spaces (implicit AND).</p>
        <p class="help-note">
          Press Enter or wait 600ms to search. All searches use AQL and appear in the Request
          Inspector.
        </p>
        <div class="dialog-actions">
          <button type="button" class="btn btn-sm" @click="showHelpModal = false">Close</button>
        </div>
      </div>
    </div>

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
.clear-btn,
.help-btn {
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
.clear-btn:hover,
.help-btn:hover {
  background: var(--color-border);
  color: var(--color-text);
}

.filter-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 24px;
  padding: 0 10px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-surface);
  color: var(--color-text-secondary);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.15s;
}
.filter-btn:hover {
  background: var(--color-border);
  color: var(--color-text);
}
.filter-count-badge {
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

.help-table {
  width: 100%;
  font-size: 12px;
  border-collapse: collapse;
}
.help-table td {
  padding: 3px 8px;
  border-bottom: 1px solid var(--color-border);
}
.help-example {
  font-family: var(--font-mono);
  color: var(--color-primary);
  white-space: nowrap;
}
.help-note {
  font-size: 11px;
  color: var(--color-text-muted);
  margin: 6px 0 0;
}
.help-warning {
  color: var(--color-warning, #e6a817);
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
