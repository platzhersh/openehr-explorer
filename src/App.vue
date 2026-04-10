<script setup lang="ts">
import { onMounted, onUnmounted, watch } from "vue";
import { useRouter } from "vue-router";
import { useServerStore } from "./stores/server";
import { useInspectorStore } from "./stores/inspector";
import { useSettingsStore } from "./stores/settings";
import AppSidebar from "./components/AppSidebar.vue";
import ServerSwitcher from "./components/ServerSwitcher.vue";
import RequestInspector from "./components/RequestInspector.vue";

const router = useRouter();
const serverStore = useServerStore();
const inspectorStore = useInspectorStore();
const settingsStore = useSettingsStore();

// Global keyboard shortcuts for navigation
function handleKeydown(e: KeyboardEvent) {
  if (e.metaKey || e.ctrlKey) {
    // Ctrl/Cmd + 1: Switch to EHR Browser
    if (e.key === "1") {
      e.preventDefault();
      router.push("/ehrs");
    }
    // Ctrl/Cmd + 2: Switch to Template Browser
    else if (e.key === "2") {
      e.preventDefault();
      router.push("/templates");
    }
    // Ctrl/Cmd + 3: Switch to AQL Runner
    else if (e.key === "3") {
      e.preventDefault();
      router.push("/aql");
    }
    // Ctrl/Cmd + 4: Switch to Server Manager
    else if (e.key === "4") {
      e.preventDefault();
      router.push("/servers");
    }
    // Ctrl/Cmd + ,: Open Settings
    else if (e.key === ",") {
      e.preventDefault();
      router.push("/settings");
    }
  }
}

onMounted(() => {
  serverStore.loadProfiles();
  inspectorStore.startListening();
  settingsStore.loadSettings();
  document.addEventListener("keydown", handleKeydown);
});

onUnmounted(() => {
  document.removeEventListener("keydown", handleKeydown);
});

// Reset inspector when active server changes
watch(
  () => serverStore.activeServerId,
  () => {
    inspectorStore.reset();
  },
);
</script>

<template>
  <div class="app-layout">
    <aside class="app-sidebar">
      <div class="app-logo">
        <h1>openEHR Explorer</h1>
      </div>
      <ServerSwitcher />
      <AppSidebar />
    </aside>
    <div class="app-content-area">
      <main class="app-main">
        <router-view />
      </main>
      <RequestInspector />
    </div>
  </div>
</template>

<style>
*,
*::before,
*::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

:root {
  --sidebar-width: 240px;
  --color-bg: #1a1a2e;
  --color-bg-secondary: #16213e;
  --color-bg-tertiary: #0f3460;
  --color-surface: #1e2a4a;
  --color-surface-hover: #253456;
  --color-border: #2a3a5c;
  --color-text: #e0e0e0;
  --color-text-secondary: #8892b0;
  --color-text-muted: #5a6a8a;
  --color-primary: #64ffda;
  --color-primary-dim: #3d9e85;
  --color-error: #ff6b6b;
  --color-warning: #ffd93d;
  --color-success: #6bff8e;
  --font-mono: "JetBrains Mono", "Fira Code", "Cascadia Code", monospace;
  --font-sans: "Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  --radius: 6px;
}

body {
  font-family: var(--font-sans);
  font-size: 14px;
  color: var(--color-text);
  background: var(--color-bg);
  line-height: 1.5;
  overflow: hidden;
  height: 100vh;
}

#app {
  height: 100vh;
}

.app-layout {
  display: flex;
  height: 100vh;
}

.app-sidebar {
  width: var(--sidebar-width);
  min-width: var(--sidebar-width);
  background: var(--color-bg-secondary);
  border-right: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}

.app-logo {
  padding: 16px;
  border-bottom: 1px solid var(--color-border);
}

.app-logo h1 {
  font-size: 16px;
  font-weight: 600;
  color: var(--color-primary);
  letter-spacing: -0.3px;
}

.app-content-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.app-main {
  flex: 1;
  overflow: auto;
  background: var(--color-bg);
}

/* Shared utility styles */
.btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-surface);
  color: var(--color-text);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s;
}
.btn:hover {
  background: var(--color-surface-hover);
  border-color: var(--color-primary-dim);
}
.btn-primary {
  background: var(--color-primary-dim);
  border-color: var(--color-primary);
  color: #fff;
}
.btn-primary:hover {
  background: var(--color-primary);
  color: var(--color-bg);
}
.btn-sm {
  padding: 3px 8px;
  font-size: 12px;
}
.btn-danger {
  border-color: var(--color-error);
  color: var(--color-error);
}
.btn-danger:hover {
  background: var(--color-error);
  color: #fff;
}

.input {
  padding: 6px 10px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-surface);
  color: var(--color-text);
  font-size: 13px;
  outline: none;
  transition: border-color 0.15s;
}
.input:focus {
  border-color: var(--color-primary-dim);
}
.input::placeholder {
  color: var(--color-text-muted);
}

select.input {
  cursor: pointer;
}

.badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  border-radius: 10px;
  font-size: 11px;
  font-weight: 500;
  background: var(--color-surface);
  color: var(--color-text-secondary);
  border: 1px solid var(--color-border);
}
.badge-primary {
  background: var(--color-primary-dim);
  color: #fff;
  border-color: var(--color-primary);
}

.copy-btn {
  background: none;
  border: none;
  color: var(--color-text-muted);
  cursor: pointer;
  padding: 2px 4px;
  border-radius: 3px;
  font-size: 12px;
}
.copy-btn:hover {
  color: var(--color-primary);
  background: var(--color-surface);
}

.error-msg {
  padding: 12px;
  background: rgba(255, 107, 107, 0.1);
  border: 1px solid var(--color-error);
  border-radius: var(--radius);
  color: var(--color-error);
  font-size: 13px;
}

.loading {
  padding: 24px;
  text-align: center;
  color: var(--color-text-muted);
}

.empty-state {
  padding: 48px 24px;
  text-align: center;
  color: var(--color-text-muted);
}
.empty-state h3 {
  margin-bottom: 8px;
  color: var(--color-text-secondary);
}
</style>
