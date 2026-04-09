<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useServerStore } from "../stores/server";
import { useEhrStore, type CompositionSummary, type EhrSearchCriteria } from "../stores/ehr";
import EhrCreateDialog from "../components/EhrCreateDialog.vue";

const route = useRoute();
const router = useRouter();
const serverStore = useServerStore();
const ehrStore = useEhrStore();
const searchQuery = ref("");
const currentPage = ref(0);
const showCreateDialog = ref(false);
const showDeleteDialog = ref(false);
const deleteConfirmText = ref("");
const deleting = ref(false);
const deleteError = ref<string | null>(null);
const activeTab = ref<"detail" | "json">("detail");
const showHelpPopover = ref(false);
const validationError = ref<string | null>(null);
const searchHistory = ref<string[]>([]);
const showHistory = ref(false);

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

const ehrId = computed(() => route.params.ehrId as string | undefined);

watch(
  () => serverStore.activeServerId,
  (id) => {
    if (id) {
      ehrStore.fetchEhrs(id, 0);
      currentPage.value = 0;
    }
  },
  { immediate: true },
);

watch(ehrId, (id) => {
  if (id && serverStore.activeServerId) {
    // If we have search results and the selected EHR is in them, pre-populate from search data
    if (ehrStore.searchActive) {
      const searchResult = ehrStore.searchResults.find((r) => r.ehr_id === id);
      if (searchResult) {
        // Still fetch full detail for compositions, but we have metadata already
        ehrStore.fetchEhrDetail(serverStore.activeServerId, id);
        return;
      }
    }
    ehrStore.fetchEhrDetail(serverStore.activeServerId, id);
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
  ehrStore.searchEhrs(serverStore.activeServerId, criteria);
}

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

async function copyToClipboard(text: string) {
  await navigator.clipboard.writeText(text);
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

const ehrJson = computed(() => {
  if (!ehrStore.selectedEhr) return "";
  return JSON.stringify(ehrStore.selectedEhr, null, 2);
});

async function copyEhrJson() {
  if (ehrJson.value) {
    await navigator.clipboard.writeText(ehrJson.value);
  }
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
          <button class="btn btn-sm btn-primary" @click="showCreateDialog = true">+ New EHR</button>
          <button class="btn btn-sm" @click="refresh">Refresh</button>
        </div>
      </div>

      <div class="search-bar">
        <div class="search-input-wrapper">
          <input
            class="input search-input"
            v-model="searchQuery"
            placeholder="EHR ID, or subject:...  namespace:...  system:...  modifiable:...  hasCompositions:true"
            autocapitalize="off"
            autocorrect="off"
            spellcheck="false"
            @keydown="onSearchKeydown"
            @input="onSearchInput"
            @focus="showHistory = searchHistory.length > 0 && !searchQuery"
            @blur="hideHistoryDelayed"
          />
          <button v-if="searchQuery" class="clear-btn" @click="clearSearch" title="Clear search">
            &times;
          </button>
          <button
            class="help-btn"
            @click="showHelpPopover = !showHelpPopover"
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

        <!-- Help popover -->
        <div v-if="showHelpPopover" class="help-popover">
          <div class="help-popover-header">
            <strong>Search Syntax</strong>
            <button class="close-btn" @click="showHelpPopover = false">&times;</button>
          </div>
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
            </tbody>
          </table>
          <p class="help-note">
            Combine terms with spaces (implicit AND).
          </p>
          <p class="help-note">
            Press Enter or wait 600ms to search. All searches use AQL and appear in the Request
            Inspector.
          </p>
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
              <button
                class="copy-btn"
                @click.stop="copyToClipboard(ehr.ehr_id)"
                title="Copy full ID"
              >
                Copy
              </button>
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
              <button
                class="copy-btn"
                @click.stop="copyToClipboard(ehr.ehr_id)"
                title="Copy full ID"
              >
                Copy
              </button>
            </div>
            <div class="ehr-meta">
              <span v-if="ehr.time_created" class="meta-item">{{ ehr.time_created }}</span>
              <span v-if="ehr.subject_id" class="meta-item">Subject: {{ ehr.subject_id }}</span>
            </div>
          </div>
        </div>

        <div class="pagination">
          <button class="btn btn-sm" :disabled="currentPage === 0" @click="prevPage">
            Previous
          </button>
          <span class="page-info">Page {{ currentPage + 1 }}</span>
          <button class="btn btn-sm" @click="nextPage">Next</button>
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
                class="tab"
                :class="{ active: activeTab === 'detail' }"
                @click="activeTab = 'detail'"
              >
                Detail
              </button>
              <button
                class="tab"
                :class="{ active: activeTab === 'json' }"
                @click="activeTab = 'json'"
              >
                JSON
              </button>
            </div>
            <button class="btn btn-sm" v-if="activeTab === 'json'" @click="copyEhrJson">
              Copy JSON
            </button>
            <button class="btn btn-sm btn-danger" @click="openDeleteDialog">Delete EHR</button>
          </div>
        </div>

        <div v-if="activeTab === 'detail'" class="detail-section">
          <div class="detail-row">
            <span class="detail-label">EHR ID</span>
            <span class="detail-value mono">
              {{ ehrStore.selectedEhr.ehr_id }}
              <button class="copy-btn" @click="copyToClipboard(ehrStore.selectedEhr!.ehr_id)">
                Copy
              </button>
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
          <pre class="json-pre">{{ ehrJson }}</pre>
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
          <button class="btn btn-sm" @click="showDeleteDialog = false" :disabled="deleting">
            Cancel
          </button>
          <button
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

.json-view {
  margin-top: 16px;
  overflow: auto;
}
.json-pre {
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.6;
  color: var(--color-text);
  white-space: pre-wrap;
  word-break: break-word;
  background: var(--color-surface);
  padding: 16px;
  border-radius: var(--radius);
  border: 1px solid var(--color-border);
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

.search-validation-error {
  margin-top: 6px;
  padding: 6px 10px;
  background: rgba(255, 90, 90, 0.1);
  border: 1px solid rgba(255, 90, 90, 0.3);
  border-radius: var(--radius);
  color: var(--color-error);
  font-size: 12px;
}

.help-popover {
  position: absolute;
  top: 100%;
  left: 16px;
  right: 16px;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 12px;
  z-index: 100;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}
.help-popover-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.help-popover-header strong {
  font-size: 13px;
}
.close-btn {
  background: none;
  border: none;
  font-size: 16px;
  cursor: pointer;
  color: var(--color-text-secondary);
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
