<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { useServerStore } from "../stores/server";
import { useQueryStore, type SavedQuery, type StoredQuerySummary } from "../stores/query";
import { useTemplateStore } from "../stores/template";
import { useAnalytics } from "../composables/useAnalytics";
import { useTourStore } from "../stores/tour";
import { useVirtualList } from "../composables/useVirtualList";
import AqlEditor from "../components/AqlEditor.vue";
import CompassIcon from "../components/CompassIcon.vue";
import JsonViewer from "../components/JsonViewer.vue";
import DeleteButton from "../components/DeleteButton.vue";
import SearchableSelect, { type SearchableSelectOption } from "../components/SearchableSelect.vue";
import { extractAqlPathIndex, extractAqlPathsForArchetype } from "../lib/aql/aqlPathIndex";
import type { AqlPathEntry } from "../lib/aql/aqlPathIndex";

const analytics = useAnalytics();
const tourStore = useTourStore();

const serverStore = useServerStore();
const queryStore = useQueryStore();
const templateStore = useTemplateStore();

const queryText = ref(
  "SELECT e/ehr_id/value, c/uid/value, c/name/value\nFROM EHR e\nCONTAINS COMPOSITION c\nLIMIT 20",
);
const saveName = ref("");
const showSaveDialog = ref(false);

// Context Template (Layer 3)
const contextTemplateId = ref<string | null>(null);
const contextTemplateOptions = computed<SearchableSelectOption[]>(() =>
  templateStore.templates.map((t) => ({ value: t.template_id, label: t.template_id })),
);

// Resizable editor
const editorHeight = ref(240);
const isResizing = ref(false);

// Template path index for Layer 3 completions
const templatePaths = ref<Map<string, AqlPathEntry[]>>(new Map());
const allTemplatePaths = ref<AqlPathEntry[]>([]);

// Reference to AqlEditor component
const editorRef = ref<InstanceType<typeof AqlEditor> | null>(null);

// Both query lists are virtualized (src/composables/useVirtualList.ts) — a
// server with hundreds/thousands of STORED_QUERY definitions used to render
// one .saved-item row per entry up front, which made switching to the AQL
// Runner (or to a server with many stored queries) slow and unresponsive.
// ROW_HEIGHT must match `.saved-item`'s fixed CSS height below.
const QUERY_ROW_HEIGHT = 34;
const savedListEl = ref<HTMLElement | null>(null);
const {
  onScroll: onSavedScroll,
  topPadding: savedTopPadding,
  bottomPadding: savedBottomPadding,
  visibleItems: visibleSavedQueries,
} = useVirtualList(
  computed(() => queryStore.savedQueries),
  savedListEl,
  { rowHeight: QUERY_ROW_HEIGHT },
);
const storedListEl = ref<HTMLElement | null>(null);
const {
  onScroll: onStoredScroll,
  topPadding: storedTopPadding,
  bottomPadding: storedBottomPadding,
  visibleItems: visibleStoredQueries,
} = useVirtualList(
  computed(() => queryStore.storedQueries),
  storedListEl,
  { rowHeight: QUERY_ROW_HEIGHT },
);

// Load data after component is mounted to avoid blocking render
onMounted(() => {
  if (!serverStore.activeServerId) return;

  // Load saved queries (local file read)
  queryStore.loadSavedQueries(serverStore.activeServerId).catch(console.error);

  // Load stored queries (server-side STORED_QUERY definitions)
  queryStore.loadStoredQueries(serverStore.activeServerId);

  // Load templates (network request for dropdown)
  templateStore.fetchTemplates(serverStore.activeServerId).catch(console.error);
});

// Reload when server changes
watch(
  () => serverStore.activeServerId,
  (serverId) => {
    if (serverId) {
      queryStore.loadSavedQueries(serverId).catch(console.error);
      queryStore.loadStoredQueries(serverId);
      templateStore.fetchTemplates(serverId).catch(console.error);
      queryStore.clearSelectedStoredQuery();
    }
  },
);

// Stored query (server-side) selection + execution
const selectedStoredParams = ref<Record<string, string>>({});

const storedQueryParamNames = computed(() => {
  const q = queryStore.selectedStoredQuery?.q ?? "";
  const names = new Set<string>();
  const re = /\$([a-zA-Z_]\w*)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(q))) names.add(m[1]);
  return Array.from(names);
});

async function selectStoredQuery(sq: StoredQuerySummary) {
  if (!serverStore.activeServerId) return;
  const isSame =
    queryStore.selectedStoredQuery?.qualified_query_name === sq.qualified_query_name &&
    queryStore.selectedStoredQuery?.version === sq.version;
  if (isSame) {
    queryStore.clearSelectedStoredQuery();
    return;
  }
  selectedStoredParams.value = {};
  await queryStore.loadStoredQueryDefinition(
    serverStore.activeServerId,
    sq.qualified_query_name,
    sq.version,
  );
  void analytics.track("aql_stored_query_viewed");
}

async function runStoredQuery() {
  if (!serverStore.activeServerId || !queryStore.selectedStoredQuery) return;
  const parameters: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(selectedStoredParams.value)) {
    if (value.trim() !== "") parameters[key] = value;
  }
  await queryStore.executeStoredQuery(
    serverStore.activeServerId,
    queryStore.selectedStoredQuery.qualified_query_name,
    queryStore.selectedStoredQuery.version,
    parameters,
  );
  // Feature-adoption ping only — NEVER include the query name or parameters.
  void analytics.track("aql_stored_query_executed");
}

// Build path index when context template changes
watch(contextTemplateId, async (templateId) => {
  if (!templateId || !serverStore.activeServerId) {
    templatePaths.value = new Map();
    allTemplatePaths.value = [];
    return;
  }

  await templateStore.fetchWebTemplate(serverStore.activeServerId, templateId);

  if (templateStore.selectedWebTemplate) {
    const allPaths = extractAqlPathIndex(templateStore.selectedWebTemplate);
    allTemplatePaths.value = allPaths;

    // Also build per-archetype maps by finding archetype nodes in the web template
    const pathMap = new Map<string, AqlPathEntry[]>();
    const tree = templateStore.selectedWebTemplate.tree as Record<string, unknown> | undefined;
    if (tree) {
      collectArchetypePaths(tree, pathMap);
    }
    templatePaths.value = pathMap;
  }
});

function collectArchetypePaths(
  node: Record<string, unknown>,
  pathMap: Map<string, AqlPathEntry[]>,
): void {
  const nodeId = node.id as string | undefined;
  if (nodeId && nodeId.startsWith("openEHR-") && templateStore.selectedWebTemplate) {
    const paths = extractAqlPathsForArchetype(templateStore.selectedWebTemplate, nodeId);
    if (paths.length > 0) {
      pathMap.set(nodeId, paths);
    }
  }
  const children = node.children as Record<string, unknown>[] | undefined;
  if (children) {
    for (const child of children) {
      collectArchetypePaths(child, pathMap);
    }
  }
}

async function runQuery() {
  if (!serverStore.activeServerId || !queryText.value.trim()) return;
  await queryStore.executeAql(serverStore.activeServerId, queryText.value);
  // Feature-adoption ping only — NEVER include the query text itself.
  void analytics.track("aql_executed");
}

function replayTour() {
  void analytics.track("tour_replayed", { tour_id: "aql" });
  tourStore.start("aql");
}

function formatQuery() {
  editorRef.value?.format();
}

async function saveCurrentQuery() {
  if (!saveName.value.trim()) return;
  const query: SavedQuery = {
    id: crypto.randomUUID(),
    name: saveName.value,
    query: queryText.value,
    server_id: serverStore.activeServerId,
    created_at: new Date().toISOString(),
  };
  await queryStore.saveQuery(query);
  showSaveDialog.value = false;
  saveName.value = "";
  // Count only — never the query name or body.
  void analytics.track("aql_query_saved");
}

function loadQuery(query: SavedQuery) {
  queryText.value = query.query;
  void analytics.track("aql_query_loaded");
}

async function deleteSavedQuery(id: string) {
  await queryStore.deleteSavedQuery(id);
  void analytics.track("aql_query_deleted");
}

function exportCsv() {
  if (!queryStore.result) return;
  // Feature-usage signal only — row counts intentionally omitted because
  // they correlate with dataset size on customer servers.
  void analytics.track("aql_results_exported");

  const headers = queryStore.result.columns.map((c) => c.path || c.name);
  const rows = queryStore.result.rows.map((row, idx) => {
    const rowWithNumber = [
      String(idx + 1),
      ...row.map((cell) => {
        const val = typeof cell === "object" ? JSON.stringify(cell) : String(cell ?? "");
        return `"${val.replace(/"/g, '""')}"`;
      }),
    ];
    return rowWithNumber;
  });

  const csv = [["#", ...headers].join(","), ...rows.map((r) => r.join(","))].join("\n");
  const blob = new Blob([csv], { type: "text/csv" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = "aql_results.csv";
  a.click();
  URL.revokeObjectURL(url);
}

function formatCellValue(value: unknown): string {
  if (value === null || value === undefined) return "";
  if (typeof value === "object") return JSON.stringify(value);
  return String(value);
}

const isComplex = (value: unknown): boolean => typeof value === "object" && value !== null;

// Resize drag handle
function startResize(event: MouseEvent) {
  event.preventDefault();
  isResizing.value = true;
  const startY = event.clientY;
  const startHeight = editorHeight.value;

  function onMouseMove(e: MouseEvent) {
    const delta = e.clientY - startY;
    const newHeight = Math.max(120, startHeight + delta);
    editorHeight.value = newHeight;
  }

  function onMouseUp() {
    isResizing.value = false;
    document.removeEventListener("mousemove", onMouseMove);
    document.removeEventListener("mouseup", onMouseUp);
  }

  document.addEventListener("mousemove", onMouseMove);
  document.addEventListener("mouseup", onMouseUp);
}

const editorStyle = computed(() => ({
  height: `${editorHeight.value}px`,
}));
</script>

<template>
  <div class="aql-runner">
    <div class="runner-layout">
      <!-- Left: Saved queries -->
      <div class="saved-panel" data-tour="aql-saved-queries">
        <div class="panel-header">
          <h3>Saved Queries <span class="panel-header-qualifier">(local)</span></h3>
        </div>
        <div ref="savedListEl" class="saved-list" @scroll="onSavedScroll">
          <div
            class="saved-list-inner"
            :style="{
              paddingTop: `${savedTopPadding}px`,
              paddingBottom: `${savedBottomPadding}px`,
            }"
          >
            <div
              v-for="sq in visibleSavedQueries"
              :key="sq.id"
              class="saved-item"
              @click="loadQuery(sq)"
            >
              <div class="saved-name">{{ sq.name }}</div>
              <DeleteButton title="Delete saved query" @click.stop="deleteSavedQuery(sq.id)" />
            </div>
          </div>
          <div v-if="queryStore.savedQueries.length === 0" class="empty-state">
            <p>No saved queries yet.</p>
          </div>
        </div>

        <div class="panel-header" data-tour="aql-stored-queries">
          <h3>Stored Queries <span class="panel-header-qualifier">(server)</span></h3>
        </div>
        <div ref="storedListEl" class="saved-list" @scroll="onStoredScroll">
          <div v-if="queryStore.storedQueriesLoading" class="empty-state">
            <p>Loading…</p>
          </div>
          <div v-else-if="queryStore.storedQueriesError" class="empty-state">
            <p>Not available on this server.</p>
          </div>
          <template v-else>
            <div
              class="saved-list-inner"
              :style="{
                paddingTop: `${storedTopPadding}px`,
                paddingBottom: `${storedBottomPadding}px`,
              }"
            >
              <div
                v-for="sq in visibleStoredQueries"
                :key="sq.qualified_query_name + (sq.version || '')"
                class="saved-item"
                :class="{
                  active:
                    queryStore.selectedStoredQuery?.qualified_query_name ===
                      sq.qualified_query_name &&
                    queryStore.selectedStoredQuery?.version === sq.version,
                }"
                @click="selectStoredQuery(sq)"
              >
                <div class="saved-name">
                  {{ sq.qualified_query_name }}
                  <span v-if="sq.version" class="stored-version">v{{ sq.version }}</span>
                </div>
              </div>
            </div>
            <div v-if="queryStore.storedQueries.length === 0" class="empty-state">
              <p>No stored queries on this server.</p>
            </div>
          </template>
        </div>
      </div>

      <!-- Main area -->
      <div class="main-area">
        <!-- Stored query detail / execute panel -->
        <div
          v-if="
            queryStore.selectedStoredQuery ||
            queryStore.selectedStoredQueryLoading ||
            queryStore.selectedStoredQueryError
          "
          class="stored-query-panel"
        >
          <div v-if="queryStore.selectedStoredQueryLoading" class="stored-query-status">
            Loading definition…
          </div>
          <div v-else-if="queryStore.selectedStoredQueryError" class="stored-query-header">
            <span class="error-msg">{{ queryStore.selectedStoredQueryError }}</span>
            <button type="button" class="btn btn-sm" @click="queryStore.clearSelectedStoredQuery">
              Close
            </button>
          </div>
          <template v-else-if="queryStore.selectedStoredQuery">
            <div class="stored-query-header">
              <h3>
                {{ queryStore.selectedStoredQuery.qualified_query_name }}
                <span v-if="queryStore.selectedStoredQuery.version" class="stored-version">
                  v{{ queryStore.selectedStoredQuery.version }}
                </span>
              </h3>
              <button type="button" class="btn btn-sm" @click="queryStore.clearSelectedStoredQuery">
                Close
              </button>
            </div>
            <pre class="stored-query-aql">{{
              queryStore.selectedStoredQuery.q || "(no AQL text returned by server)"
            }}</pre>
            <div v-if="storedQueryParamNames.length" class="stored-query-params">
              <div v-for="p in storedQueryParamNames" :key="p" class="stored-query-param">
                <label :for="`stored-param-${p}`">${{ p }}</label>
                <input
                  :id="`stored-param-${p}`"
                  class="input"
                  v-model="selectedStoredParams[p]"
                  :placeholder="`Value for $${p}`"
                />
              </div>
            </div>
            <div class="stored-query-actions">
              <button type="button" class="btn btn-sm btn-primary" @click="runStoredQuery">
                Execute
              </button>
            </div>
          </template>
        </div>

        <!-- Editor -->
        <div class="editor-section">
          <div class="editor-header">
            <h2>AQL Query</h2>
            <div class="editor-actions">
              <button
                type="button"
                class="tour-trigger-btn"
                title="Take a tour of the AQL Runner"
                @click="replayTour"
              >
                <CompassIcon />
              </button>
              <button
                type="button"
                class="btn btn-sm"
                data-tour="aql-format"
                @click="formatQuery"
                title="Format query (Shift+Alt+F)"
              >
                Format
              </button>
              <button type="button" class="btn btn-sm" @click="showSaveDialog = !showSaveDialog">
                Save
              </button>
              <button
                type="button"
                class="btn btn-sm btn-primary"
                data-tour="aql-run"
                @click="runQuery"
              >
                Run (Ctrl+Enter)
              </button>
            </div>
          </div>

          <div v-if="showSaveDialog" class="save-dialog">
            <input
              class="input"
              v-model="saveName"
              placeholder="Query name..."
              @keydown.enter="saveCurrentQuery"
            />
            <button type="button" class="btn btn-sm btn-primary" @click="saveCurrentQuery">
              Save
            </button>
            <button type="button" class="btn btn-sm" @click="showSaveDialog = false">Cancel</button>
          </div>

          <!-- Context Template selector (Layer 3) -->
          <div class="context-template-bar" data-tour="aql-context-template">
            <label class="context-template-label">Context Template</label>
            <SearchableSelect
              v-model="contextTemplateId"
              class="context-template-select"
              :options="contextTemplateOptions"
              placeholder="— No template context —"
              search-placeholder="Search templates..."
              clearable
            />
          </div>

          <div class="editor-wrapper" :style="editorStyle">
            <AqlEditor
              ref="editorRef"
              v-model="queryText"
              :template-paths="templatePaths"
              :all-template-paths="allTemplatePaths"
              @execute="runQuery"
            />
          </div>

          <!-- Resize handle -->
          <div class="resize-handle" @mousedown="startResize">
            <div class="resize-grip"></div>
          </div>
        </div>

        <!-- Results -->
        <div class="results-section">
          <div v-if="queryStore.loading" class="loading">Executing query...</div>
          <div v-else-if="queryStore.error" class="error-msg">{{ queryStore.error }}</div>
          <div v-else-if="queryStore.result" class="results">
            <div class="results-header">
              <span class="results-info">
                {{ queryStore.result.total_count }} rows in
                {{ queryStore.result.execution_time_ms }}ms
              </span>
              <button type="button" class="btn btn-sm" @click="exportCsv">Export CSV</button>
            </div>

            <div class="results-table-wrapper">
              <table class="results-table">
                <thead>
                  <tr>
                    <th class="row-number-header">#</th>
                    <th v-for="col in queryStore.result.columns" :key="col.name">
                      {{ col.path || col.name }}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(row, i) in queryStore.result.rows" :key="i">
                    <td class="row-number-cell">{{ i + 1 }}</td>
                    <td v-for="(cell, j) in row" :key="j">
                      <details v-if="isComplex(cell)">
                        <summary class="cell-summary">
                          {{ JSON.stringify(cell).substring(0, 40) }}...
                        </summary>
                        <div class="cell-detail">
                          <JsonViewer :value="cell" :show-line-numbers="false" />
                        </div>
                      </details>
                      <span v-else>{{ formatCellValue(cell) }}</span>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>

          <div v-else class="empty-state">
            <h3>Run a query</h3>
            <p>Write an AQL query above and press Ctrl+Enter to execute.</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.aql-runner {
  height: 100%;
}

.runner-layout {
  display: flex;
  height: 100%;
}

.saved-panel {
  width: 220px;
  min-width: 220px;
  border-right: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.panel-header {
  padding: 16px;
  border-bottom: 1px solid var(--color-border);
}
.panel-header h3 {
  font-size: 13px;
  font-weight: 600;
}
.panel-header-qualifier {
  font-weight: 400;
  font-size: 11px;
  color: var(--color-text-muted);
}

/* Each list gets an equal, independently-scrolling share of the panel
   (rather than one shared panel-level scrollbar) so a long "Stored Queries"
   list — potentially hundreds/thousands of server-defined queries — doesn't
   push the "Saved Queries" section out of view above it. Virtualized via
   useVirtualList, so `.saved-list-inner`'s padding stands in for rows
   scrolled out of the rendered window. */
.saved-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.saved-list-inner {
  display: flex;
  flex-direction: column;
}

.saved-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  box-sizing: border-box;
  height: 34px;
  padding: 0 16px;
  border-bottom: 1px solid var(--color-border);
  cursor: pointer;
  transition: background 0.15s;
}
.saved-item:hover {
  background: var(--color-surface);
}
.saved-item.active {
  background: var(--color-surface);
  box-shadow: inset 2px 0 0 var(--color-primary);
}
.saved-name {
  font-size: 13px;
}
.stored-version {
  margin-left: 6px;
  padding: 1px 6px;
  font-size: 10px;
  font-weight: 600;
  color: var(--color-text-muted);
  background: var(--color-bg-secondary);
  border-radius: var(--radius);
}

/* Stored query (server-side) detail / execute panel */
.stored-query-panel {
  flex-shrink: 0;
  padding: 12px 24px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-surface);
}
.stored-query-status {
  font-size: 13px;
  color: var(--color-text-secondary);
}
.stored-query-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.stored-query-header h3 {
  font-size: 14px;
  font-weight: 600;
}
.stored-query-aql {
  margin: 0 0 8px;
  padding: 8px 12px;
  font-family: var(--font-mono);
  font-size: 12px;
  white-space: pre-wrap;
  overflow-wrap: break-word;
  max-height: 160px;
  overflow-y: auto;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
}
.stored-query-params {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 8px;
}
.stored-query-param {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 160px;
}
.stored-query-param label {
  font-size: 11px;
  font-weight: 500;
  color: var(--color-text-muted);
  font-family: var(--font-mono);
}
.stored-query-actions {
  display: flex;
  justify-content: flex-end;
}

.main-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.editor-section {
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
}

.editor-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 24px;
  border-bottom: 1px solid var(--color-border);
}
.editor-header h2 {
  font-size: 16px;
  font-weight: 600;
}
.editor-actions {
  display: flex;
  gap: 8px;
}

.save-dialog {
  display: flex;
  gap: 8px;
  padding: 8px 24px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-surface);
}
.save-dialog .input {
  flex: 1;
}

/* Context Template bar */
.context-template-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 24px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-secondary);
}

.context-template-label {
  font-size: 11px;
  font-weight: 500;
  color: var(--color-text-muted);
  white-space: nowrap;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.context-template-select {
  flex: 1;
  min-width: 0;
}

.context-template-select :deep(.searchable-select-control) {
  padding: 3px 8px;
  font-size: 12px;
  font-family: var(--font-mono);
  background: var(--color-bg);
}

.context-template-select :deep(.searchable-select-control.no-selection) {
  border-style: dashed;
}

.editor-wrapper {
  background: var(--color-bg);
  overflow: hidden;
}

/* Resize handle */
.resize-handle {
  height: 6px;
  cursor: ns-resize;
  background: var(--color-bg-secondary);
  border-top: 1px solid var(--color-border);
  border-bottom: 1px solid var(--color-border);
  display: flex;
  align-items: center;
  justify-content: center;
}

.resize-grip {
  width: 32px;
  height: 2px;
  border-radius: 1px;
  background: var(--color-text-muted);
  opacity: 0.4;
}

.resize-handle:hover .resize-grip {
  opacity: 0.8;
}

.results-section {
  flex: 1;
  overflow: auto;
}

.results-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 24px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg);
  position: sticky;
  top: 0;
  z-index: 1;
}
.results-info {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.results-table-wrapper {
  overflow: auto;
}

.results-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}
.results-table th {
  padding: 8px 16px;
  text-align: left;
  font-weight: 600;
  font-size: 12px;
  color: var(--color-text-secondary);
  background: var(--color-surface);
  border-bottom: 2px solid var(--color-border);
  position: sticky;
  top: 0;
  z-index: 1;
}
.results-table .row-number-header {
  width: 60px;
  text-align: center;
  background: var(--color-bg-secondary);
}
.results-table td {
  padding: 6px 16px;
  border-bottom: 1px solid var(--color-border);
  font-family: var(--font-mono);
  font-size: 12px;
  max-width: 400px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.results-table .row-number-cell {
  text-align: center;
  color: var(--color-text-muted);
  background: var(--color-bg-secondary);
  font-weight: 600;
  position: sticky;
  left: 0;
}
.results-table tr:hover td {
  background: var(--color-surface);
}
.results-table tr:hover .row-number-cell {
  background: var(--color-bg-secondary);
}

.cell-summary {
  cursor: pointer;
  color: var(--color-text-secondary);
}
.cell-detail {
  margin-top: 8px;
  font-size: 11px;
  max-height: 200px;
  overflow-y: auto;
}
</style>
