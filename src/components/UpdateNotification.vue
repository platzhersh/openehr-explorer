<script setup lang="ts">
import { onMounted } from "vue";
import { useSettingsStore } from "../stores/settings";
import { useUpdateStore } from "../stores/update";

const settingsStore = useSettingsStore();
const updateStore = useUpdateStore();

onMounted(async () => {
  // Respect the user's preference. Settings are loaded by App.vue on mount,
  // but this component mounts alongside App, so ensure settings are loaded
  // before reading the flag.
  if (!settingsStore.loaded) {
    await settingsStore.loadSettings();
  }
  if (!settingsStore.settings.check_updates_on_startup) {
    return;
  }

  // checkForUpdates() never throws — failures (offline, rate-limited, GitHub
  // down, missing/placeholder pubkey in dev builds, etc.) land in
  // updateStore.error instead, and the app continues to work normally
  // without the updater. The startup banner just stays hidden in that case.
  await updateStore.checkForUpdates();
});

const progressPercent = () => {
  if (updateStore.totalBytes === 0) return 0;
  return Math.round((updateStore.downloadedBytes / updateStore.totalBytes) * 100);
};
</script>

<template>
  <div v-if="updateStore.update && !updateStore.dismissed" class="update-notification">
    <div class="update-info">
      <span class="update-icon">↻</span>
      <div class="update-text">
        <strong>Update available: v{{ updateStore.update.version }}</strong>
        <span class="update-current"> (current: v{{ updateStore.update.currentVersion }})</span>
      </div>
    </div>

    <div v-if="updateStore.error" class="update-error" :title="updateStore.error">
      Update failed: {{ updateStore.error }}
    </div>
    <div v-else-if="updateStore.downloading" class="update-progress">
      Downloading… {{ updateStore.totalBytes > 0 ? `${progressPercent()}%` : "" }}
    </div>
    <div v-else class="update-actions">
      <button type="button" class="btn btn-sm btn-primary" @click="updateStore.downloadAndInstall">
        Download &amp; Install
      </button>
      <button type="button" class="btn btn-sm" @click="updateStore.dismiss">Later</button>
    </div>
  </div>
</template>

<style scoped>
.update-notification {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 8px 16px;
  background: var(--color-primary-dim);
  border-bottom: 1px solid var(--color-primary);
  color: #fff;
  font-size: 13px;
}

.update-info {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.update-icon {
  font-size: 16px;
  flex-shrink: 0;
}

.update-text {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.update-current {
  opacity: 0.8;
}

.update-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.update-actions .btn {
  color: #fff;
  border-color: rgba(255, 255, 255, 0.3);
  background: rgba(255, 255, 255, 0.1);
}

.update-actions .btn:hover {
  background: rgba(255, 255, 255, 0.2);
}

.update-actions .btn-primary {
  background: #fff;
  color: var(--color-primary-dim);
  border-color: #fff;
}

.update-actions .btn-primary:hover {
  background: #f0f0f0;
  color: var(--color-bg);
}

.update-progress,
.update-error {
  font-size: 12px;
  opacity: 0.95;
  flex-shrink: 0;
}

.update-error {
  max-width: 320px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
