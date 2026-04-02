<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useServerStore } from "../stores/server";
import { useEhrStore, type CompositionSummary } from "../stores/ehr";

const route = useRoute();
const router = useRouter();
const serverStore = useServerStore();
const ehrStore = useEhrStore();
const searchQuery = ref("");
const currentPage = ref(0);

const ehrId = computed(() => route.params.ehrId as string | undefined);

watch(
  () => serverStore.activeServerId,
  (id) => {
    if (id) {
      ehrStore.fetchEhrs(id, 0);
      currentPage.value = 0;
    }
  },
  { immediate: true }
);

watch(ehrId, (id) => {
  if (id && serverStore.activeServerId) {
    ehrStore.fetchEhrDetail(serverStore.activeServerId, id);
  }
});

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
    ehrStore.fetchEhrs(serverStore.activeServerId, currentPage.value);
  }
}

async function copyToClipboard(text: string) {
  await navigator.clipboard.writeText(text);
}

const filteredEhrs = computed(() => {
  if (!searchQuery.value) return ehrStore.ehrs;
  const q = searchQuery.value.toLowerCase();
  return ehrStore.ehrs.filter(
    (e) =>
      e.ehr_id.toLowerCase().includes(q) ||
      e.subject_id?.toLowerCase().includes(q)
  );
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
</script>

<template>
  <div class="ehr-browser">
    <div class="panel-left">
      <div class="panel-header">
        <h2>EHRs</h2>
        <button class="btn btn-sm" @click="refresh">Refresh</button>
      </div>

      <div class="search-bar">
        <input
          class="input search-input"
          v-model="searchQuery"
          placeholder="Search by EHR ID or subject..."
        />
      </div>

      <div v-if="ehrStore.loading" class="loading">Loading...</div>
      <div v-else-if="ehrStore.error" class="error-msg">{{ ehrStore.error }}</div>
      <div v-else-if="!serverStore.activeServerId" class="empty-state">
        <h3>No server selected</h3>
        <p>Configure a server in the Servers tab.</p>
      </div>
      <div v-else>
        <div class="ehr-list">
          <div
            v-for="ehr in filteredEhrs"
            :key="ehr.ehr_id"
            class="ehr-item"
            :class="{ active: ehr.ehr_id === ehrId }"
            @click="selectEhr(ehr.ehr_id)"
          >
            <div class="ehr-id">
              <span class="id-text">{{ ehr.ehr_id.substring(0, 8) }}...</span>
              <button class="copy-btn" @click.stop="copyToClipboard(ehr.ehr_id)" title="Copy full ID">
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
        </div>

        <div class="detail-section">
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
        </div>

        <h3 class="section-title">
          Compositions ({{ ehrStore.selectedEhr.compositions.length }})
        </h3>

        <div v-for="(comps, templateId) in compositionsByTemplate" :key="templateId" class="template-group">
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

      <div v-else class="empty-state">
        <h3>Select an EHR</h3>
        <p>Click on an EHR from the list to view its details and compositions.</p>
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

.search-bar {
  padding: 8px 16px;
}
.search-input {
  width: 100%;
}

.ehr-list {
  flex: 1;
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
</style>
