<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from "vue";
import {
  useInspectorStore,
  extractPath,
  generateCurl,
  formatTimestamp,
  statusClass,
  methodClass,
} from "../stores/inspector";
import type { RequestLogEntry } from "../stores/inspector";
import JsonTreeNode from "./JsonTreeNode.vue";

type DrawerState = "collapsed" | "half" | "expanded";

const store = useInspectorStore();

const drawerState = ref<DrawerState>(
  (localStorage.getItem("inspector-drawer-state") as DrawerState) || "collapsed",
);
const detailTab = ref<"request" | "response">("response");
const bodyViewTab = ref(localStorage.getItem("inspector-body-tab") || "tree");
const treeSearch = ref("");
const showClearConfirm = ref(false);
const flatFilter = ref("");

// Persist drawer state
watch(drawerState, (s) => localStorage.setItem("inspector-drawer-state", s));
watch(bodyViewTab, (t) => localStorage.setItem("inspector-body-tab", t));

// Auto-scroll log when new entry arrives
const logListRef = ref<HTMLElement | null>(null);
watch(
  () => store.entries.length,
  async () => {
    await nextTick();
    if (logListRef.value) {
      logListRef.value.scrollTop = 0;
    }
  },
);

// Keyboard shortcut: Ctrl+Shift+L
function handleKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "L") {
    e.preventDefault();
    toggleDrawer();
  }
}

onMounted(() => {
  document.addEventListener("keydown", handleKeydown);
});

onUnmounted(() => {
  document.removeEventListener("keydown", handleKeydown);
});

function toggleDrawer() {
  if (drawerState.value === "collapsed") drawerState.value = "half";
  else if (drawerState.value === "half") drawerState.value = "expanded";
  else drawerState.value = "collapsed";
}

function cycleUp() {
  if (drawerState.value === "collapsed") drawerState.value = "half";
  else if (drawerState.value === "half") drawerState.value = "expanded";
}

function cycleDown() {
  if (drawerState.value === "expanded") drawerState.value = "half";
  else if (drawerState.value === "half") drawerState.value = "collapsed";
}

const drawerHeight = computed(() => {
  switch (drawerState.value) {
    case "collapsed":
      return "32px";
    case "half":
      return "40vh";
    case "expanded":
      return "calc(100vh - 32px)";
    default:
      return "32px";
  }
});

const selected = computed(() => store.selectedEntry);

// Parse body JSON
function tryParseJson(body: string | null): unknown | null {
  if (!body) return null;
  try {
    return JSON.parse(body);
  } catch {
    return null;
  }
}

// Check if response is XML
function isXmlResponse(entry: RequestLogEntry | null): boolean {
  if (!entry) return false;
  const contentType = entry.response_headers["content-type"] || "";
  return contentType.includes("application/xml") || contentType.includes("text/xml");
}

// Format XML with syntax highlighting
function formatXml(xml: string): string {
  // Basic XML formatting with indentation
  let formatted = "";
  let indent = 0;
  const lines = xml.split(/>\s*</);

  lines.forEach((line, index) => {
    if (index > 0) line = "<" + line;
    if (index < lines.length - 1) line = line + ">";

    // Detect closing tags
    if (line.match(/^<\/\w/)) indent--;

    // Add indentation
    formatted += "  ".repeat(Math.max(0, indent)) + line + "\n";

    // Detect opening tags (not self-closing)
    if (line.match(/^<\w[^>]*[^\/]>$/)) indent++;
  });

  return formatted.trim();
}

// Apply syntax highlighting to XML
function highlightXml(xml: string): string {
  return xml
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(
      /(&lt;\/?)([\w-]+)([\s\S]*?)(&gt;)/g,
      (_match, openBracket, tagName, content, closeBracket) => {
        // Highlight tag names
        const highlightedTag = `${openBracket}<span class="xml-tag">${tagName}</span>`;
        // Highlight attributes
        const highlightedContent = content.replace(
          /([\w-]+)=(["'])(.*?)\2/g,
          '<span class="xml-attr-name">$1</span>=<span class="xml-attr-value">$2$3$2</span>',
        );
        return `${highlightedTag}${highlightedContent}${closeBracket}`;
      },
    )
    .replace(/(&lt;!--[\s\S]*?--&gt;)/g, '<span class="xml-comment">$1</span>')
    .replace(/(&lt;\?[\s\S]*?\?&gt;)/g, '<span class="xml-declaration">$1</span>');
}

const responseJson = computed(() =>
  selected.value ? tryParseJson(selected.value.response_body) : null,
);
const requestJson = computed(() =>
  selected.value ? tryParseJson(selected.value.request_body) : null,
);

const isResponseXml = computed(() => isXmlResponse(selected.value));
const formattedXml = computed(() => {
  if (!selected.value?.response_body || !isResponseXml.value) return "";
  return formatXml(selected.value.response_body);
});

// Auto-switch to XML tab when XML response is detected
watch(isResponseXml, (isXml) => {
  if (isXml && (bodyViewTab.value === "tree" || bodyViewTab.value === "flat")) {
    bodyViewTab.value = "xml";
  }
});

// FLAT view: detect if response is a FLAT composition (flat key-value object)
// or a structured COMPOSITION that could be flattened
const flatEntries = computed(() => {
  if (!responseJson.value || typeof responseJson.value !== "object") return null;
  const obj = responseJson.value as Record<string, unknown>;

  // Check if this looks like a FLAT composition (keys are path-like)
  const keys = Object.keys(obj);
  const looksFlat = keys.some(
    (k) => k.includes("/") && (k.includes("openEHR") || k.includes("ctx/")),
  );
  if (looksFlat) {
    return keys
      .filter((k) => {
        if (!flatFilter.value) return true;
        return k.toLowerCase().includes(flatFilter.value.toLowerCase());
      })
      .map((k) => ({ path: k, value: String(obj[k]) }));
  }

  // Check if it's a structured COMPOSITION
  if (obj._type === "COMPOSITION") {
    return null; // Would need Web Template to derive FLAT paths
  }

  return null;
});

const showFlatTab = computed(() => {
  if (!responseJson.value || typeof responseJson.value !== "object") return false;
  const obj = responseJson.value as Record<string, unknown>;
  const keys = Object.keys(obj);
  const looksFlat = keys.some(
    (k) => k.includes("/") && (k.includes("openEHR") || k.includes("ctx/")),
  );
  return looksFlat || obj._type === "COMPOSITION";
});

function responseBodySize(entry: RequestLogEntry): string {
  if (!entry.response_body) return "0 B";
  const bytes = new TextEncoder().encode(entry.response_body).length;
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

async function copyToClipboard(text: string) {
  await navigator.clipboard.writeText(text);
}

async function copyCurl() {
  if (!selected.value) return;
  await copyToClipboard(generateCurl(selected.value));
}

function confirmClear() {
  showClearConfirm.value = true;
}

function doClear() {
  store.clear();
  showClearConfirm.value = false;
}
</script>

<template>
  <div class="inspector-drawer" :style="{ height: drawerHeight }">
    <!-- Header bar -->
    <div class="inspector-header" @click="toggleDrawer">
      <div class="header-left">
        <span class="drawer-icon">{{ drawerState === "collapsed" ? "\u25B2" : "\u25BC" }}</span>
        <span class="header-title">Request Inspector</span>
        <span v-if="store.entries.length > 0" class="entry-count-badge">
          {{ store.entries.length }}
        </span>
        <span v-if="store.hasErrors && drawerState === 'collapsed'" class="error-dot" />
      </div>
      <div class="header-actions" @click.stop>
        <button v-if="store.entries.length > 0" class="btn btn-sm" @click="confirmClear">
          Clear
        </button>
        <div v-if="showClearConfirm" class="clear-confirm">
          <span>Clear request history?</span>
          <button class="btn btn-sm btn-danger" @click="doClear">Yes</button>
          <button class="btn btn-sm" @click="showClearConfirm = false">No</button>
        </div>
        <button class="btn btn-sm" @click="cycleUp" title="Expand">&#x2B06;</button>
        <button class="btn btn-sm" @click="cycleDown" title="Collapse">&#x2B07;</button>
      </div>
    </div>

    <!-- Content (visible when not collapsed) -->
    <div v-if="drawerState !== 'collapsed'" class="inspector-content">
      <!-- Left: Log List -->
      <div class="log-pane">
        <div class="log-filters">
          <input
            v-model="store.filterText"
            class="input filter-input"
            placeholder="Filter by URL..."
          />
        </div>
        <div ref="logListRef" class="log-list">
          <div
            v-for="entry in store.filteredEntries"
            :key="entry.id"
            class="log-entry"
            :class="{ selected: entry.id === store.selectedId }"
            @click="store.selectEntry(entry.id)"
          >
            <span :class="['method-badge', methodClass(entry.method)]">
              {{ entry.method }}
            </span>
            <span class="entry-path" :title="entry.url">
              {{ extractPath(entry.url) }}
            </span>
            <span :class="['entry-status', statusClass(entry.status)]">
              {{ entry.status }}
            </span>
            <span class="entry-duration">{{ entry.duration_ms }}ms</span>
          </div>
          <div v-if="store.filteredEntries.length === 0" class="log-empty">
            {{ store.entries.length === 0 ? "No requests yet" : "No matches" }}
          </div>
        </div>
      </div>

      <!-- Right: Detail Panel -->
      <div class="detail-pane">
        <template v-if="selected">
          <!-- Tabs: Request / Response -->
          <div class="detail-tabs">
            <button
              :class="['tab-btn', { active: detailTab === 'request' }]"
              @click="detailTab = 'request'"
            >
              Request
            </button>
            <button
              :class="['tab-btn', { active: detailTab === 'response' }]"
              @click="detailTab = 'response'"
            >
              Response
            </button>
          </div>

          <div class="detail-body">
            <!-- REQUEST TAB -->
            <template v-if="detailTab === 'request'">
              <div class="detail-section">
                <div class="section-header">Summary</div>
                <div class="summary-grid">
                  <span class="summary-label">Method</span>
                  <span :class="['method-badge', methodClass(selected.method)]">{{
                    selected.method
                  }}</span>
                  <span class="summary-label">URL</span>
                  <span class="summary-value mono">{{ selected.url }}</span>
                  <span class="summary-label">Duration</span>
                  <span class="summary-value">{{ selected.duration_ms }}ms</span>
                  <span class="summary-label">Time</span>
                  <span class="summary-value">{{ formatTimestamp(selected.timestamp_ms) }}</span>
                </div>
              </div>

              <div class="detail-section">
                <div class="section-header">
                  Request Headers
                  <span class="header-count"
                    >({{ Object.keys(selected.request_headers).length }})</span
                  >
                </div>
                <table class="headers-table">
                  <tr v-for="(val, key) in selected.request_headers" :key="key">
                    <td class="header-key">{{ key }}</td>
                    <td class="header-value">
                      {{ val }}
                      <button
                        class="copy-btn"
                        @click="copyToClipboard(`${key}: ${val}`)"
                        title="Copy"
                      >
                        copy
                      </button>
                    </td>
                  </tr>
                </table>
              </div>

              <div v-if="selected.request_body" class="detail-section">
                <div class="section-header">Request Body</div>
                <div class="body-view-tabs">
                  <button
                    :class="['tab-btn-sm', { active: bodyViewTab === 'tree' }]"
                    @click="bodyViewTab = 'tree'"
                  >
                    Tree
                  </button>
                  <button
                    :class="['tab-btn-sm', { active: bodyViewTab === 'raw' }]"
                    @click="bodyViewTab = 'raw'"
                  >
                    Raw
                  </button>
                </div>
                <div v-if="bodyViewTab === 'tree' && requestJson" class="tree-container">
                  <JsonTreeNode label="root" :value="requestJson" :depth="0" />
                </div>
                <pre v-else class="raw-body">{{ selected.request_body }}</pre>
              </div>

              <div class="detail-section">
                <button class="btn btn-sm" @click="copyCurl">Copy as curl</button>
              </div>
            </template>

            <!-- RESPONSE TAB -->
            <template v-if="detailTab === 'response'">
              <div class="detail-section">
                <div class="section-header">Summary</div>
                <div class="summary-grid">
                  <span class="summary-label">Status</span>
                  <span :class="['summary-value', statusClass(selected.status)]">
                    {{ selected.status }}
                  </span>
                  <span class="summary-label">Content-Type</span>
                  <span class="summary-value mono">{{
                    selected.response_headers["content-type"] || "N/A"
                  }}</span>
                  <span class="summary-label">Size</span>
                  <span class="summary-value">{{ responseBodySize(selected) }}</span>
                </div>
                <div v-if="selected.body_truncated" class="truncation-notice">
                  Response body truncated (exceeds 2 MB limit)
                </div>
              </div>

              <div class="detail-section">
                <div class="section-header">
                  Response Headers
                  <span class="header-count"
                    >({{ Object.keys(selected.response_headers).length }})</span
                  >
                </div>
                <table class="headers-table">
                  <tr v-for="(val, key) in selected.response_headers" :key="key">
                    <td class="header-key">{{ key }}</td>
                    <td class="header-value">
                      {{ val }}
                      <button
                        class="copy-btn"
                        @click="copyToClipboard(`${key}: ${val}`)"
                        title="Copy"
                      >
                        copy
                      </button>
                    </td>
                  </tr>
                </table>
              </div>

              <div v-if="selected.response_body" class="detail-section">
                <div class="section-header">Response Body</div>
                <div class="body-view-tabs">
                  <button
                    v-if="!isResponseXml"
                    :class="['tab-btn-sm', { active: bodyViewTab === 'tree' }]"
                    @click="bodyViewTab = 'tree'"
                  >
                    Tree
                  </button>
                  <button
                    v-if="isResponseXml"
                    :class="['tab-btn-sm', { active: bodyViewTab === 'xml' }]"
                    @click="bodyViewTab = 'xml'"
                  >
                    XML
                  </button>
                  <button
                    :class="['tab-btn-sm', { active: bodyViewTab === 'raw' }]"
                    @click="bodyViewTab = 'raw'"
                  >
                    Raw
                  </button>
                  <button
                    v-if="showFlatTab"
                    :class="['tab-btn-sm', { active: bodyViewTab === 'flat' }]"
                    @click="bodyViewTab = 'flat'"
                  >
                    FLAT
                  </button>
                </div>

                <!-- Tree View -->
                <div v-if="bodyViewTab === 'tree' && responseJson" class="tree-container">
                  <div class="tree-toolbar">
                    <input
                      v-model="treeSearch"
                      class="input tree-search"
                      placeholder="Search tree..."
                    />
                  </div>
                  <div class="tree-scroll">
                    <JsonTreeNode
                      label="root"
                      :value="responseJson"
                      :depth="0"
                      :search-term="treeSearch"
                    />
                  </div>
                </div>

                <!-- XML View -->
                <div v-else-if="bodyViewTab === 'xml'" class="xml-container">
                  <div class="xml-toolbar">
                    <button
                      class="btn btn-sm"
                      @click="copyToClipboard(selected.response_body || '')"
                    >
                      Copy All
                    </button>
                  </div>
                  <pre class="xml-body" v-html="highlightXml(formattedXml)"></pre>
                </div>

                <!-- Raw View -->
                <div v-else-if="bodyViewTab === 'raw'" class="raw-container">
                  <div class="raw-toolbar">
                    <button
                      class="btn btn-sm"
                      @click="copyToClipboard(selected.response_body || '')"
                    >
                      Copy All
                    </button>
                  </div>
                  <pre class="raw-body">{{
                    responseJson ? JSON.stringify(responseJson, null, 2) : selected.response_body
                  }}</pre>
                </div>

                <!-- FLAT View -->
                <div v-else-if="bodyViewTab === 'flat'" class="flat-container">
                  <template v-if="flatEntries">
                    <input
                      v-model="flatFilter"
                      class="input flat-filter"
                      placeholder="Filter by path..."
                    />
                    <table class="flat-table">
                      <tr v-for="entry in flatEntries" :key="entry.path">
                        <td class="flat-path">
                          {{ entry.path }}
                          <button
                            class="copy-btn"
                            @click="copyToClipboard(entry.path)"
                            title="Copy path"
                          >
                            copy
                          </button>
                        </td>
                        <td class="flat-value">
                          {{ entry.value }}
                          <button
                            class="copy-btn"
                            @click="copyToClipboard(entry.value)"
                            title="Copy value"
                          >
                            copy
                          </button>
                        </td>
                      </tr>
                    </table>
                  </template>
                  <div v-else class="flat-notice">
                    FLAT view requires a composition in FLAT format. Load the template and use FLAT
                    Accept header to enable this view.
                  </div>
                </div>

                <!-- Fallback for non-JSON -->
                <pre v-else class="raw-body">{{ selected.response_body }}</pre>
              </div>
            </template>
          </div>
        </template>

        <div v-else class="detail-empty">Select a request to inspect</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.inspector-drawer {
  border-top: 1px solid var(--color-border);
  background: var(--color-bg-secondary);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  transition: height 0.2s ease;
  flex-shrink: 0;
}

.inspector-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  height: 32px;
  min-height: 32px;
  background: var(--color-surface);
  border-bottom: 1px solid var(--color-border);
  cursor: pointer;
  user-select: none;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.drawer-icon {
  font-size: 10px;
  color: var(--color-text-muted);
}

.header-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-secondary);
}

.entry-count-badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 8px;
  background: var(--color-bg-tertiary);
  color: var(--color-text-secondary);
}

.error-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-error);
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.clear-confirm {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--color-text-secondary);
}

.inspector-content {
  display: flex;
  flex: 1;
  overflow: hidden;
}

/* Log Pane */
.log-pane {
  width: 35%;
  min-width: 200px;
  border-right: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.log-filters {
  padding: 6px;
  border-bottom: 1px solid var(--color-border);
}

.filter-input {
  width: 100%;
  font-size: 11px;
  padding: 4px 8px;
}

.log-list {
  flex: 1;
  overflow-y: auto;
}

.log-entry {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  cursor: pointer;
  font-size: 12px;
  border-bottom: 1px solid rgba(42, 58, 92, 0.3);
}

.log-entry:hover {
  background: var(--color-surface-hover);
}

.log-entry.selected {
  background: var(--color-bg-tertiary);
  border-left: 2px solid var(--color-primary);
}

.method-badge {
  font-size: 10px;
  font-weight: 700;
  padding: 1px 5px;
  border-radius: 3px;
  font-family: var(--font-mono);
  flex-shrink: 0;
  min-width: 42px;
  text-align: center;
}

.method-get {
  background: rgba(100, 149, 237, 0.15);
  color: #6495ed;
}
.method-post {
  background: rgba(107, 255, 142, 0.15);
  color: #6bff8e;
}
.method-put {
  background: rgba(255, 217, 61, 0.15);
  color: #ffd93d;
}
.method-delete {
  background: rgba(255, 107, 107, 0.15);
  color: #ff6b6b;
}
.method-other {
  background: var(--color-surface);
  color: var(--color-text-secondary);
}

.entry-path {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-text-secondary);
  font-family: var(--font-mono);
  font-size: 11px;
}

.entry-status {
  font-family: var(--font-mono);
  font-weight: 600;
  font-size: 11px;
  flex-shrink: 0;
}

.status-2xx {
  color: #6bff8e;
}
.status-3xx {
  color: #6495ed;
}
.status-4xx {
  color: #ffa500;
}
.status-5xx {
  color: #ff6b6b;
}
.status-other {
  color: var(--color-text-muted);
}

.entry-duration {
  color: var(--color-text-muted);
  font-size: 10px;
  flex-shrink: 0;
  min-width: 40px;
  text-align: right;
}

.log-empty {
  padding: 24px;
  text-align: center;
  color: var(--color-text-muted);
  font-size: 12px;
}

/* Detail Pane */
.detail-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.detail-tabs {
  display: flex;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-surface);
}

.tab-btn {
  padding: 6px 16px;
  border: none;
  background: none;
  color: var(--color-text-secondary);
  font-size: 12px;
  cursor: pointer;
  border-bottom: 2px solid transparent;
}

.tab-btn.active {
  color: var(--color-primary);
  border-bottom-color: var(--color-primary);
}

.tab-btn:hover {
  color: var(--color-text);
}

.detail-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px;
}

.detail-section {
  margin-bottom: 12px;
}

.section-header {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  color: var(--color-text-muted);
  margin-bottom: 6px;
  letter-spacing: 0.5px;
}

.header-count {
  font-weight: 400;
  text-transform: none;
}

.summary-grid {
  display: grid;
  grid-template-columns: 80px 1fr;
  gap: 4px 8px;
  font-size: 12px;
}

.summary-label {
  color: var(--color-text-muted);
}

.summary-value {
  color: var(--color-text);
  word-break: break-all;
}

.mono {
  font-family: var(--font-mono);
  font-size: 11px;
}

.truncation-notice {
  margin-top: 6px;
  padding: 4px 8px;
  background: rgba(255, 217, 61, 0.1);
  color: var(--color-warning);
  border-radius: 4px;
  font-size: 11px;
}

/* Headers table */
.headers-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 11px;
  font-family: var(--font-mono);
}

.headers-table tr {
  border-bottom: 1px solid rgba(42, 58, 92, 0.3);
}

.headers-table td {
  padding: 3px 6px;
  vertical-align: top;
}

.header-key {
  color: var(--color-primary);
  white-space: nowrap;
  width: 1%;
}

.header-value {
  color: var(--color-text);
  word-break: break-all;
}

/* Body view tabs */
.body-view-tabs {
  display: flex;
  gap: 2px;
  margin-bottom: 6px;
}

.tab-btn-sm {
  padding: 3px 10px;
  border: 1px solid var(--color-border);
  border-radius: 4px;
  background: none;
  color: var(--color-text-secondary);
  font-size: 11px;
  cursor: pointer;
}

.tab-btn-sm.active {
  background: var(--color-bg-tertiary);
  color: var(--color-primary);
  border-color: var(--color-primary-dim);
}

/* Tree view */
.tree-container {
  border: 1px solid var(--color-border);
  border-radius: 4px;
  background: var(--color-bg);
  overflow: hidden;
}

.tree-toolbar {
  padding: 4px 6px;
  border-bottom: 1px solid var(--color-border);
}

.tree-search {
  width: 100%;
  font-size: 11px;
  padding: 3px 6px;
}

.tree-scroll {
  max-height: 400px;
  overflow: auto;
  padding: 4px;
}

/* Raw view */
.raw-container {
  border: 1px solid var(--color-border);
  border-radius: 4px;
  background: var(--color-bg);
  overflow: hidden;
}

.raw-toolbar {
  padding: 4px 6px;
  border-bottom: 1px solid var(--color-border);
  text-align: right;
}

.raw-body {
  max-height: 400px;
  overflow: auto;
  padding: 8px;
  margin: 0;
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--color-text);
  white-space: pre-wrap;
  word-break: break-all;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: 4px;
}

/* FLAT view */
.flat-container {
  border: 1px solid var(--color-border);
  border-radius: 4px;
  background: var(--color-bg);
  padding: 6px;
}

.flat-filter {
  width: 100%;
  font-size: 11px;
  padding: 3px 6px;
  margin-bottom: 6px;
}

.flat-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 11px;
  font-family: var(--font-mono);
}

.flat-table tr {
  border-bottom: 1px solid rgba(42, 58, 92, 0.3);
}

.flat-table td {
  padding: 3px 6px;
  vertical-align: top;
}

.flat-path {
  color: var(--color-primary);
  word-break: break-all;
  max-width: 50%;
}

.flat-value {
  color: var(--color-text);
  word-break: break-all;
}

.flat-notice {
  padding: 16px;
  text-align: center;
  color: var(--color-text-muted);
  font-size: 12px;
}

.detail-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--color-text-muted);
  font-size: 13px;
}

/* XML view */
.xml-container {
  border: 1px solid var(--color-border);
  border-radius: 4px;
  background: var(--color-bg);
  overflow: hidden;
}

.xml-toolbar {
  padding: 4px 6px;
  border-bottom: 1px solid var(--color-border);
  text-align: right;
}

.xml-body {
  max-height: 400px;
  overflow: auto;
  padding: 8px;
  margin: 0;
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--color-text);
  white-space: pre;
  background: var(--color-bg);
  line-height: 1.5;
}

/* XML syntax highlighting */
.xml-body :deep(.xml-tag) {
  color: #6495ed;
  font-weight: 600;
}

.xml-body :deep(.xml-attr-name) {
  color: #ffd93d;
}

.xml-body :deep(.xml-attr-value) {
  color: #6bff8e;
}

.xml-body :deep(.xml-comment) {
  color: var(--color-text-muted);
  font-style: italic;
}

.xml-body :deep(.xml-declaration) {
  color: #ff6b6b;
}
</style>
