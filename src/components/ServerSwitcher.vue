<script setup lang="ts">
import { computed } from "vue";
import { useServerStore } from "../stores/server";
import SearchableSelect, { type SearchableSelectOption } from "./SearchableSelect.vue";

const serverStore = useServerStore();

const serverOptions = computed<SearchableSelectOption[]>(() =>
  serverStore.profiles.map((profile) => ({
    value: profile.id,
    label: profile.name + (serverStore.connectionStatus[profile.id] === "connected" ? " [ok]" : ""),
  })),
);
</script>

<template>
  <div class="server-switcher" data-tour="server-select">
    <label class="switcher-label">Server</label>
    <SearchableSelect
      class="server-select"
      :options="serverOptions"
      :model-value="serverStore.activeServerId"
      :placeholder="serverStore.profiles.length === 0 ? 'No servers configured' : 'Select a server'"
      search-placeholder="Search servers..."
      no-options-text="No servers configured"
      @update:model-value="(id) => id && serverStore.setActiveServer(id)"
    />
    <div
      v-if="serverStore.activeServer"
      class="connection-indicator"
      :class="serverStore.connectionStatus[serverStore.activeServer.id] ?? 'unknown'"
    >
      <span class="dot"></span>
      <span class="status-text">
        {{ serverStore.connectionStatus[serverStore.activeServer.id] ?? "unknown" }}
      </span>
    </div>
  </div>
</template>

<style scoped>
.server-switcher {
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border);
}

.switcher-label {
  display: block;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--color-text-muted);
  margin-bottom: 6px;
}

.server-select {
  width: 100%;
}

.server-select :deep(.searchable-select-control) {
  font-size: 12px;
}

.connection-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 6px;
  font-size: 11px;
  color: var(--color-text-muted);
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

.status-text {
  text-transform: capitalize;
}
</style>
