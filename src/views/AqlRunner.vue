<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useServerStore } from "../stores/server";
import { useQueryStore, type SavedQuery } from "../stores/query";
import AqlEditor from "../components/AqlEditor.vue";

const serverStore = useServerStore();
const queryStore = useQueryStore();

const queryText = ref(
  "SELECT e/ehr_id/value, c/uid/value, c/name/value\nFROM EHR e\nCONTAINS COMPOSITION c\nLIMIT 20",
);
const saveName = ref("");
const showSaveDialog = ref(false);

onMounted(() => {
  if (serverStore.activeServerId) {
    queryStore.loadSavedQueries(serverStore.activeServerId);
  }
});

async function runQuery() {
  if (!serverStore.activeServerId || !queryText.value.trim()) return;
  await queryStore.executeAql(serverStore.activeServerId, queryText.value);
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
}

function loadQuery(query: SavedQuery) {
  queryText.value = query.query;
}

async function deleteSavedQuery(id: string) {
  await queryStore.deleteSavedQuery(id);
}

function exportCsv() {
  if (!queryStore.result) return;

  const headers = queryStore.result.columns.map((c) => c.path || c.name);
  const rows = queryStore.result.rows.map((row, idx) => {
    const rowWithNumber = [String(idx + 1), ...row.map((cell) => {
      const val = typeof cell === "object" ? JSON.stringify(cell) : String(cell ?? "");
      return `"${val.replace(/"/g, '""')}"`;
    })];
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

function handleKeydown(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
    event.preventDefault();
    runQuery();
  }
}

function formatCellValue(value: unknown): string {
  if (value === null || value === undefined) return "";
  if (typeof value === "object") return JSON.stringify(value);
  return String(value);
}

const isComplex = (value: unknown): boolean => typeof value === "object" && value !== null;
</script>

<template>
  <div class="aql-runner">
    <div class="runner-layout">
      <!-- Left: Saved queries -->
      <div class="saved-panel">
        <div class="panel-header">
          <h3>Saved Queries</h3>
        </div>
        <div class="saved-list">
          <div
            v-for="sq in queryStore.savedQueries"
            :key="sq.id"
            class="saved-item"
            @click="loadQuery(sq)"
          >
            <div class="saved-name">{{ sq.name }}</div>
            <button class="copy-btn" @click.stop="deleteSavedQuery(sq.id)" title="Delete">X</button>
          </div>
          <div v-if="queryStore.savedQueries.length === 0" class="empty-state">
            <p>No saved queries yet.</p>
          </div>
        </div>
      </div>

      <!-- Main area -->
      <div class="main-area">
        <!-- Editor -->
        <div class="editor-section">
          <div class="editor-header">
            <h2>AQL Query</h2>
            <div class="editor-actions">
              <button class="btn btn-sm" @click="showSaveDialog = !showSaveDialog">Save</button>
              <button class="btn btn-sm btn-primary" @click="runQuery">Run (Ctrl+Enter)</button>
            </div>
          </div>

          <div v-if="showSaveDialog" class="save-dialog">
            <input
              class="input"
              v-model="saveName"
              placeholder="Query name..."
              @keydown.enter="saveCurrentQuery"
            />
            <button class="btn btn-sm btn-primary" @click="saveCurrentQuery">Save</button>
            <button class="btn btn-sm" @click="showSaveDialog = false">Cancel</button>
          </div>

          <div class="editor-wrapper">
            <AqlEditor v-model="queryText" @keydown="handleKeydown" />
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
              <button class="btn btn-sm" @click="exportCsv">Export CSV</button>
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
                        <pre class="cell-detail">{{ JSON.stringify(cell, null, 2) }}</pre>
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
  overflow-y: auto;
}

.panel-header {
  padding: 16px;
  border-bottom: 1px solid var(--color-border);
}
.panel-header h3 {
  font-size: 13px;
  font-weight: 600;
}

.saved-list {
  flex: 1;
}

.saved-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 16px;
  border-bottom: 1px solid var(--color-border);
  cursor: pointer;
  transition: background 0.15s;
}
.saved-item:hover {
  background: var(--color-surface);
}
.saved-name {
  font-size: 13px;
}

.main-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.editor-section {
  border-bottom: 1px solid var(--color-border);
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

.editor-wrapper {
  background: var(--color-bg-secondary);
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
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 200px;
  overflow-y: auto;
}
</style>
