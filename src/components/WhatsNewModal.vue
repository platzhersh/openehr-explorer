<script setup lang="ts">
/**
 * "What's New" modal — see PRD-0018. Shown once per app-version upgrade
 * (driven by `useWhatsNewStore`), or reopened manually from Settings.
 *
 * Styling mirrors `AnalyticsConsentDialog.vue` for visual consistency.
 */
import { useRouter } from "vue-router";
import { useWhatsNewStore } from "../stores/whatsNew";
import { useTourStore } from "../stores/tour";

const props = defineProps<{
  currentVersion: string;
}>();

const whatsNewStore = useWhatsNewStore();
const tourStore = useTourStore();
const router = useRouter();

function close() {
  void whatsNewStore.dismiss(props.currentVersion);
}

function takeTour(tourId: string, routePath?: string) {
  close();
  if (routePath) router.push(routePath);
  tourStore.start(tourId);
}
</script>

<template>
  <div class="dialog-overlay">
    <dialog open class="dialog" aria-modal="true" aria-labelledby="whats-new-title">
      <div class="dialog-header">
        <h2 id="whats-new-title">What's New</h2>
      </div>

      <div class="dialog-body">
        <div v-for="entry in whatsNewStore.entries" :key="entry.version" class="version-block">
          <div class="version-heading">
            <span class="version-badge">v{{ entry.version }}</span>
            <span class="version-date">{{ entry.date }}</span>
          </div>
          <div v-for="highlight in entry.highlights" :key="highlight.title" class="highlight">
            <div class="highlight-title">{{ highlight.title }}</div>
            <p class="highlight-desc">{{ highlight.description }}</p>
            <button
              v-if="highlight.tourId"
              type="button"
              class="btn btn-sm take-tour-btn"
              @click="takeTour(highlight.tourId, highlight.routePath)"
            >
              Take the tour →
            </button>
          </div>
        </div>
      </div>

      <div class="dialog-footer">
        <button type="button" class="btn btn-primary" @click="close">Got it</button>
      </div>
    </dialog>
  </div>
</template>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000; /* above FeatureTourOverlay */
}

.dialog {
  /* Reset the <dialog> element's UA defaults — position: absolute would
     otherwise pull it out of .dialog-overlay's flex-centering, and margin:
     auto/color/border defaults would fight the ones below. */
  position: static;
  margin: 0;
  color: inherit;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  width: 90%;
  max-width: 480px;
  max-height: 80vh;
  overflow: auto;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.6);
}

.dialog-header {
  padding: 20px 24px 0 24px;
}
.dialog-header h2 {
  font-size: 18px;
  font-weight: 600;
  margin: 0;
  color: var(--color-primary);
}

.dialog-body {
  padding: 16px 24px 8px 24px;
}

.version-block {
  margin-bottom: 20px;
}
.version-block:last-child {
  margin-bottom: 0;
}

.version-heading {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin-bottom: 10px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--color-border);
}
.version-badge {
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 700;
  color: var(--color-primary);
}
.version-date {
  font-size: 11px;
  color: var(--color-text-muted);
}

.highlight {
  margin-bottom: 14px;
}
.highlight:last-child {
  margin-bottom: 0;
}
.highlight-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text);
  margin-bottom: 4px;
}
.highlight-desc {
  font-size: 12px;
  color: var(--color-text-secondary);
  line-height: 1.5;
  margin-bottom: 6px;
}

.take-tour-btn {
  padding: 3px 10px;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 16px 24px 20px 24px;
  border-top: 1px solid var(--color-border);
  margin-top: 8px;
}
</style>
