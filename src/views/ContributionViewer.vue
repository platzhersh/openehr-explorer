<script setup lang="ts">
import { computed, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useServerStore } from "../stores/server";
import { useContributionStore } from "../stores/contribution";
import { useAnalytics } from "../composables/useAnalytics";

const route = useRoute();
const router = useRouter();
const serverStore = useServerStore();
const contributionStore = useContributionStore();
const analytics = useAnalytics();

const ehrId = computed(() => route.params.ehrId as string);
const contributionUid = computed(() => route.params.contributionUid as string);

watch(
  [() => serverStore.activeServerId, ehrId, contributionUid],
  ([serverId, ehr, uid]) => {
    if (!serverId || !ehr || !uid) return;
    void load(serverId, ehr, uid);
  },
  { immediate: true },
);

async function load(serverId: string, ehr: string, uid: string) {
  await contributionStore.fetchContribution(serverId, ehr, uid);
  if (contributionStore.detail) {
    // Coarse feature-adoption ping only — no IDs. See useAnalytics guidelines.
    void analytics.track("contribution_viewed");
  }
}

function goBack() {
  router.push({ name: "ehr-detail", params: { ehrId: ehrId.value } });
}

async function copyToClipboard(text: string) {
  await navigator.clipboard.writeText(text);
}

function copyContributionUid() {
  const detail = contributionStore.detail;
  if (!detail) return;
  void copyToClipboard(detail.contribution_uid);
}

function openVersion(versionId: string) {
  router.push({
    name: "composition",
    params: { ehrId: ehrId.value, compositionUid: versionId },
  });
}
</script>

<template>
  <div class="contribution-viewer">
    <div class="viewer-header">
      <button type="button" class="btn btn-sm" @click="goBack">Back</button>
      <h2>Contribution</h2>
    </div>

    <div v-if="contributionStore.loading" class="loading">Loading contribution...</div>
    <div v-else-if="contributionStore.error" class="error-msg">
      Failed to load contribution: {{ contributionStore.error }}
    </div>
    <div v-else-if="contributionStore.detail" class="viewer-content">
      <div class="detail-row">
        <span class="detail-label">Contribution UID</span>
        <span class="detail-value mono">
          {{ contributionStore.detail.contribution_uid }}
          <button type="button" class="copy-btn" @click="copyContributionUid">Copy</button>
        </span>
      </div>

      <template v-if="contributionStore.detail.audit">
        <div class="detail-row" v-if="contributionStore.detail.audit.change_type">
          <span class="detail-label">Change Type</span>
          <span class="detail-value">
            <span class="badge">{{ contributionStore.detail.audit.change_type }}</span>
          </span>
        </div>
        <div class="detail-row" v-if="contributionStore.detail.audit.committer_name">
          <span class="detail-label">Committer</span>
          <span class="detail-value">{{ contributionStore.detail.audit.committer_name }}</span>
        </div>
        <div class="detail-row" v-if="contributionStore.detail.audit.time_committed">
          <span class="detail-label">Time Committed</span>
          <span class="detail-value">{{ contributionStore.detail.audit.time_committed }}</span>
        </div>
        <div class="detail-row" v-if="contributionStore.detail.audit.system_id">
          <span class="detail-label">System ID</span>
          <span class="detail-value mono">{{ contributionStore.detail.audit.system_id }}</span>
        </div>
        <div class="detail-row" v-if="contributionStore.detail.audit.description">
          <span class="detail-label">Description</span>
          <span class="detail-value">{{ contributionStore.detail.audit.description }}</span>
        </div>
      </template>
      <div v-else class="empty-audit">No audit details returned by the server.</div>

      <h3 class="section-title">Versions ({{ contributionStore.detail.versions.length }})</h3>

      <div v-if="contributionStore.detail.versions.length === 0" class="empty-state">
        <p>This contribution has no version references.</p>
      </div>
      <div v-else class="version-list">
        <div v-for="v in contributionStore.detail.versions" :key="v.id" class="version-item">
          <div class="version-main">
            <span v-if="v.version_type" class="badge">{{ v.version_type }}</span>
            <span class="version-id mono">{{ v.id }}</span>
          </div>
          <div class="version-actions">
            <button type="button" class="copy-btn" @click="copyToClipboard(v.id)">Copy</button>
            <button
              v-if="v.version_type === 'COMPOSITION'"
              type="button"
              class="btn btn-sm"
              @click="openVersion(v.id)"
            >
              Open
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.contribution-viewer {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
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

.viewer-content {
  padding: 16px 24px 24px;
}

.loading,
.error-msg {
  padding: 24px;
}
.error-msg {
  color: var(--color-error);
}

.detail-row {
  display: flex;
  padding: 8px 0;
  border-bottom: 1px solid var(--color-border);
}
.detail-label {
  width: 140px;
  flex-shrink: 0;
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

.copy-btn {
  margin-left: 8px;
  padding: 2px 8px;
  font-size: 11px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-surface);
  color: var(--color-text-secondary);
  cursor: pointer;
}
.copy-btn:hover {
  background: var(--color-border);
  color: var(--color-text);
}

.badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: var(--radius);
  background: var(--color-primary-dim);
  color: #fff;
  font-size: 11px;
  font-weight: 600;
}

.empty-audit {
  padding: 12px 0;
  font-size: 12px;
  color: var(--color-text-muted);
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  margin: 24px 0 12px;
  color: var(--color-text-secondary);
}

.empty-state {
  padding: 16px 0;
  color: var(--color-text-muted);
  font-size: 13px;
}

.version-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.version-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-surface);
}
.version-main {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}
.version-id {
  font-size: 12px;
  word-break: break-all;
}
.version-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}
</style>
