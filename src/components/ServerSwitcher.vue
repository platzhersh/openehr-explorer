<script setup lang="ts">
import { computed } from "vue";
import { useServerStore } from "../stores/server";
import SearchableSelect, { type SearchableSelectOption } from "./SearchableSelect.vue";

const serverStore = useServerStore();

// Options for the server picker: one per profile, with a connected profile's
// name suffixed "[ok]" the same way the old native <select> showed it.
const serverOptions = computed<SearchableSelectOption[]>(() =>
  serverStore.profiles.map((profile) => ({
    value: profile.id,
    label: profile.name + (serverStore.connectionStatus[profile.id] === "connected" ? " [ok]" : ""),
  })),
);

// Distinct from the empty-selection placeholder: this is what shows *inside*
// the open panel when there's nothing left to pick, whether that's because
// no servers are configured at all or because the typed filter matched none.
const noOptionsText = computed(() =>
  serverStore.profiles.length === 0 ? "No servers configured" : "No matching servers",
);
</script>

<template>
  <div class="server-switcher" data-tour="server-select">
    <SearchableSelect
      class="server-select"
      label="Server"
      :options="serverOptions"
      :model-value="serverStore.activeServerId"
      :placeholder="serverStore.profiles.length === 0 ? 'No servers configured' : 'Select a server'"
      search-placeholder="Search servers..."
      :no-options-text="noOptionsText"
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

.server-select {
  width: 100%;
}

.server-select :deep(.searchable-select-label) {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--color-text-muted);
  margin-bottom: 6px;
  cursor: pointer;
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
