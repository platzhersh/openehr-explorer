<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useServerStore } from "../stores/server";
import { useTemplateStore } from "../stores/template";
import { extractFlatPaths } from "../lib/webtemplate";
import { open } from "@tauri-apps/plugin-dialog";
import { readTextFile } from "@tauri-apps/plugin-fs";
import OptMetadata from "../components/OptMetadata.vue";

const route = useRoute();
const router = useRouter();
const serverStore = useServerStore();
const templateStore = useTemplateStore();

const searchQuery = ref("");
const activeTab = ref<"tree" | "json" | "opt" | "flat">("tree");
const uploadDragOver = ref(false);
const uploadStatus = ref<string | null>(null);
const uploadError = ref<string | null>(null);

const selectedTemplateId = computed(() => route.params.templateId as string | undefined);

watch(
  () => serverStore.activeServerId,
  (id) => {
    if (id) templateStore.fetchTemplates(id);
  },
  { immediate: true },
);

watch(selectedTemplateId, (id) => {
  if (id && serverStore.activeServerId) {
    templateStore.fetchWebTemplate(serverStore.activeServerId, id);
    templateStore.fetchOpt(serverStore.activeServerId, id);
  }
});

function selectTemplate(id: string) {
  router.push({ name: "template-detail", params: { templateId: id } });
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
}

function buildWtTree(node: Record<string, unknown>): WtNode {
  const children = (node.children as Record<string, unknown>[]) ?? [];
  return {
    id: (node.id as string) ?? "",
    name: (node.name as string) ?? (node.localizedName as string) ?? "",
    rmType: (node.rmType as string) ?? "",
    aqlPath: (node.aqlPath as string) ?? "",
    children: children.map(buildWtTree),
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

const highlightedWebTemplate = computed(() => highlightJson(webTemplateJson.value));
const highlightedOpt = computed(() =>
  templateStore.selectedOpt ? highlightXml(templateStore.selectedOpt) : "",
);

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
</script>

<template>
  <div class="template-browser">
    <!-- Left: Template list -->
    <div class="panel-left">
      <div class="panel-header">
        <h2>Templates</h2>
      </div>

      <div class="search-bar">
        <input class="input search-input" v-model="searchQuery" placeholder="Filter templates..." />
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
            <div @click="selectTemplate(tmpl.template_id)" style="flex: 1; cursor: pointer">
              <div class="template-name">{{ tmpl.template_id }}</div>
              <div class="template-meta">
                <span v-if="tmpl.concept" class="meta-item">{{ tmpl.concept }}</span>
                <span v-if="tmpl.created_timestamp" class="meta-item">
                  {{ tmpl.created_timestamp }}
                </span>
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
            <div class="info-icon">ℹ️</div>
            <div class="info-content">
              <strong>About OPT Tree View:</strong> This view shows the human-readable structure
              derived from the Web Template. Cryptic archetype node IDs (e.g., <code>at0001</code>)
              from the raw OPT XML are automatically resolved to their meaningful labels.
              For the unresolved XML with archetype codes, see the <strong>OPT XML</strong> tab.
            </div>
          </div>

          <div v-if="wtTree" class="wt-tree">
            <WtTreeNode :node="wtTree" :depth="0" @copy="copyToClipboard" />
          </div>
        </div>

        <!-- Web Template JSON -->
        <div v-if="activeTab === 'json'" class="json-view">
          <div class="json-actions">
            <button class="btn btn-sm" @click="copyToClipboard(webTemplateJson)">Copy JSON</button>
          </div>
          <pre class="json-pre"><code v-html="highlightedWebTemplate"></code></pre>
        </div>

        <!-- OPT XML -->
        <div v-if="activeTab === 'opt'" class="xml-view">
          <div class="xml-actions">
            <button class="btn btn-sm" @click="copyToClipboard(templateStore.selectedOpt ?? '')">
              Copy XML
            </button>
          </div>
          <pre class="xml-pre"><code v-html="highlightedOpt"></code></pre>
        </div>

        <!-- FLAT Paths -->
        <div v-if="activeTab === 'flat'" class="flat-view">
          <div v-if="flatPaths.length > 0">
            <div class="flat-paths-header">
              <h3>FLAT Paths ({{ flatPaths.length }})</h3>
            </div>
            <div class="flat-paths-list">
              <div v-for="path in flatPaths" :key="path" class="flat-path-item">
                <span class="path-text">{{ path }}</span>
                <button class="copy-btn" @click="copyToClipboard(path)">Copy</button>
              </div>
            </div>
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
  align-items: center;
  gap: 12px;
}
.template-item:hover {
  background: var(--color-surface);
}
.template-item.active {
  background: var(--color-surface);
  border-left: 3px solid var(--color-primary);
}

.template-name {
  font-size: 13px;
  font-weight: 500;
  font-family: var(--font-mono);
}
.template-meta {
  display: flex;
  gap: 12px;
  margin-top: 4px;
}
.meta-item {
  font-size: 11px;
  color: var(--color-text-muted);
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
  font-size: 18px;
  flex-shrink: 0;
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
</style>
