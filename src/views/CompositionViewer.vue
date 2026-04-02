<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { useServerStore } from "../stores/server";
import CompositionTree from "../components/CompositionTree.vue";
import FlatPathPanel from "../components/FlatPathPanel.vue";

const route = useRoute();
const router = useRouter();
const serverStore = useServerStore();

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

watch(
  [() => serverStore.activeServerId, compositionUid],
  async ([serverId, uid]) => {
    if (!serverId || !uid) return;
    loading.value = true;
    error.value = null;
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
  { immediate: true }
);

function goBack() {
  router.push({ name: "ehr-detail", params: { ehrId: ehrId.value } });
}

async function copyJson() {
  const json =
    activeTab.value === "flat" && flatComposition.value
      ? flatComposition.value
      : composition.value;
  if (json) {
    await navigator.clipboard.writeText(JSON.stringify(json, null, 2));
  }
}

const jsonDisplay = computed(() => {
  const data =
    activeTab.value === "flat" && flatComposition.value
      ? flatComposition.value
      : composition.value;
  return data ? JSON.stringify(data, null, 2) : "";
});

// Extract flat paths from the flat composition or web template
const flatPaths = computed(() => {
  if (flatComposition.value && typeof flatComposition.value === "object") {
    return Object.keys(flatComposition.value).sort();
  }
  return [];
});
</script>

<template>
  <div class="composition-viewer">
    <div class="viewer-header">
      <button class="btn btn-sm" @click="goBack">Back</button>
      <h2>Composition</h2>
      <div class="header-actions">
        <div class="tab-bar">
          <button
            class="tab"
            :class="{ active: activeTab === 'pretty' }"
            @click="activeTab = 'pretty'"
          >
            Pretty
          </button>
          <button
            class="tab"
            :class="{ active: activeTab === 'json' }"
            @click="activeTab = 'json'"
          >
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
        <button class="btn btn-sm" @click="showFlatPaths = !showFlatPaths">
          {{ showFlatPaths ? "Hide" : "Show" }} Paths
        </button>
        <button class="btn btn-sm" @click="copyJson">Copy JSON</button>
      </div>
    </div>

    <div v-if="loading" class="loading">Loading composition...</div>
    <div v-else-if="error" class="error-msg">{{ error }}</div>
    <div v-else-if="composition" class="viewer-content">
      <div class="main-content" :class="{ 'with-sidebar': showFlatPaths }">
        <!-- Pretty View -->
        <div v-if="activeTab === 'pretty'" class="pretty-view">
          <CompositionTree
            :data="composition"
            :web-template="webTemplate"
            :highlighted-path="highlightedPath"
          />
        </div>

        <!-- JSON View -->
        <div v-if="activeTab === 'json' || activeTab === 'flat'" class="json-view">
          <pre class="json-pre"><code>{{ jsonDisplay }}</code></pre>
        </div>
      </div>

      <!-- FLAT Path Panel -->
      <FlatPathPanel
        v-if="showFlatPaths"
        :paths="flatPaths"
        @highlight="highlightedPath = $event"
      />
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
</style>
