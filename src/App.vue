<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useServerStore } from "./stores/server";
import { useInspectorStore } from "./stores/inspector";
import { useSettingsStore } from "./stores/settings";
import { useUpdateStore } from "./stores/update";
import { useTourStore } from "./stores/tour";
import { useWhatsNewStore } from "./stores/whatsNew";
import { useAnalytics } from "./composables/useAnalytics";
import AppSidebar from "./components/AppSidebar.vue";
import ServerSwitcher from "./components/ServerSwitcher.vue";
import RequestInspector from "./components/RequestInspector.vue";
import UpdateNotification from "./components/UpdateNotification.vue";
import AnalyticsConsentDialog from "./components/AnalyticsConsentDialog.vue";
import FeatureTourOverlay from "./components/FeatureTourOverlay.vue";
import WhatsNewModal from "./components/WhatsNewModal.vue";
import { openUrl } from "@tauri-apps/plugin-opener";

// First-run consent dialog visibility. Driven by the persisted
// `analytics_consent_asked` flag: shown once, then the flag flips to true
// and the dialog never appears again.
const showAnalyticsConsent = ref(false);

// App version, fetched once on launch — feeds both the What's New gate and
// the modal's "dismiss up to this version" call.
const appVersion = ref<string | null>(null);

const route = useRoute();
const router = useRouter();
const serverStore = useServerStore();
const inspectorStore = useInspectorStore();
const settingsStore = useSettingsStore();
const updateStore = useUpdateStore();
const tourStore = useTourStore();
const whatsNewStore = useWhatsNewStore();
const analytics = useAnalytics();

// Unlisten handle for the native "Check for Updates…" menu item (see
// `install_update_check_menu_item` in the Rust backend), which emits this
// event instead of calling the updater directly — the updater plugin only
// has a JS API, so the Rust menu handler can't trigger it itself.
let unlistenMenuCheckForUpdates: UnlistenFn | null = null;

// Global keyboard shortcuts for navigation
function handleKeydown(e: KeyboardEvent) {
  if (e.metaKey || e.ctrlKey) {
    // Ctrl/Cmd + Shift + D: Open Documentation
    if (e.shiftKey && (e.key === "D" || e.key === "d")) {
      e.preventDefault();
      openUrl("https://platzhersh.github.io/openehr-explorer/docs.html");
      void analytics.track("documentation_opened");
    }
    // Ctrl/Cmd + 1: Switch to EHR Browser
    else if (e.key === "1") {
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

async function emitLaunchEvents() {
  // Two events fire at the start of every session:
  //
  //   1. `session_started { consent }`  — ALWAYS, regardless of consent
  //      toggle. This is the denominator used to compute opt-in rate; it
  //      carries nothing except the consent value itself. See ADR-0018.
  //   2. `app_launched  { version, os }` — ONLY if the user has opted in.
  //      Gives us the OS/version distribution for the opt-in population.
  //
  // Deferred until after any first-run consent-dialog decision so the
  // `consent` prop on `session_started` is never "pending".
  const consent = settingsStore.settings.analytics_enabled ? "yes" : "no";
  try {
    await analytics.trackUngated("session_started", { consent });
  } catch (e) {
    console.debug("[analytics] session_started failed:", e);
  }

  // Feature telemetry — consent-gated no-op for opted-out users.
  try {
    const version = await invoke<string>("get_app_version");
    analytics.track("app_launched", {
      version,
      os: navigator.platform || "unknown",
    });
  } catch (e) {
    console.debug("[analytics] app_launched failed:", e);
  }
}

async function handleConsentDecision(accepted: boolean) {
  // Persist BOTH flags in the same save so we atomically record "we asked"
  // and "here's the answer". If the user accepts, `analytics_enabled` is
  // true before the next `analytics.track()` call sees it.
  showAnalyticsConsent.value = false;
  await settingsStore.saveSettings({
    ...settingsStore.settings,
    analytics_enabled: accepted,
    analytics_consent_asked: true,
  });
  await emitLaunchEvents();
  checkWhatsNewAndTours();
}

// What's New (version-gated) and the route-aware feature tour both hold off
// until the consent dialog is resolved, so we never stack two modals — see
// PRD-0018. If What's New has something to show, the tour waits for it to
// be dismissed (below) rather than starting behind/alongside it.
function checkWhatsNewAndTours() {
  if (appVersion.value) {
    whatsNewStore.checkForUpdate(appVersion.value);
  }
  if (!whatsNewStore.visible) {
    tourStore.maybeAutoStart(route.name as string | undefined);
  }
}

onMounted(async () => {
  serverStore.loadProfiles();
  inspectorStore.startListening();
  // Settings must be loaded before the first analytics call so the consent
  // flag is accurate — otherwise the composable would gate on a stale default.
  await settingsStore.loadSettings();
  document.addEventListener("keydown", handleKeydown);

  try {
    appVersion.value = await invoke<string>("get_app_version");
  } catch (e) {
    console.error("Failed to get app version:", e);
  }

  unlistenMenuCheckForUpdates = await listen("check-for-updates-requested", () => {
    void analytics.track("update_check_triggered", { source: "menu" });
    void updateStore.checkForUpdates();
  });

  if (!settingsStore.settings.analytics_consent_asked) {
    // First run (or upgrade from a version without the flag): show the
    // one-time consent dialog. Hold off on emitting app_launched until the
    // user has made a choice so we don't silently track the session that
    // the user might be about to opt out of.
    showAnalyticsConsent.value = true;
  } else {
    await emitLaunchEvents();
    checkWhatsNewAndTours();
  }
});

onUnmounted(() => {
  document.removeEventListener("keydown", handleKeydown);
  unlistenMenuCheckForUpdates?.();
});

// Reset inspector when active server changes
watch(
  () => serverStore.activeServerId,
  () => {
    inspectorStore.reset();
  },
);

// Route-aware tour: offer the destination route's tour every time the user
// navigates somewhere new, unless it's already been seen/skipped or a
// first-run dialog is currently occupying the screen.
watch(
  () => route.name,
  async (name) => {
    if (!settingsStore.loaded || showAnalyticsConsent.value || whatsNewStore.visible) return;
    await nextTick();
    tourStore.maybeAutoStart(name as string | undefined);
  },
);

// Once the What's New modal is dismissed, offer the current route's tour —
// it was held back while the modal was up (see checkWhatsNewAndTours).
watch(
  () => whatsNewStore.visible,
  (visible) => {
    if (!visible) tourStore.maybeAutoStart(route.name as string | undefined);
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
      <UpdateNotification />
      <main class="app-main">
        <router-view />
      </main>
      <RequestInspector />
    </div>
    <AnalyticsConsentDialog
      v-if="showAnalyticsConsent"
      @accept="handleConsentDecision(true)"
      @decline="handleConsentDecision(false)"
    />
    <WhatsNewModal v-if="whatsNewStore.visible && appVersion" :current-version="appVersion" />
    <FeatureTourOverlay />
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

/* Manual "take a tour" trigger — used in view headers alongside the
   feature-tour system (see PRD-0018). A circular icon button so it reads
   as a secondary affordance next to primary header actions. */
.tour-trigger-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 50%;
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  color: var(--color-text-muted);
  font-size: 13px;
  line-height: 1;
  cursor: pointer;
  transition: all 0.15s;
  flex-shrink: 0;
}
.tour-trigger-btn:hover {
  color: var(--color-primary);
  border-color: var(--color-primary-dim);
  background: var(--color-surface-hover);
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
