<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { useServerStore } from "../stores/server";
import { useDashboardStore } from "../stores/dashboard";
import { useAnalytics } from "../composables/useAnalytics";
import TourReplayButton from "../components/TourReplayButton.vue";
import RefreshButton from "../components/RefreshButton.vue";

const router = useRouter();
const serverStore = useServerStore();
const dashboardStore = useDashboardStore();
const analytics = useAnalytics();

// Timestamp of the last successful count fetch, shown next to the refresh
// button so "live" doesn't silently mean "however stale this happens to be".
const lastUpdated = ref<Date | null>(null);

const numberFormatter = new Intl.NumberFormat();

function formatCount(n: number | undefined): string {
  return n === undefined ? "—" : numberFormatter.format(n);
}

const statCards = computed(() => [
  { label: "EHRs", value: dashboardStore.counts?.ehr_count, to: "/ehrs" },
  { label: "Compositions", value: dashboardStore.counts?.composition_count, to: "/ehrs" },
  { label: "Templates", value: dashboardStore.counts?.template_count, to: "/templates" },
]);

// Not tracked — used for the automatic loads below (mount, server switch).
// Only the explicit refresh button counts as a `dashboard_refreshed` event.
async function loadAll(serverId: string) {
  await Promise.all([
    dashboardStore.fetchCounts(serverId),
    serverStore.fetchServerVersion(serverId),
  ]);
  if (!dashboardStore.error) lastUpdated.value = new Date();
}

async function refresh() {
  if (!serverStore.activeServerId) return;
  void analytics.track("dashboard_refreshed");
  await loadAll(serverStore.activeServerId);
}

onMounted(async () => {
  void analytics.track("dashboard_viewed");
  if (serverStore.profiles.length === 0) await serverStore.loadProfiles();
});

// Re-fetch whenever the active server changes — including the very first
// time one becomes available (e.g. profiles finish loading after mount).
// `immediate: true` covers the case where a server is already active by the
// time this view is set up.
watch(
  () => serverStore.activeServerId,
  async (serverId) => {
    dashboardStore.clear();
    lastUpdated.value = null;
    if (serverId) await loadAll(serverId);
  },
  { immediate: true },
);
</script>

<template>
  <div class="dashboard">
    <div class="view-header">
      <h2>Overview</h2>
      <div class="header-actions">
        <TourReplayButton tour-id="dashboard" view-label="Overview" />
        <span v-if="lastUpdated" class="last-updated">
          Updated {{ lastUpdated.toLocaleTimeString() }}
        </span>
        <RefreshButton
          data-tour="dashboard-refresh"
          variant="bordered"
          size="md"
          :disabled="!serverStore.activeServerId"
          :loading="dashboardStore.loading"
          @click="refresh"
        />
      </div>
    </div>

    <div v-if="!serverStore.activeServer" class="onboarding-card">
      <template v-if="serverStore.profiles.length === 0">
        <h3>Welcome to openEHR Explorer</h3>
        <p>
          Connect your first openEHR CDR — EHRBase, Better Platform, FerroEHR, or any generic
          openEHR REST server — to see live EHR, composition, and template counts here.
        </p>
        <button type="button" class="btn btn-primary" @click="router.push('/servers')">
          + Add Your First Server
        </button>
      </template>
      <template v-else>
        <h3>No server selected</h3>
        <p>Select a server profile to see its live EHR, composition, and template counts.</p>
        <button type="button" class="btn btn-primary" @click="router.push('/servers')">
          Go to Servers
        </button>
      </template>
    </div>

    <template v-else>
      <div v-if="dashboardStore.error" class="error-msg">
        Failed to load counts: {{ dashboardStore.error }}
      </div>

      <div class="stat-grid" data-tour="dashboard-stats">
        <router-link
          v-for="card in statCards"
          :key="card.label"
          :to="card.to"
          class="stat-card"
          :class="{ loading: dashboardStore.loading && card.value === undefined }"
        >
          <div class="stat-value">{{ formatCount(card.value) }}</div>
          <div class="stat-label">{{ card.label }}</div>
        </router-link>
      </div>

      <router-link to="/servers" class="server-info-card" data-tour="dashboard-server-info">
        <h3>Connected Server</h3>
        <dl class="server-info-grid">
          <dt>Name</dt>
          <dd>{{ serverStore.activeServer.name }}</dd>

          <dt>URL</dt>
          <dd class="mono">{{ serverStore.activeServer.base_url }}</dd>

          <dt>Type</dt>
          <dd>
            <span class="badge">{{ serverStore.activeServer.server_type }}</span>
          </dd>

          <dt>Status</dt>
          <dd>
            <span
              class="connection-indicator"
              :class="serverStore.connectionStatus[serverStore.activeServer.id] ?? 'unknown'"
            >
              <span class="dot"></span>
              {{ serverStore.connectionStatus[serverStore.activeServer.id] ?? "unknown" }}
            </span>
          </dd>

          <template v-if="serverStore.versionInfo[serverStore.activeServer.id]?.server_version">
            <dt>Version</dt>
            <dd>v{{ serverStore.versionInfo[serverStore.activeServer.id]?.server_version }}</dd>
          </template>
        </dl>
      </router-link>
    </template>
  </div>
</template>

<style scoped>
.dashboard {
  padding: 24px;
}

.view-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}
.view-header h2 {
  font-size: 20px;
  font-weight: 600;
}
.view-header .header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.last-updated {
  font-size: 12px;
  color: var(--color-text-muted);
  white-space: nowrap;
}

.onboarding-card {
  max-width: 480px;
  margin: 48px auto 0;
  padding: 40px 32px;
  text-align: center;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
}
.onboarding-card h3 {
  font-size: 18px;
  font-weight: 600;
  margin-bottom: 8px;
  color: var(--color-text);
}
.onboarding-card p {
  margin-bottom: 20px;
  color: var(--color-text-secondary);
  line-height: 1.6;
}

.stat-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
  gap: 16px;
  margin-bottom: 24px;
}

.stat-card {
  display: block;
  padding: 20px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  text-decoration: none;
  color: inherit;
  transition: all 0.15s;
}
.stat-card:hover {
  border-color: var(--color-primary-dim);
  background: var(--color-surface-hover);
}
.stat-card.loading {
  opacity: 0.6;
}

.stat-value {
  font-size: 32px;
  font-weight: 700;
  font-family: var(--font-mono);
  color: var(--color-primary);
  line-height: 1.2;
}

.stat-label {
  margin-top: 4px;
  font-size: 13px;
  color: var(--color-text-secondary);
}

.server-info-card {
  display: block;
  padding: 20px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  max-width: 560px;
  text-decoration: none;
  color: inherit;
  transition: all 0.15s;
}
.server-info-card:hover {
  border-color: var(--color-primary-dim);
  background: var(--color-surface-hover);
}
.server-info-card h3 {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 16px;
}

.server-info-grid {
  display: grid;
  grid-template-columns: auto 1fr;
  row-gap: 10px;
  column-gap: 16px;
  font-size: 13px;
}
.server-info-grid dt {
  color: var(--color-text-muted);
}
.server-info-grid dd {
  color: var(--color-text);
  overflow-wrap: anywhere;
}
.server-info-grid dd.mono {
  font-family: var(--font-mono);
  font-size: 12px;
}

.connection-indicator {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  text-transform: capitalize;
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-text-muted);
}
.connection-indicator.connected .dot {
  background: var(--color-success);
}
.connection-indicator.error .dot {
  background: var(--color-error);
}
</style>
