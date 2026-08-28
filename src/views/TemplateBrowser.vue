<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted, nextTick } from "vue";
import { useRoute, useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { useServerStore } from "../stores/server";
import { useTemplateStore } from "../stores/template";
import { useAnalytics } from "../composables/useAnalytics";
import { extractFlatPaths, classifyCodedTextNode } from "../lib/webtemplate";
import { lookupCode } from "../lib/terminology";
import OptMetadata from "../components/OptMetadata.vue";
import SearchOverlay from "../components/SearchOverlay.vue";
import CompassIcon from "../components/CompassIcon.vue";
import JsonViewer from "../components/JsonViewer.vue";
import CopyButton from "../components/CopyButton.vue";
import XmlViewer from "../components/XmlViewer.vue";
import TemplateUploadModal from "../components/TemplateUploadModal.vue";
import TemplateUploadZone from "../components/TemplateUploadZone.vue";
import { useTemplateUpload } from "../composables/useTemplateUpload";
import { useTourStore } from "../stores/tour";

interface TermBinding {
  terminology: string;
  code: string;
  node_id: string;
}

const route = useRoute();
const router = useRouter();
const serverStore = useServerStore();
const templateStore = useTemplateStore();
const analytics = useAnalytics();
const tourStore = useTourStore();

// Named distinctly from the `searchQuery` prop on the nested
// WtTreeNodeFiltered component below (and the same-named param on
// highlightSearchInContent) — sharing the identifier across scopes in one
// SFC was tripping static analysis into flagging this as a prop mutation.
const templateFilterQuery = ref("");
const panelSearchQuery = ref("");
const showPanelSearch = ref(false);
const activeTab = ref<"tree" | "json" | "opt" | "flat">("tree");
const currentMatchIndex = ref(0);
const searchOverlayRef = ref<InstanceType<typeof SearchOverlay> | null>(null);
const showBoundConceptsHelp = ref(false);
const showUploadModal = ref(false);

// Independent instance from the one inside TemplateUploadModal — this one
// backs the inline drop zone shown when the server has no templates yet
// (see the empty-state markup below), so the two never need to share state.
const inlineUpload = useTemplateUpload();

const selectedTemplateId = computed(() => route.params.templateId as string | undefined);
const termBindings = ref<TermBinding[]>([]);
const resolvedBindingTerms = ref<Record<string, string>>({});

watch(
  () => serverStore.activeServerId,
  (id) => {
    if (id) templateStore.fetchTemplates(id);
  },
  { immediate: true },
);

watch(selectedTemplateId, async (id) => {
  if (id && serverStore.activeServerId) {
    // Clear the previous template's data immediately rather than leaving it
    // on screen (and, via the OPT download button, downloadable) under the
    // new template's header while its own fetch is still in flight.
    templateStore.selectedWebTemplate = null;
    templateStore.selectedOpt = null;
    templateStore.fetchWebTemplate(serverStore.activeServerId, id);
    templateStore.fetchOpt(serverStore.activeServerId, id);
    // Fetch term bindings from OPT
    try {
      termBindings.value = await invoke<TermBinding[]>("get_term_bindings", {
        serverId: serverStore.activeServerId,
        templateId: id,
      });
      // Resolve display names for bound concepts
      resolvedBindingTerms.value = {};
      const servId = serverStore.activeServerId;
      for (const binding of termBindings.value) {
        lookupCode(servId, binding.terminology, binding.code).then((display) => {
          if (display) {
            resolvedBindingTerms.value = {
              ...resolvedBindingTerms.value,
              [`${binding.terminology}|${binding.code}`]: display,
            };
          }
        });
      }
    } catch {
      termBindings.value = [];
    }
  } else {
    termBindings.value = [];
    resolvedBindingTerms.value = {};
  }
});

function selectTemplate(id: string) {
  router.push({ name: "template-detail", params: { templateId: id } });
  // Feature-adoption ping only — the template_id is an archetype identifier
  // that could be customer-specific, so it's deliberately NOT included.
  void analytics.track("template_inspected");
}

function replayTour() {
  void analytics.track("tour_replayed", { tour_id: "templates" });
  tourStore.start("templates");
}

const filteredTemplates = computed(() => {
  if (!templateFilterQuery.value) return templateStore.templates;
  const q = templateFilterQuery.value.toLowerCase();
  return templateStore.templates.filter((t) => t.template_id.toLowerCase().includes(q));
});

const flatPaths = computed(() => {
  if (!templateStore.selectedWebTemplate) return [];
  return extractFlatPaths(templateStore.selectedWebTemplate);
});

// Web template tree nodes for display
interface WtNode {
  id: string;
  name: string;
  rmType: string;
  aqlPath: string;
  children: WtNode[];
  terminologyType: string | null;
  /**
   * Stable structural identity for this node — the id chain from the root,
   * with each segment's sibling index folded in so repeated archetype ids
   * (or empty ids) at different positions never collide. Search filtering
   * rebuilds the node objects on every keystroke, so this (not array
   * position) is what per-node UI state like collapse must key off of —
   * see `wtTreeCollapsedByPath` below.
   */
  path: string;
}

function buildWtTree(node: Record<string, unknown>, parentPath = ""): WtNode {
  const children = (node.children as Record<string, unknown>[]) ?? [];
  const id = (node.id as string) ?? "";
  const path = parentPath ? `${parentPath}/${id}` : id;
  return {
    id,
    name: (node.name as string) ?? (node.localizedName as string) ?? "",
    rmType: (node.rmType as string) ?? "",
    aqlPath: (node.aqlPath as string) ?? "",
    children: children.map((child, index) => buildWtTree(child, `${path}[${index}]`)),
    terminologyType: classifyCodedTextNode(node),
    path,
  };
}

const wtTree = computed(() => {
  if (!templateStore.selectedWebTemplate) return null;
  const tree = templateStore.selectedWebTemplate.tree as Record<string, unknown> | undefined;
  return tree ? buildWtTree(tree) : null;
});

function createComposition(templateId: string) {
  router.push({ name: "compose", params: { templateId } });
}

function downloadOpt() {
  const opt = templateStore.selectedOpt;
  const templateId = selectedTemplateId.value;
  if (!opt || !templateId) return;

  void analytics.track("template_opt_exported");

  const blob = new Blob([opt], { type: "application/xml" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `${templateId}.opt`;
  a.click();
  URL.revokeObjectURL(url);
}

// Search functionality
function handleKeydown(e: KeyboardEvent) {
  if (!selectedTemplateId.value) return;

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

watch(activeTab, () => {
  closePanelSearch();
});

watch(selectedTemplateId, () => {
  closePanelSearch();
});

// FLAT Paths filtering
const filteredFlatPaths = computed(() => {
  if (!panelSearchQuery.value) return flatPaths.value;
  const query = panelSearchQuery.value.toLowerCase();
  return flatPaths.value.filter((path) => path.toLowerCase().includes(query));
});

// FLAT Paths highlighting
function highlightPathMatch(path: string, query: string): string {
  if (!query) return path;

  const regex = new RegExp(`(${escapeRegex(query)})`, "gi");
  return path.replace(regex, '<mark class="path-search-match">$1</mark>');
}

// Tree filtering with ancestor preservation
interface FilteredNode extends WtNode {
  isMatch: boolean;
  isAncestor: boolean;
}

function filterTreeNode(node: WtNode, query: string): FilteredNode | null {
  if (!query) {
    return { ...node, isMatch: false, isAncestor: false };
  }

  const lowerQuery = query.toLowerCase();
  const matchesQuery =
    node.name.toLowerCase().includes(lowerQuery) ||
    node.id.toLowerCase().includes(lowerQuery) ||
    node.rmType.toLowerCase().includes(lowerQuery) ||
    node.aqlPath.toLowerCase().includes(lowerQuery);

  const filteredChildren = node.children
    .map((child) => filterTreeNode(child, query))
    .filter((child): child is FilteredNode => child !== null);

  if (matchesQuery || filteredChildren.length > 0) {
    return {
      ...node,
      children: filteredChildren,
      isMatch: matchesQuery,
      isAncestor: !matchesQuery && filteredChildren.length > 0,
    };
  }

  return null;
}

const filteredWtTree = computed(() => {
  if (!wtTree.value) return null;
  return filterTreeNode(wtTree.value, panelSearchQuery.value);
});

const treeMatchCount = computed(() => {
  if (!filteredWtTree.value) return 0;
  function countMatches(node: FilteredNode): number {
    let count = node.isMatch ? 1 : 0;
    for (const child of node.children) {
      count += countMatches(child as FilteredNode);
    }
    return count;
  }
  return countMatches(filteredWtTree.value);
});

// JSON/XML highlighting with search
function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

// Both match counts come from their respective viewer component (see
// @total-matches on the JsonViewer/XmlViewer instances below) — each viewer
// owns its own highlighting, current-match tracking, and scroll-into-view.
const jsonViewerMatches = ref(0);
const xmlViewerMatches = ref(0);
const activeTabMatches = computed(() =>
  activeTab.value === "json" ? jsonViewerMatches.value : xmlViewerMatches.value,
);

function goToNextMatch() {
  if (activeTabMatches.value === 0) return;
  currentMatchIndex.value = (currentMatchIndex.value + 1) % activeTabMatches.value;
}

function goToPreviousMatch() {
  if (activeTabMatches.value === 0) return;
  currentMatchIndex.value =
    (currentMatchIndex.value - 1 + activeTabMatches.value) % activeTabMatches.value;
}

watch(panelSearchQuery, () => {
  currentMatchIndex.value = 0;
});

onMounted(() => {
  window.addEventListener("keydown", handleKeydown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleKeydown);
});
</script>

<template>
  <div class="template-browser">
    <!-- Left: Template list -->
    <div class="panel-left">
      <div class="panel-header">
        <h2>Templates</h2>
        <div class="header-actions">
          <button
            type="button"
            class="tour-trigger-btn"
            title="Take a tour of the Template Browser"
            @click="replayTour"
          >
            <CompassIcon />
          </button>
          <button
            type="button"
            class="btn btn-sm btn-primary"
            data-tour="template-upload"
            @click="showUploadModal = true"
          >
            + Upload Template
          </button>
        </div>
      </div>

      <div class="search-bar">
        <label for="template-filter-input" class="sr-only">Filter templates</label>
        <input
          id="template-filter-input"
          class="input search-input"
          data-tour="template-filter"
          v-model="templateFilterQuery"
          placeholder="Filter templates..."
        />
      </div>

      <div v-if="templateStore.loading && !selectedTemplateId" class="loading">Loading...</div>
      <div v-else-if="templateStore.error" class="error-msg">
        {{ templateStore.error }}
      </div>
      <div v-else>
        <div class="template-list">
          <div
            v-for="tmpl in filteredTemplates"
            :key="tmpl.template_id"
            class="template-item"
            :class="{ active: tmpl.template_id === selectedTemplateId }"
          >
            <div @click="selectTemplate(tmpl.template_id)" class="template-content">
              <div class="template-id">{{ tmpl.template_id }}</div>
              <div v-if="tmpl.concept" class="template-concept">{{ tmpl.concept }}</div>
              <div v-if="tmpl.created_timestamp" class="template-date">
                {{ tmpl.created_timestamp }}
              </div>
            </div>
            <button
              class="btn btn-sm btn-primary"
              @click.stop="createComposition(tmpl.template_id)"
              title="Create new composition"
            >
              New Composition
            </button>
          </div>
        </div>

        <!-- Inline drop zone for the empty-server case — with no templates
             to scroll past, this stays visible without needing the header
             button, which is the only entry point once the list fills up. -->
        <div v-if="templateStore.templates.length === 0" class="empty-upload-state">
          <p class="empty-upload-hint">No templates on this server yet.</p>
          <TemplateUploadZone
            :drag-over="inlineUpload.dragOver.value"
            :uploading="inlineUpload.uploading.value"
            :upload-status="inlineUpload.uploadStatus.value"
            :upload-error="inlineUpload.uploadError.value"
            @dragover="inlineUpload.dragOver.value = true"
            @dragleave="inlineUpload.dragOver.value = false"
            @drop="inlineUpload.handleDrop"
            @choose-file="inlineUpload.handleFileSelect"
          />
        </div>
      </div>
    </div>

    <!-- Right: Template detail -->
    <div
      class="panel-right"
      :class="{ 'panel-right--bounded': activeTab === 'opt' || activeTab === 'json' }"
    >
      <template v-if="selectedTemplateId && templateStore.selectedWebTemplate">
        <div class="panel-header">
          <h2>{{ selectedTemplateId }}</h2>
          <div class="panel-header-actions">
            <div class="tab-bar">
              <button
                class="tab"
                :class="{ active: activeTab === 'tree' }"
                @click="activeTab = 'tree'"
              >
                OPT Tree
              </button>
              <button
                class="tab"
                :class="{ active: activeTab === 'opt' }"
                @click="activeTab = 'opt'"
                :disabled="!templateStore.selectedOpt"
              >
                OPT XML
              </button>
              <button
                class="tab"
                :class="{ active: activeTab === 'json' }"
                @click="activeTab = 'json'"
              >
                Web Template
              </button>
              <button
                class="tab"
                :class="{ active: activeTab === 'flat' }"
                @click="activeTab = 'flat'"
              >
                FLAT Paths
              </button>
            </div>
            <button
              type="button"
              class="btn btn-sm icon-btn"
              :disabled="!templateStore.selectedOpt"
              title="Download OPT"
              aria-label="Download OPT"
              @click="downloadOpt"
            >
              <svg
                width="14"
                height="14"
                viewBox="0 0 20 20"
                fill="none"
                xmlns="http://www.w3.org/2000/svg"
                aria-hidden="true"
              >
                <path
                  d="M10 3v9m0 0-3.5-3.5M10 12l3.5-3.5"
                  stroke="currentColor"
                  stroke-width="1.5"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
                <path
                  d="M4 14v1.5A1.5 1.5 0 0 0 5.5 17h9a1.5 1.5 0 0 0 1.5-1.5V14"
                  stroke="currentColor"
                  stroke-width="1.5"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
              </svg>
            </button>
          </div>
        </div>

        <!-- Tree view -->
        <div v-if="activeTab === 'tree'" class="tree-view">
          <OptMetadata v-if="templateStore.selectedOpt" :optXml="templateStore.selectedOpt" />

          <div class="info-banner">
            <svg
              class="info-icon"
              width="20"
              height="20"
              viewBox="0 0 20 20"
              fill="none"
              xmlns="http://www.w3.org/2000/svg"
            >
              <circle cx="10" cy="10" r="8" stroke="currentColor" stroke-width="1.5" />
              <path d="M10 10V14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
              <circle cx="10" cy="7" r="0.75" fill="currentColor" />
            </svg>
            <div class="info-content">
              <strong>About OPT Tree View:</strong> This view shows the human-readable structure
              derived from the Web Template. Cryptic archetype node IDs (e.g., <code>at0001</code>)
              from the raw OPT XML are automatically resolved to their meaningful labels. For the
              unresolved XML with archetype codes, see the <strong>OPT XML</strong> tab.
            </div>
          </div>

          <!-- Term Bindings -->
          <div v-if="termBindings.length > 0" class="term-bindings-section">
            <h4>
              Bound Concepts
              <button
                class="help-toggle"
                :class="{ active: showBoundConceptsHelp }"
                @click="showBoundConceptsHelp = !showBoundConceptsHelp"
                title="What are bound concepts?"
              >
                ?
              </button>
            </h4>
            <div v-if="showBoundConceptsHelp" class="bound-concepts-help">
              <strong>Term bindings</strong> link archetype nodes to external terminology codes.
              They declare the clinical meaning of each node in standard systems like SNOMED CT or
              LOINC. For example, a "blood pressure" node bound to <code>SNOMED-CT 163020007</code>
              tells integration systems exactly what concept this node represents, enabling
              interoperability across different EHR systems. Display names are resolved via the
              configured terminology server.
            </div>
            <div class="term-bindings-list">
              <div v-for="(binding, idx) in termBindings" :key="idx" class="term-binding-item">
                <span class="badge term-badge term-external">{{ binding.terminology }}</span>
                <span class="term-code">{{ binding.code }}</span>
                <span
                  v-if="resolvedBindingTerms[`${binding.terminology}|${binding.code}`]"
                  class="term-display"
                >
                  {{ resolvedBindingTerms[`${binding.terminology}|${binding.code}`] }}
                </span>
                <span v-if="binding.node_id" class="term-node-id">({{ binding.node_id }})</span>
              </div>
            </div>
          </div>

          <SearchOverlay
            v-if="showPanelSearch"
            ref="searchOverlayRef"
            v-model="panelSearchQuery"
            placeholder="Search tree..."
            :total-matches="treeMatchCount"
            @close="closePanelSearch"
          />

          <div v-if="filteredWtTree" class="wt-tree">
            <WtTreeNodeFiltered
              :key="filteredWtTree.path"
              :node="filteredWtTree"
              :depth="0"
              :search-query="panelSearchQuery"
            />
          </div>
          <div v-else-if="panelSearchQuery" class="empty-search">
            No nodes match '{{ panelSearchQuery }}'
          </div>
        </div>

        <!-- Web Template JSON -->
        <div v-if="activeTab === 'json'" class="json-view">
          <SearchOverlay
            v-if="showPanelSearch"
            ref="searchOverlayRef"
            v-model="panelSearchQuery"
            placeholder="Search JSON..."
            :match-count="currentMatchIndex"
            :total-matches="jsonViewerMatches"
            @close="closePanelSearch"
            @next="goToNextMatch"
            @previous="goToPreviousMatch"
          />
          <JsonViewer
            :value="templateStore.selectedWebTemplate"
            :search-term="panelSearchQuery"
            :current-match-index="currentMatchIndex"
            @total-matches="jsonViewerMatches = $event"
          />
        </div>

        <!-- OPT XML -->
        <div v-if="activeTab === 'opt'" class="xml-view">
          <SearchOverlay
            v-if="showPanelSearch"
            ref="searchOverlayRef"
            v-model="panelSearchQuery"
            placeholder="Search XML..."
            :match-count="currentMatchIndex"
            :total-matches="xmlViewerMatches"
            @close="closePanelSearch"
            @next="goToNextMatch"
            @previous="goToPreviousMatch"
          />
          <XmlViewer
            :xml="templateStore.selectedOpt ?? ''"
            :search-term="panelSearchQuery"
            :current-match-index="currentMatchIndex"
            @total-matches="xmlViewerMatches = $event"
          />
        </div>

        <!-- FLAT Paths -->
        <div v-if="activeTab === 'flat'" class="flat-view">
          <SearchOverlay
            v-if="showPanelSearch"
            ref="searchOverlayRef"
            v-model="panelSearchQuery"
            placeholder="Filter paths..."
            :total-matches="filteredFlatPaths.length"
            @close="closePanelSearch"
          />
          <div v-if="filteredFlatPaths.length > 0">
            <div class="flat-paths-header">
              <h3>
                FLAT Paths ({{ filteredFlatPaths.length
                }}{{ panelSearchQuery ? ` of ${flatPaths.length}` : "" }})
              </h3>
            </div>
            <div class="flat-paths-list">
              <div v-for="path in filteredFlatPaths" :key="path" class="flat-path-item">
                <span class="path-text" v-html="highlightPathMatch(path, panelSearchQuery)"></span>
                <CopyButton :text="path" title="Copy path" />
              </div>
            </div>
          </div>
          <div v-else-if="panelSearchQuery" class="empty-search">
            No paths match '{{ panelSearchQuery }}'
          </div>
        </div>
      </template>

      <div v-else class="empty-state">
        <h3>Select a template</h3>
        <p>Click on a template to view its structure and FLAT paths.</p>
      </div>
    </div>

    <TemplateUploadModal :open="showUploadModal" @close="showUploadModal = false" />
  </div>
</template>

<script lang="ts">
import { defineComponent, h, reactive, ref as vueRef, type PropType, type VNode } from "vue";

interface WtNodeType {
  id: string;
  name: string;
  rmType: string;
  aqlPath: string;
  children: WtNodeType[];
  terminologyType: string | null;
  path: string;
}

/**
 * Collapse state for the OPT tree, keyed by each node's structural `path`
 * rather than held as local per-instance state. `filterTreeNode` rebuilds
 * the filtered node objects (and their VNodes) on every search keystroke,
 * so instance-local state would get reused across — or dropped for —
 * whichever node happens to land in the same position, silently losing or
 * misapplying a manual expand/collapse once the search is cleared. Keying
 * by path instead makes a node's collapse state survive filtering intact.
 */
const wtTreeCollapsedByPath = reactive<Record<string, boolean>>({});

function isCollapsedByDefault(depth: number): boolean {
  return depth > 2;
}

const WtTreeNode: ReturnType<typeof defineComponent> = defineComponent({
  name: "WtTreeNode",
  props: {
    node: { type: Object as PropType<WtNodeType>, required: true },
    depth: { type: Number, default: 0 },
  },
  setup(props): () => VNode {
    const collapsed = vueRef(props.depth > 2);

    return (): VNode => {
      const node = props.node;
      const hasChildren = node.children.length > 0;

      const elements = [];

      const headerChildren = [];

      if (hasChildren) {
        headerChildren.push(
          h(
            "span",
            {
              class: "toggle",
              onClick: () => (collapsed.value = !collapsed.value),
            },
            collapsed.value ? "\u25B6" : "\u25BC",
          ),
        );
      } else {
        headerChildren.push(h("span", { class: "toggle-spacer" }));
      }

      headerChildren.push(h("span", { class: "wt-name" }, node.name || node.id));
      headerChildren.push(h("span", { class: "badge rm-type" }, node.rmType));

      // Terminology badge
      if (node.terminologyType) {
        if (node.terminologyType === "local") {
          headerChildren.push(
            h(
              "span",
              {
                class: "badge term-badge term-local",
                title: "Local codes — values are defined within the archetype (select list)",
              },
              "LOCAL",
            ),
          );
        } else {
          const label = node.terminologyType === "external" ? "EXTERNAL" : node.terminologyType;
          const tooltip =
            node.terminologyType === "external"
              ? "External terminology — values must be looked up from an external code system"
              : `External terminology — values from ${node.terminologyType} require lookup against a terminology server`;
          headerChildren.push(
            h("span", { class: "badge term-badge term-external", title: tooltip }, label),
          );
        }
      }

      if (node.aqlPath) {
        headerChildren.push(h("span", { class: "aql-path" }, node.aqlPath));
        headerChildren.push(h(CopyButton, { text: node.aqlPath, title: "Copy AQL path" }));
      }

      elements.push(
        h(
          "div",
          {
            class: "wt-node-header",
            style: { paddingLeft: `${props.depth * 20}px` },
          },
          headerChildren,
        ),
      );

      if (hasChildren && !collapsed.value) {
        elements.push(
          ...node.children.map((child) =>
            h(WtTreeNode, {
              node: child,
              depth: props.depth + 1,
            }),
          ),
        );
      }

      return h("div", { class: "wt-node" }, elements);
    };
  },
});

interface FilteredNodeType extends WtNodeType {
  isMatch: boolean;
  isAncestor: boolean;
}

const WtTreeNodeFiltered: ReturnType<typeof defineComponent> = defineComponent({
  name: "WtTreeNodeFiltered",
  props: {
    node: { type: Object as PropType<FilteredNodeType>, required: true },
    depth: { type: Number, default: 0 },
    searchQuery: { type: String, default: "" },
  },
  setup(props): () => VNode {
    const toggle = () => {
      const path = props.node.path;
      const current = wtTreeCollapsedByPath[path] ?? isCollapsedByDefault(props.depth);
      wtTreeCollapsedByPath[path] = !current;
    };

    function highlightMatch(text: string, query: string): VNode[] {
      if (!query) return [h("span", text)];

      const regex = new RegExp(`(${query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")})`, "gi");
      const parts = text.split(regex);

      return parts.map((part) =>
        regex.test(part) ? h("mark", { class: "tree-search-match" }, part) : h("span", part),
      );
    }

    return (): VNode => {
      const node = props.node;
      const hasChildren = node.children.length > 0;
      const collapsed = wtTreeCollapsedByPath[node.path] ?? isCollapsedByDefault(props.depth);

      const elements = [];
      const headerChildren = [];

      if (hasChildren) {
        headerChildren.push(
          h(
            "button",
            {
              type: "button",
              class: "toggle",
              "aria-expanded": !collapsed,
              "aria-label": `${collapsed ? "Expand" : "Collapse"} ${node.name || node.id}`,
              onClick: toggle,
            },
            collapsed ? "\u25B6" : "\u25BC",
          ),
        );
      } else {
        headerChildren.push(h("span", { class: "toggle-spacer" }));
      }

      headerChildren.push(
        h(
          "span",
          { class: ["wt-name", node.isAncestor && "ancestor"] },
          highlightMatch(node.name || node.id, props.searchQuery),
        ),
      );
      headerChildren.push(h("span", { class: "badge rm-type" }, node.rmType));

      // Terminology badge
      if (node.terminologyType) {
        if (node.terminologyType === "local") {
          headerChildren.push(
            h(
              "span",
              {
                class: "badge term-badge term-local",
                title: "Local codes — values are defined within the archetype (select list)",
              },
              "LOCAL",
            ),
          );
        } else {
          const label = node.terminologyType === "external" ? "EXTERNAL" : node.terminologyType;
          const tooltip =
            node.terminologyType === "external"
              ? "External terminology — values must be looked up from an external code system"
              : `External terminology — values from ${node.terminologyType} require lookup against a terminology server`;
          headerChildren.push(
            h("span", { class: "badge term-badge term-external", title: tooltip }, label),
          );
        }
      }

      if (node.aqlPath) {
        headerChildren.push(h("span", { class: "aql-path" }, node.aqlPath));
        headerChildren.push(h(CopyButton, { text: node.aqlPath, title: "Copy AQL path" }));
      }

      elements.push(
        h(
          "div",
          {
            class: ["wt-node-header", node.isMatch && "is-match", node.isAncestor && "is-ancestor"],
            style: { paddingLeft: `${props.depth * 20}px` },
          },
          headerChildren,
        ),
      );

      const shouldShowChildren = hasChildren && (!collapsed || props.searchQuery);
      if (shouldShowChildren) {
        elements.push(
          ...node.children.map((child) =>
            h(WtTreeNodeFiltered, {
              key: child.path,
              node: child,
              depth: props.depth + 1,
              searchQuery: props.searchQuery,
            }),
          ),
        );
      }

      return h("div", { class: "wt-node" }, elements);
    };
  },
});
</script>

<style scoped>
.template-browser {
  display: flex;
  height: 100%;
}

/* Visually hidden but still readable by screen readers — used to give the
   template filter input an accessible label without a visible one, since
   the input's placeholder already conveys the same text visually. */
.sr-only {
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

.panel-left {
  width: 350px;
  min-width: 350px;
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

/* The OPT XML and Web Template JSON tabs need a real bounded height to
   virtualize against (XmlViewer.vue/JsonViewer.vue fill whatever height
   they're given rather than guessing a viewport-relative max-height — see
   ADR-0023) — so while either is active, panel-right itself stops
   scrolling and instead becomes a flex column that hands the active tab's
   content div the remaining space below the header. Other tabs are
   untouched and keep scrolling the whole panel as before. */
.panel-right.panel-right--bounded {
  display: flex;
  flex-direction: column;
  overflow: hidden;
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

.search-bar {
  padding: 8px 16px;
}
.search-input {
  width: 100%;
}

.template-list {
  flex: 1;
}

.template-item {
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border);
  transition: background 0.15s;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.template-item:hover {
  background: var(--color-surface);
}
.template-item.active {
  background: var(--color-surface);
  border-left: 3px solid var(--color-primary);
}

.template-content {
  flex: 1;
  cursor: pointer;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.template-id {
  font-size: 13px;
  font-weight: 600;
  font-family: var(--font-mono);
  color: var(--color-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.template-concept {
  font-size: 12px;
  color: var(--color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.template-date {
  font-size: 11px;
  color: var(--color-text-muted);
  font-family: var(--font-mono);
}

.empty-upload-state {
  padding: 0 16px 16px;
}

.empty-upload-hint {
  margin: 16px 0 12px;
  font-size: 13px;
  color: var(--color-text-muted);
  text-align: center;
}

.panel-header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* Icon-only variant of .btn — same footprint as a tab so it sits flush
   next to the tab bar without widening the header. */
.icon-btn {
  padding: 4px;
  width: 26px;
  height: 26px;
  justify-content: center;
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

.tree-view {
  padding-top: 16px;
}

.info-banner {
  display: flex;
  gap: 12px;
  padding: 12px 16px;
  margin-bottom: 20px;
  background: rgba(100, 149, 237, 0.08);
  border: 1px solid rgba(100, 149, 237, 0.2);
  border-left: 3px solid rgba(100, 149, 237, 0.6);
  border-radius: var(--radius);
  font-size: 13px;
  line-height: 1.5;
}

.info-icon {
  width: 20px;
  height: 20px;
  flex-shrink: 0;
  color: rgba(100, 149, 237, 0.8);
}

.info-content {
  color: var(--color-text-secondary);
}

.info-content strong {
  color: var(--color-text);
  font-weight: 600;
}

.info-content code {
  font-family: var(--font-mono);
  font-size: 11px;
  padding: 2px 4px;
  background: rgba(0, 0, 0, 0.3);
  border-radius: 3px;
  color: rgba(100, 255, 218, 0.9);
}

:deep(.wt-node-header) {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  border-radius: 3px;
  font-size: 13px;
}
:deep(.wt-node-header:hover) {
  background: var(--color-surface);
}
:deep(.toggle) {
  width: 16px;
  text-align: center;
  cursor: pointer;
  font-size: 10px;
  color: var(--color-text-muted);
  user-select: none;
  background: none;
  border: none;
  padding: 0;
  font-family: inherit;
}
:deep(.toggle-spacer) {
  width: 16px;
}
:deep(.wt-name) {
  font-weight: 500;
}
:deep(.rm-type) {
  font-size: 10px;
}
:deep(.aql-path) {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--color-text-muted);
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Terminology badges */
:deep(.term-badge) {
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.5px;
  text-transform: uppercase;
}

:deep(.term-external) {
  background: rgba(255, 165, 0, 0.15);
  color: #ffa500;
  border-color: rgba(255, 165, 0, 0.3);
}

:deep(.term-local) {
  background: rgba(107, 255, 142, 0.1);
  color: var(--color-success);
  border-color: rgba(107, 255, 142, 0.2);
}

/* Term bindings section */
.term-bindings-section {
  margin-bottom: 20px;
  padding: 12px 16px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
}

.term-bindings-section h4 {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 8px;
  color: var(--color-text-secondary);
  display: flex;
  align-items: center;
  gap: 8px;
}

.help-toggle {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  border: 1px solid var(--color-text-muted);
  background: none;
  color: var(--color-text-muted);
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
  padding: 0;
}
.help-toggle:hover,
.help-toggle.active {
  border-color: var(--color-primary);
  color: var(--color-primary);
  background: rgba(100, 255, 218, 0.1);
}

.bound-concepts-help {
  margin-bottom: 12px;
  padding: 10px 14px;
  background: rgba(100, 149, 237, 0.08);
  border: 1px solid rgba(100, 149, 237, 0.2);
  border-left: 3px solid rgba(100, 149, 237, 0.6);
  border-radius: var(--radius);
  font-size: 12px;
  line-height: 1.5;
  color: var(--color-text-secondary);
}
.bound-concepts-help strong {
  color: var(--color-text);
}
.bound-concepts-help code {
  font-family: var(--font-mono);
  font-size: 11px;
  padding: 1px 4px;
  background: rgba(0, 0, 0, 0.3);
  border-radius: 3px;
  color: rgba(100, 255, 218, 0.9);
}

.term-bindings-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.term-binding-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}

.term-code {
  font-family: var(--font-mono);
  color: var(--color-primary);
  font-weight: 500;
}

.term-display {
  color: var(--color-success);
  font-size: 12px;
  font-weight: 500;
}

.term-node-id {
  color: var(--color-text-muted);
  font-family: var(--font-mono);
  font-size: 11px;
}

.flat-view {
  padding-top: 16px;
}

.flat-paths-header {
  margin-bottom: 12px;
}

.flat-paths-header h3 {
  font-size: 14px;
  color: var(--color-text-secondary);
}

.flat-paths-list {
  border-top: 1px solid var(--color-border);
}

.flat-path-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px;
  border-bottom: 1px solid var(--color-border);
}
.flat-path-item:hover {
  background: var(--color-surface);
}
.path-text {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--color-text-secondary);
  word-break: break-all;
  flex: 1;
}

.json-view {
  padding-top: 16px;
}

/* XML view */
.xml-view {
  padding-top: 16px;
}

/* Fills the remaining height panel-right--bounded hands it (see above), so
   XmlViewer's/JsonViewer's own flex:1 sizing has a real height to fill
   instead of shrinking to its content (which would just push the page
   taller). */
.panel-right--bounded .xml-view,
.panel-right--bounded .json-view {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* Search highlighting */
:deep(.search-match),
:deep(.path-search-match) {
  background: rgba(255, 255, 0, 0.3);
  border-radius: 2px;
  padding: 0 2px;
  font-weight: 600;
}

:deep(.search-match.current-match) {
  background: rgba(255, 165, 0, 0.5);
  outline: 1px solid rgba(255, 165, 0, 0.8);
}

:deep(.tree-search-match) {
  background: rgba(255, 255, 0, 0.3);
  border-radius: 2px;
  padding: 0 2px;
  font-weight: 700;
}

:deep(.wt-node-header.is-match) {
  background: rgba(255, 255, 0, 0.1);
}

:deep(.wt-node-header.is-ancestor) {
  opacity: 0.6;
  font-style: italic;
}

:deep(.wt-name.ancestor) {
  color: var(--color-text-muted);
}

.empty-search {
  text-align: center;
  padding: 48px 24px;
  color: var(--color-text-muted);
  font-size: 14px;
}
</style>
