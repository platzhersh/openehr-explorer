<script setup lang="ts">
import { useRoute } from "vue-router";
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useAnalytics } from "../composables/useAnalytics";

const route = useRoute();
const analytics = useAnalytics();
const appVersion = ref<string>("");

const navItems = [
  { path: "/ehrs", label: "EHR Browser", icon: "H" },
  { path: "/templates", label: "Templates", icon: "T" },
  { path: "/aql", label: "AQL Runner", icon: "Q" },
  { path: "/servers", label: "Servers", icon: "S" },
];

function isActive(path: string): boolean {
  return route.path.startsWith(path);
}

async function openDocumentation() {
  await openUrl("https://platzhersh.github.io/openehr-explorer/docs.html");
  void analytics.track("documentation_opened");
}

onMounted(async () => {
  try {
    appVersion.value = await invoke<string>("get_app_version");
  } catch (error) {
    console.error("Failed to get app version:", error);
  }
});
</script>

<template>
  <nav class="sidebar-nav">
    <div class="nav-main" data-tour="nav-tabs">
      <router-link
        v-for="item in navItems"
        :key="item.path"
        :to="item.path"
        class="nav-item"
        :class="{ active: isActive(item.path) }"
      >
        <span class="nav-icon">{{ item.icon }}</span>
        <span class="nav-label">{{ item.label }}</span>
      </router-link>
    </div>
    <div class="nav-bottom">
      <div v-if="appVersion" class="version-display">v{{ appVersion }}</div>
      <div class="nav-divider"></div>
      <a
        @click="openDocumentation"
        class="nav-item external-link"
        role="button"
        tabindex="0"
        data-tour="nav-docs"
      >
        <span class="nav-icon doc-icon">
          <svg
            width="14"
            height="14"
            viewBox="0 0 20 20"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M6 2h8a2 2 0 012 2v12a2 2 0 01-2 2H6a2 2 0 01-2-2V4a2 2 0 012-2z"
              stroke="currentColor"
              stroke-width="1.5"
            />
            <path
              d="M7 7h6M7 11h6M7 15h4"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
            />
          </svg>
        </span>
        <span class="nav-label">Documentation</span>
        <svg class="external-icon" width="10" height="10" viewBox="0 0 12 12" fill="none">
          <path
            d="M3 1h8v8M11 1L1 11"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
          />
        </svg>
      </a>
      <router-link
        to="/settings"
        class="nav-item"
        :class="{ active: isActive('/settings') }"
        data-tour="nav-settings"
      >
        <span class="nav-icon gear-icon">
          <svg
            width="14"
            height="14"
            viewBox="0 0 20 20"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M10 12.5a2.5 2.5 0 100-5 2.5 2.5 0 000 5z"
              stroke="currentColor"
              stroke-width="1.5"
            />
            <path
              d="M16.2 12.2a1.3 1.3 0 00.26 1.43l.05.05a1.58 1.58 0 11-2.23 2.23l-.05-.05a1.3 1.3 0 00-1.43-.26 1.3 1.3 0 00-.79 1.19v.14a1.58 1.58 0 01-3.16 0v-.07a1.3 1.3 0 00-.85-1.19 1.3 1.3 0 00-1.43.26l-.05.05a1.58 1.58 0 11-2.23-2.23l.05-.05a1.3 1.3 0 00.26-1.43 1.3 1.3 0 00-1.19-.79h-.14a1.58 1.58 0 010-3.16h.07a1.3 1.3 0 001.19-.85 1.3 1.3 0 00-.26-1.43l-.05-.05a1.58 1.58 0 112.23-2.23l.05.05a1.3 1.3 0 001.43.26h.06a1.3 1.3 0 00.79-1.19v-.14a1.58 1.58 0 013.16 0v.07a1.3 1.3 0 00.79 1.19 1.3 1.3 0 001.43-.26l.05-.05a1.58 1.58 0 112.23 2.23l-.05.05a1.3 1.3 0 00-.26 1.43v.06a1.3 1.3 0 001.19.79h.14a1.58 1.58 0 010 3.16h-.07a1.3 1.3 0 00-1.19.79z"
              stroke="currentColor"
              stroke-width="1.5"
            />
          </svg>
        </span>
        <span class="nav-label">Settings</span>
      </router-link>
    </div>
  </nav>
</template>

<style scoped>
.sidebar-nav {
  display: flex;
  flex-direction: column;
  padding: 8px;
  gap: 2px;
  flex: 1;
}

.nav-main {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-bottom {
  margin-top: auto;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-radius: var(--radius);
  color: var(--color-text-secondary);
  text-decoration: none;
  font-size: 13px;
  transition: all 0.15s;
}
.nav-item:hover {
  background: var(--color-surface);
  color: var(--color-text);
}
.nav-item.active {
  background: var(--color-surface);
  color: var(--color-primary);
}

.nav-icon {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  background: var(--color-bg-tertiary);
  font-size: 12px;
  font-weight: 700;
  font-family: var(--font-mono);
}
.nav-item.active .nav-icon {
  background: var(--color-primary-dim);
  color: #fff;
}

.gear-icon,
.doc-icon {
  display: flex;
  align-items: center;
  justify-content: center;
}

.external-link {
  cursor: pointer;
  position: relative;
}

.external-icon {
  margin-left: auto;
  opacity: 0.5;
  transition: opacity 0.15s;
}

.external-link:hover .external-icon {
  opacity: 1;
}

.version-display {
  padding: 8px 12px;
  text-align: left;
  font-size: 11px;
  font-family: var(--font-mono);
  color: var(--color-text-muted);
  letter-spacing: 0.3px;
}

.nav-divider {
  height: 1px;
  background: var(--color-border);
  margin: 8px 0;
}
</style>
