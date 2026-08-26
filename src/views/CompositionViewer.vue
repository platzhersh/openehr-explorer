<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted, nextTick } from "vue";
import { useRoute, useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { useServerStore } from "../stores/server";
import { useCompositionStore } from "../stores/composition";
import { useAnalytics } from "../composables/useAnalytics";
import { useTourStore } from "../stores/tour";
import CompositionTree from "../components/CompositionTree.vue";
import FlatPathPanel from "../components/FlatPathPanel.vue";
import SearchOverlay from "../components/SearchOverlay.vue";
import CompassIcon from "../components/CompassIcon.vue";

const route = useRoute();
const router = useRouter();
const serverStore = useServerStore();
const compositionStore = useCompositionStore();
const analytics = useAnalytics();
const tourStore = useTourStore();

const ehrId = computed(() => route.params.ehrId as string);
const compositionUid = computed(() => route.params.compositionUid as string);

const composition = ref<Record<string, unknown> | null>(null);
const flatComposition = ref<Record<string, unknown> | null>(null);
const webTemplate = ref<Record<string, unknown> | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);
const activeTab = ref<"pretty" | "json" | "flat">("pretty");
const showFlatPaths = ref(false);
const highlightedPath = ref<string | null>(null);
const showDeleteDialog = ref(false);
const deleting = ref(false);

// Search state
const showPanelSearch = ref(false);
const panelSearchQuery = ref("");
const currentMatchIndex = ref(0);
const searchOverlayRef = ref<InstanceType<typeof SearchOverlay> | null>(null);

watch(
  [() => serverStore.activeServerId, compositionUid],
  async ([serverId, uid]) => {
    if (!serverId || !uid) return;
    loading.value = true;
    error.value = null;
    // Track the initial composition open with the default format — subsequent
    // tab changes are tracked in the `watch(activeTab)` handler below.
    void analytics.track("composition_viewed", { format: activeTab.value });
    try {
      composition.value = await invoke("get_composition", {
        serverId,
        ehrId: ehrId.value,
        compositionUid: uid,
      });

      // Try to fetch FLAT format
      try {
        flatComposition.value = await invoke("get_composition_flat", {
          serverId,
          ehrId: ehrId.value,
          compositionUid: uid,
        });
      } catch {
        flatComposition.value = null;
      }

      // Try to fetch web template for pretty view
      const templateId =
        (composition.value as any)?.archetype_details?.template_id?.value ??
        (composition.value as any)?.archetype_node_id;
      if (templateId) {
        try {
          webTemplate.value = await invoke("get_web_template", {
            serverId,
            templateId,
          });
        } catch {
          webTemplate.value = null;
        }
      }
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  },
  { immediate: true },
);

// Search functionality
function handleKeydown(e: KeyboardEvent) {
  if (!composition.value) return;

  if ((e.ctrlKey || e.metaKey) && e.key === "f") {
    e.preventDefault();
    showPanelSearch.value = true;
    currentMatchIndex.value = 0;
    nextTick(() => searchOverlayRef.value?.focus());
  }
}

function closePanelSearch() {
  showPanelSearch.value = false;
  panelSearchQuery.value = "";
  currentMatchIndex.value = 0;
}

// Clear search when switching tabs or compositions
watch(activeTab, (tab) => {
  closePanelSearch();
  // Track which composition-view format the user prefers. `format` is a
  // known-enum string, not free text — safe to send.
  void analytics.track("composition_viewed", { format: tab });
});

watch(compositionUid, () => {
  closePanelSearch();
});

// Helper functions
function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function escapeHtml(str: string): string {
  return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function goBack() {
  router.push({ name: "ehr-detail", params: { ehrId: ehrId.value } });
}

function replayTour() {
  void analytics.track("tour_replayed", { tour_id: "composition" });
  tourStore.start("composition");
}

async function copyJson() {
  const json =
    activeTab.value === "flat" && flatComposition.value ? flatComposition.value : composition.value;
  if (json) {
    await navigator.clipboard.writeText(JSON.stringify(json, null, 2));
  }
}

const jsonDisplay = computed(() => {
  const data =
    activeTab.value === "flat" && flatComposition.value ? flatComposition.value : composition.value;
  return data ? JSON.stringify(data, null, 2) : "";
});

// Syntax highlighting
function highlightJson(json: string): string {
  return json
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"([^"]+)":/g, '<span class="json-key">"$1"</span>:')
    .replace(/: "([^"]*)"/g, ': <span class="json-string">"$1"</span>')
    .replace(/: (\d+)/g, ': <span class="json-number">$1</span>')
    .replace(/: (true|false)/g, ': <span class="json-boolean">$1</span>')
    .replace(/: (null)/g, ': <span class="json-null">$1</span>');
}

// Search highlighting in JSON/FLAT content
function highlightSearchInContent(html: string, searchQuery: string): string {
  if (!searchQuery) return html;

  // Escape HTML entities in search query to match the escaped content
  const escapedQuery = escapeHtml(searchQuery);
  const searchRegex = new RegExp(`(${escapeRegex(escapedQuery)})`, "gi");

  return html.replace(searchRegex, `<mark class="search-match" data-match>$1</mark>`);
}

const highlightedJson = computed(() => {
  let highlighted = highlightJson(jsonDisplay.value);
  if (panelSearchQuery.value && (activeTab.value === "json" || activeTab.value === "flat")) {
    highlighted = highlightSearchInContent(highlighted, panelSearchQuery.value);
  }
  return highlighted;
});

// Match counting for JSON/FLAT views
const jsonMatches = computed(() => {
  if (!panelSearchQuery.value || (activeTab.value !== "json" && activeTab.value !== "flat")) {
    return 0;
  }
  const content = jsonDisplay.value;
  const regex = new RegExp(escapeRegex(panelSearchQuery.value), "gi");
  const matches = content.match(regex);
  return matches ? matches.length : 0;
});

// Match navigation for JSON/FLAT views
function goToNextMatch() {
  if (jsonMatches.value === 0) return;
  currentMatchIndex.value = (currentMatchIndex.value + 1) % jsonMatches.value;
  scrollToMatch();
}

function goToPreviousMatch() {
  if (jsonMatches.value === 0) return;
  currentMatchIndex.value = (currentMatchIndex.value - 1 + jsonMatches.value) % jsonMatches.value;
  scrollToMatch();
}

function scrollToMatch() {
  nextTick(() => {
    const matches = document.querySelectorAll(".search-match");
    if (matches[currentMatchIndex.value]) {
      // Remove current-match class from all
      matches.forEach((el) => el.classList.remove("current-match"));

      // Add to current
      matches[currentMatchIndex.value].classList.add("current-match");
      matches[currentMatchIndex.value].scrollIntoView({
        behavior: "smooth",
        block: "center",
      });
    }
  });
}

// Trigger scroll when search query changes
watch(panelSearchQuery, () => {
  currentMatchIndex.value = 0;
  if (panelSearchQuery.value && (activeTab.value === "json" || activeTab.value === "flat")) {
    scrollToMatch();
  }
});

// Extract flat paths from the flat composition or web template
const flatPaths = computed(() => {
  if (flatComposition.value && typeof flatComposition.value === "object") {
    return Object.keys(flatComposition.value).sort();
  }
  return [];
});

function handleEdit() {
  router.push({
    name: "edit-composition",
    params: { ehrId: ehrId.value, compositionUid: compositionUid.value },
  });
}

async function handleDelete() {
  if (!serverStore.activeServerId) return;

  deleting.value = true;
  try {
    await compositionStore.deleteComposition(
      serverStore.activeServerId,
      ehrId.value,
      compositionUid.value,
    );
    showDeleteDialog.value = false;
    void analytics.track("composition_deleted");
    // Navigate back to EHR detail
    router.push({ name: "ehr-detail", params: { ehrId: ehrId.value } });
  } catch (e) {
    error.value = `Failed to delete composition: ${e}`;
  } finally {
    deleting.value = false;
  }
}

// Keyboard event listeners
onMounted(() => {
  window.addEventListener("keydown", handleKeydown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleKeydown);
});
</script>

<template>
  <div class="composition-viewer">
    <div class="viewer-header">
      <button class="btn btn-sm" @click="goBack">Back</button>
      <h2>Composition</h2>
      <div class="header-actions">
        <button
          type="button"
          class="tour-trigger-btn"
          title="Take a tour of the Composition Viewer"
          @click="replayTour"
        >
          <CompassIcon />
        </button>
        <div class="tab-bar" data-tour="composition-tabs">
          <button
            class="tab"
            :class="{ active: activeTab === 'pretty' }"
            @click="activeTab = 'pretty'"
          >
            Pretty
          </button>
          <button class="tab" :class="{ active: activeTab === 'json' }" @click="activeTab = 'json'">
            JSON
          </button>
          <button
            class="tab"
            :class="{ active: activeTab === 'flat' }"
            @click="activeTab = 'flat'"
            :disabled="!flatComposition"
          >
            FLAT
          </button>
        </div>
        <button
          type="button"
          class="btn btn-sm"
          data-tour="composition-paths"
          @click="showFlatPaths = !showFlatPaths"
        >
          {{ showFlatPaths ? "Hide" : "Show" }} Paths
        </button>
        <button type="button" class="btn btn-sm" data-tour="composition-copy" @click="copyJson">
          Copy JSON
        </button>
        <button class="btn btn-sm" @click="handleEdit">Edit</button>
        <button class="btn btn-sm btn-danger" @click="showDeleteDialog = true">Delete</button>
      </div>
    </div>

    <div v-if="loading" class="loading">Loading composition...</div>
    <div v-else-if="error" class="error-msg">{{ error }}</div>
    <div v-else-if="composition" class="viewer-content">
      <div class="main-content" :class="{ 'with-sidebar': showFlatPaths }">
        <!-- Pretty View -->
        <div v-if="activeTab === 'pretty'" class="pretty-view">
          <SearchOverlay
            v-if="showPanelSearch"
            ref="searchOverlayRef"
            v-model="panelSearchQuery"
            placeholder="Search composition tree..."
            @close="closePanelSearch"
            @next="goToNextMatch"
            @previous="goToPreviousMatch"
          />
          <CompositionTree
            :data="composition"
            :web-template="webTemplate"
            :highlighted-path="highlightedPath"
            :search-query="panelSearchQuery"
            :server-id="serverStore.activeServerId"
          />
        </div>

        <!-- JSON View -->
        <div v-if="activeTab === 'json'" class="json-view">
          <SearchOverlay
            v-if="showPanelSearch"
            ref="searchOverlayRef"
            v-model="panelSearchQuery"
            placeholder="Search JSON..."
            :match-count="currentMatchIndex"
            :total-matches="jsonMatches"
            @close="closePanelSearch"
            @next="goToNextMatch"
            @previous="goToPreviousMatch"
          />
          <pre class="json-pre"><code v-html="highlightedJson"></code></pre>
        </div>

        <!-- FLAT View -->
        <div v-if="activeTab === 'flat'" class="json-view">
          <SearchOverlay
            v-if="showPanelSearch"
            ref="searchOverlayRef"
            v-model="panelSearchQuery"
            placeholder="Search FLAT..."
            :match-count="currentMatchIndex"
            :total-matches="jsonMatches"
            @close="closePanelSearch"
            @next="goToNextMatch"
            @previous="goToPreviousMatch"
          />
          <pre class="json-pre"><code v-html="highlightedJson"></code></pre>
        </div>
      </div>

      <!-- FLAT Path Panel -->
      <FlatPathPanel
        v-if="showFlatPaths"
        :paths="flatPaths"
        @highlight="highlightedPath = $event"
      />
    </div>

    <!-- Delete Confirmation Dialog -->
    <div v-if="showDeleteDialog" class="dialog-overlay" @click="showDeleteDialog = false">
      <div class="dialog" @click.stop>
        <h3>Delete Composition</h3>
        <p>Are you sure you want to delete this composition? This action cannot be undone.</p>
        <div class="dialog-actions">
          <button class="btn btn-sm" @click="showDeleteDialog = false" :disabled="deleting">
            Cancel
          </button>
          <button class="btn btn-sm btn-danger" @click="handleDelete" :disabled="deleting">
            {{ deleting ? "Deleting..." : "Delete" }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.composition-viewer {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.viewer-header {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 12px 24px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg);
  position: sticky;
  top: 0;
  z-index: 1;
}
.viewer-header h2 {
  font-size: 16px;
  font-weight: 600;
}
.header-actions {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 8px;
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
.tab:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.viewer-content {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.main-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px 24px;
}
.main-content.with-sidebar {
  border-right: 1px solid var(--color-border);
}

.json-view {
  overflow: auto;
}
.json-pre {
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.6;
  color: var(--color-text);
  white-space: pre-wrap;
  word-break: break-word;
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
  margin: 0 0 24px 0;
  color: var(--color-text-secondary);
  line-height: 1.5;
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

/* JSON syntax highlighting */
:deep(.json-key) {
  color: #79c0ff;
  font-weight: 500;
}

:deep(.json-string) {
  color: #a5d6ff;
}

:deep(.json-number) {
  color: #79c0ff;
}

:deep(.json-boolean) {
  color: #ff7b72;
}

:deep(.json-null) {
  color: #8b949e;
}

/* Search highlighting */
:deep(.search-match) {
  background: rgba(255, 215, 0, 0.3);
  padding: 2px 0;
  border-radius: 2px;
}

:deep(.search-match.current-match) {
  background: rgba(255, 140, 0, 0.5);
  outline: 1px solid rgba(255, 140, 0, 0.8);
}
</style>
