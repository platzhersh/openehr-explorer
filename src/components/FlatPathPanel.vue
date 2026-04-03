<script setup lang="ts">
import { ref, computed } from "vue";

const props = defineProps<{
  paths: string[];
}>();

const emit = defineEmits<{
  highlight: [path: string | null];
}>();

const searchQuery = ref("");

const filteredPaths = computed(() => {
  if (!searchQuery.value) return props.paths;
  const q = searchQuery.value.toLowerCase();
  return props.paths.filter((p) => p.toLowerCase().includes(q));
});

async function copyPath(path: string) {
  await navigator.clipboard.writeText(path);
}
</script>

<template>
  <div class="flat-path-panel">
    <div class="panel-header">
      <h3>FLAT Paths</h3>
      <span class="path-count">{{ filteredPaths.length }}</span>
    </div>

    <div class="search-bar">
      <input class="input search-input" v-model="searchQuery" placeholder="Filter paths..." />
    </div>

    <div class="path-list">
      <div
        v-for="path in filteredPaths"
        :key="path"
        class="path-item"
        @mouseenter="emit('highlight', path)"
        @mouseleave="emit('highlight', null)"
      >
        <span class="path-text">{{ path }}</span>
        <button class="copy-btn" @click="copyPath(path)" title="Copy path">Copy</button>
      </div>

      <div v-if="filteredPaths.length === 0" class="empty-state">
        <p>No paths available.</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.flat-path-panel {
  width: 350px;
  min-width: 350px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--color-bg-secondary);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border);
}
.panel-header h3 {
  font-size: 13px;
  font-weight: 600;
}
.path-count {
  font-size: 11px;
  color: var(--color-text-muted);
}

.search-bar {
  padding: 8px;
}
.search-input {
  width: 100%;
  font-size: 12px;
}

.path-list {
  flex: 1;
  overflow-y: auto;
}

.path-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  border-bottom: 1px solid var(--color-border);
  transition: background 0.1s;
}
.path-item:hover {
  background: var(--color-surface);
}

.path-text {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--color-text-secondary);
  word-break: break-all;
  flex: 1;
}
</style>
