<script setup lang="ts">
import { ref, onMounted } from "vue";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

const update = ref<Update | null>(null);
const dismissed = ref(false);
const downloading = ref(false);
const downloadedBytes = ref(0);
const totalBytes = ref(0);
const error = ref<string | null>(null);

onMounted(async () => {
  try {
    const result = await check();
    if (result?.available) {
      update.value = result;
    }
  } catch (err) {
    // Silently ignore update check failures (offline, rate-limited, GitHub down,
    // missing/placeholder pubkey in dev builds, etc.). The app must continue
    // to work normally without the updater.
    console.warn("Update check failed:", err);
  }
});

async function downloadAndInstall() {
  if (!update.value) return;
  downloading.value = true;
  error.value = null;
  downloadedBytes.value = 0;
  totalBytes.value = 0;

  try {
    await update.value.downloadAndInstall((event) => {
      if (event.event === "Started") {
        totalBytes.value = event.data.contentLength ?? 0;
      } else if (event.event === "Progress") {
        downloadedBytes.value += event.data.chunkLength;
      }
    });
    // On macOS/Linux the current binary has been replaced; relaunch to pick
    // up the new version. On Windows the NSIS installer takes over.
    await relaunch();
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
    downloading.value = false;
  }
}

function dismiss() {
  dismissed.value = true;
}

const progressPercent = () => {
  if (totalBytes.value === 0) return 0;
  return Math.round((downloadedBytes.value / totalBytes.value) * 100);
};
</script>

<template>
  <div v-if="update && !dismissed" class="update-notification">
    <div class="update-info">
      <span class="update-icon">↻</span>
      <div class="update-text">
        <strong>Update available: v{{ update.version }}</strong>
        <span class="update-current"> (current: v{{ update.currentVersion }})</span>
      </div>
    </div>

    <div v-if="error" class="update-error" :title="error">Update failed: {{ error }}</div>
    <div v-else-if="downloading" class="update-progress">
      Downloading… {{ totalBytes > 0 ? `${progressPercent()}%` : "" }}
    </div>
    <div v-else class="update-actions">
      <button class="btn btn-sm btn-primary" @click="downloadAndInstall">
        Download &amp; Install
      </button>
      <button class="btn btn-sm" @click="dismiss">Later</button>
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
