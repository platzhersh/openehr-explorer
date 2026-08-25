<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted, nextTick } from "vue";
import { useRoute, useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { useServerStore } from "../stores/server";
import { useTemplateStore } from "../stores/template";
import { useAnalytics } from "../composables/useAnalytics";
import { extractFlatPaths, classifyCodedTextNode } from "../lib/webtemplate";
import { lookupCode } from "../lib/terminology";
import { open } from "@tauri-apps/plugin-dialog";
import { readTextFile } from "@tauri-apps/plugin-fs";
import OptMetadata from "../components/OptMetadata.vue";
import SearchOverlay from "../components/SearchOverlay.vue";
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

const searchQuery = ref("");
const panelSearchQuery = ref("");
const showPanelSearch = ref(false);
const activeTab = ref<"tree" | "json" | "opt" | "flat">("tree");
const currentMatchIndex = ref(0);
const searchOverlayRef = ref<InstanceType<typeof SearchOverlay> | null>(null);
const uploadDragOver = ref(false);
const showBoundConceptsHelp = ref(false);
const uploadStatus = ref<string | null>(null);
const uploadError = ref<string | null>(null);

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

const filteredTemplates = computed(() => {
  if (!searchQuery.value) return templateStore.templates;
  const q = searchQuery.value.toLowerCase();
  return templateStore.templates.filter((t) => t.template_id.toLowerCase().includes(q));
});

const webTemplateJson = computed(() => {
  return templateStore.selectedWebTemplate
    ? JSON.stringify(templateStore.selectedWebTemplate, null, 2)
    : "";
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
}

function buildWtTree(node: Record<string, unknown>): WtNode {
  const children = (node.children as Record<string, unknown>[]) ?? [];
  return {
    id: (node.id as string) ?? "",
    name: (node.name as string) ?? (node.localizedName as string) ?? "",
    rmType: (node.rmType as string) ?? "",
    aqlPath: (node.aqlPath as string) ?? "",
    children: children.map(buildWtTree),
    terminologyType: classifyCodedTextNode(node),
  };
}

const wtTree = computed(() => {
  if (!templateStore.selectedWebTemplate) return null;
  const tree = templateStore.selectedWebTemplate.tree as Record<string, unknown> | undefined;
  return tree ? buildWtTree(tree) : null;
});

async function uploadFile(file: File) {
  if (!serverStore.activeServerId) return;

  uploadStatus.value = null;
  uploadError.value = null;

  const text = await file.text();
  try {
    const result = await templateStore.uploadTemplate(serverStore.activeServerId, text);
    uploadStatus.value = result;
    void analytics.track("template_uploaded");
    templateStore.fetchTemplates(serverStore.activeServerId);
  } catch (e) {
    uploadError.value = String(e);
  }
}

// Syntax highlighting functions
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

// Pretty-print XML with indentation so long OPT documents wrap readably.
// Mirrors the formatter used in RequestInspector.vue.
function formatXml(xml: string): string {
  let formatted = "";
  let indent = 0;
  const lines = xml.split(/>\s*</);

  lines.forEach((line, index) => {
    if (index > 0) line = "<" + line;
    if (index < lines.length - 1) line = line + ">";

    if (line.match(/^<\/\w/)) indent--;

    formatted += "  ".repeat(Math.max(0, indent)) + line + "\n";

    if (line.match(/^<\w[^>]*[^/]>$/)) indent++;
  });

  return formatted.trim();
}

function highlightXml(xml: string): string {
  return xml
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(
      /(&lt;\/?)([\w-:]+)([\s\S]*?)(&gt;)/g,
      (_match, openBracket, tagName, content, closeBracket) => {
        const highlightedTag = `${openBracket}<span class="xml-tag">${tagName}</span>`;
        const highlightedContent = content.replace(
          /([\w-:]+)=(["'])(.*?)\2/g,
          '<span class="xml-attr-name">$1</span>=<span class="xml-attr-value">$2$3$2</span>',
        );
        return `${highlightedTag}${highlightedContent}${closeBracket}`;
      },
    )
    .replace(/(&lt;!--[\s\S]*?--&gt;)/g, '<span class="xml-comment">$1</span>')
    .replace(/(&lt;\?[\s\S]*?\?&gt;)/g, '<span class="xml-declaration">$1</span>');
}

// Unused - replaced by highlightedWebTemplateWithSearch and highlightedOptWithSearch
// const highlightedWebTemplate = computed(() => highlightJson(webTemplateJson.value));
// const highlightedOpt = computed(() =>
//   templateStore.selectedOpt ? highlightXml(templateStore.selectedOpt) : "",
// );

async function handleDrop(event: DragEvent) {
  event.preventDefault();
  uploadDragOver.value = false;
  const file = event.dataTransfer?.files[0];
  if (!file) return;

  await uploadFile(file);
}

async function handleFileSelect() {
  try {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: "OPT Files",
          extensions: ["opt", "xml"],
        },
      ],
    });

    if (selected && typeof selected === "string") {
      if (!serverStore.activeServerId) return;

      uploadStatus.value = null;
      uploadError.value = null;

      try {
        // Read file using Tauri's FS plugin
        const text = await readTextFile(selected);

        const result = await templateStore.uploadTemplate(serverStore.activeServerId, text);
        uploadStatus.value = result;
        void analytics.track("template_uploaded");
        templateStore.fetchTemplates(serverStore.activeServerId);
      } catch (e) {
        uploadError.value = String(e);
      }
    }
  } catch (e) {
    uploadError.value = String(e);
  }
}

async function copyToClipboard(text: string) {
  await navigator.clipboard.writeText(text);
}

function createComposition(templateId: string) {
  router.push({ name: "compose", params: { templateId } });
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

function escapeHtml(str: string): string {
  return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function highlightSearchInContent(html: string, searchQuery: string): string {
  if (!searchQuery) return html;

  // We need to search in the text content, not the HTML
  // Strategy: find matches in plain text positions, then inject marks in HTML
  const tempDiv = document.createElement("div");
  tempDiv.innerHTML = html;
  const plainText = tempDiv.textContent || "";

  const regex = new RegExp(escapeRegex(searchQuery), "gi");
  const matches: Array<{ start: number; end: number }> = [];
  let match;

  while ((match = regex.exec(plainText)) !== null) {
    matches.push({ start: match.index, end: match.index + match[0].length });
  }

  if (matches.length === 0) return html;

  // Rebuild HTML with <mark> tags
  // This is complex because we need to preserve HTML structure
  // Simplified approach: wrap matches in the final HTML string
  // This may not be perfect but works for most cases
  let result = html;
  const escapedQuery = escapeHtml(searchQuery);
  const searchRegex = new RegExp(`(${escapeRegex(escapedQuery)})`, "gi");

  result = result.replace(searchRegex, `<mark class="search-match" data-match>$1</mark>`);

  return result;
}

const highlightedWebTemplateWithSearch = computed(() => {
  let highlighted = highlightJson(webTemplateJson.value);
  if (panelSearchQuery.value) {
    highlighted = highlightSearchInContent(highlighted, panelSearchQuery.value);
  }
  return highlighted;
});

const formattedOptXml = computed(() => {
  return templateStore.selectedOpt ? formatXml(templateStore.selectedOpt) : "";
});

const highlightedOptWithSearch = computed(() => {
  if (!formattedOptXml.value) return "";
  let highlighted = highlightXml(formattedOptXml.value);
  if (panelSearchQuery.value) {
    highlighted = highlightSearchInContent(highlighted, panelSearchQuery.value);
  }
  return highlighted;
});

const jsonXmlMatches = computed(() => {
  if (!panelSearchQuery.value) return 0;
  const content = activeTab.value === "json" ? webTemplateJson.value : formattedOptXml.value;
  const regex = new RegExp(escapeRegex(panelSearchQuery.value), "gi");
  const matches = content.match(regex);
  return matches ? matches.length : 0;
});

function goToNextMatch() {
  if (jsonXmlMatches.value === 0) return;
  currentMatchIndex.value = (currentMatchIndex.value + 1) % jsonXmlMatches.value;
  scrollToMatch();
}

function goToPreviousMatch() {
  if (jsonXmlMatches.value === 0) return;
  currentMatchIndex.value =
    (currentMatchIndex.value - 1 + jsonXmlMatches.value) % jsonXmlMatches.value;
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

watch(panelSearchQuery, () => {
  currentMatchIndex.value = 0;
  if (panelSearchQuery.value && (activeTab.value === "json" || activeTab.value === "opt")) {
    scrollToMatch();
  }
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
        <button
          class="tour-trigger-btn"
          title="Take a tour of the Template Browser"
          @click="tourStore.start('templates')"
        >
          🧭
        </button>
      </div>

      <div class="search-bar">
        <input
          class="input search-input"
          data-tour="template-filter"
          v-model="searchQuery"
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

        <!-- Upload zone -->
        <div
          class="upload-zone"
          data-tour="template-upload"
          :class="{ 'drag-over': uploadDragOver }"
          @dragover.prevent="uploadDragOver = true"
          @dragleave="uploadDragOver = false"
          @drop="handleDrop"
        >
          <p>Drop OPT file here to upload</p>
          <button class="btn btn-sm" @click="handleFileSelect">Or choose file...</button>
        </div>
        <div v-if="uploadStatus" class="upload-msg success">{{ uploadStatus }}</div>
        <div v-if="uploadError" class="upload-msg error">{{ uploadError }}</div>
      </div>
    </div>

    <!-- Right: Template detail -->
    <div class="panel-right">
      <template v-if="selectedTemplateId && templateStore.selectedWebTemplate">
        <div class="panel-header">
          <h2>{{ selectedTemplateId }}</h2>
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
              :node="filteredWtTree"
              :depth="0"
              :search-query="panelSearchQuery"
              @copy="copyToClipboard"
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
            :total-matches="jsonXmlMatches"
            @close="closePanelSearch"
            @next="goToNextMatch"
            @previous="goToPreviousMatch"
          />
          <div class="json-actions">
            <button class="btn btn-sm" @click="copyToClipboard(webTemplateJson)">Copy JSON</button>
          </div>
          <pre class="json-pre"><code v-html="highlightedWebTemplateWithSearch"></code></pre>
        </div>

        <!-- OPT XML -->
        <div v-if="activeTab === 'opt'" class="xml-view">
          <SearchOverlay
            v-if="showPanelSearch"
            ref="searchOverlayRef"
            v-model="panelSearchQuery"
            placeholder="Search XML..."
            :match-count="currentMatchIndex"
            :total-matches="jsonXmlMatches"
            @close="closePanelSearch"
            @next="goToNextMatch"
            @previous="goToPreviousMatch"
          />
          <div class="xml-actions">
            <button class="btn btn-sm" @click="copyToClipboard(formattedOptXml)">Copy XML</button>
          </div>
          <pre class="xml-pre"><code v-html="highlightedOptWithSearch"></code></pre>
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
                <button class="copy-btn" @click="copyToClipboard(path)">Copy</button>
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
  </div>
</template>

<script lang="ts">
import { defineComponent, h, ref as vueRef, type PropType, type VNode } from "vue";

interface WtNodeType {
  id: string;
  name: string;
  rmType: string;
  aqlPath: string;
  children: WtNodeType[];
  terminologyType: string | null;
}

const WtTreeNode: ReturnType<typeof defineComponent> = defineComponent({
  name: "WtTreeNode",
  props: {
    node: { type: Object as PropType<WtNodeType>, required: true },
    depth: { type: Number, default: 0 },
  },
  emits: ["copy"],
  setup(props, { emit }): () => VNode {
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
        headerChildren.push(
          h(
            "button",
            {
              class: "copy-btn",
              onClick: () => emit("copy", node.aqlPath),
            },
            "Copy",
          ),
        );
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
              onCopy: (v: string) => emit("copy", v),
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
  emits: ["copy"],
  setup(props, { emit }): () => VNode {
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

      const elements = [];
      const headerChildren = [];

      if (hasChildren) {
        headerChildren.push(h("span", { class: "toggle-expanded" }, "\u25BC"));
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
        headerChildren.push(
          h(
            "button",
            {
              class: "copy-btn",
              onClick: () => emit("copy", node.aqlPath),
            },
            "Copy",
          ),
        );
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

      if (hasChildren) {
        elements.push(
          ...node.children.map((child) =>
            h(WtTreeNodeFiltered, {
              node: child,
              depth: props.depth + 1,
              searchQuery: props.searchQuery,
              onCopy: (v: string) => emit("copy", v),
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

.upload-zone {
  margin: 16px;
  padding: 24px;
  border: 2px dashed var(--color-border);
  border-radius: var(--radius);
  text-align: center;
  color: var(--color-text-muted);
  font-size: 13px;
  transition: all 0.15s;
  display: flex;
  flex-direction: column;
  gap: 12px;
  align-items: center;
}
.upload-zone.drag-over {
  border-color: var(--color-primary);
  background: rgba(100, 255, 218, 0.05);
}
.upload-zone p {
  margin: 0;
}

.upload-msg {
  margin: 0 16px;
  padding: 8px 12px;
  border-radius: var(--radius);
  font-size: 12px;
}
.upload-msg.success {
  color: var(--color-success);
}
.upload-msg.error {
  color: var(--color-error);
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
.json-actions {
  margin-bottom: 12px;
}
.json-pre {
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
}

/* JSON syntax highlighting */
.json-pre :deep(.json-key) {
  color: #6495ed;
  font-weight: 600;
}
.json-pre :deep(.json-string) {
  color: #6bff8e;
}
.json-pre :deep(.json-number) {
  color: #ffa500;
}
.json-pre :deep(.json-boolean) {
  color: #ff6b6b;
}
.json-pre :deep(.json-null) {
  color: var(--color-text-muted);
}

/* XML view */
.xml-view {
  padding-top: 16px;
}
.xml-actions {
  margin-bottom: 12px;
}
.xml-pre {
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
}

/* XML syntax highlighting */
.xml-pre :deep(.xml-tag) {
  color: #6495ed;
  font-weight: 600;
}
.xml-pre :deep(.xml-attr-name) {
  color: #ffd93d;
}
.xml-pre :deep(.xml-attr-value) {
  color: #6bff8e;
}
.xml-pre :deep(.xml-comment) {
  color: var(--color-text-muted);
  font-style: italic;
}
.xml-pre :deep(.xml-declaration) {
  color: #ff6b6b;
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

:deep(.toggle-expanded) {
  width: 16px;
  text-align: center;
  font-size: 10px;
  color: var(--color-text-muted);
  user-select: none;
}

.empty-search {
  text-align: center;
  padding: 48px 24px;
  color: var(--color-text-muted);
  font-size: 14px;
}
</style>
