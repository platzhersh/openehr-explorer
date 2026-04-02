<script setup lang="ts">
import { useServerStore } from "../stores/server";

const serverStore = useServerStore();
</script>

<template>
  <div class="server-switcher">
    <label class="switcher-label">Server</label>
    <select
      class="input server-select"
      :value="serverStore.activeServerId"
      @change="serverStore.setActiveServer(($event.target as HTMLSelectElement).value)"
    >
      <option v-if="serverStore.profiles.length === 0" value="" disabled>
        No servers configured
      </option>
      <option
        v-for="profile in serverStore.profiles"
        :key="profile.id"
        :value="profile.id"
      >
        {{ profile.name }}
        <template v-if="serverStore.connectionStatus[profile.id] === 'connected'">
          [ok]
        </template>
      </option>
    </select>
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
