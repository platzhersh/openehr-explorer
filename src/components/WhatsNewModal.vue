<script setup lang="ts">
/**
 * "What's New" modal — see PRD-0018. Shown once per app-version upgrade
 * (driven by `useWhatsNewStore`), or reopened manually from Settings.
 *
 * Styling mirrors `AnalyticsConsentDialog.vue` for visual consistency.
 */
import { ref } from "vue";
import { isNavigationFailure, NavigationFailureType, useRouter } from "vue-router";
import { useWhatsNewStore } from "../stores/whatsNew";
import { useTourStore } from "../stores/tour";
import { useAnalytics } from "../composables/useAnalytics";
import { useModalDialog } from "../composables/useModalDialog";

const props = defineProps<{
  currentVersion: string;
}>();

const whatsNewStore = useWhatsNewStore();
const tourStore = useTourStore();
const router = useRouter();
const analytics = useAnalytics();

// Native modal <dialog> (showModal): inert background, Tab trapping, and
// focus restoration come from the browser + useModalDialog.
const dialogRef = ref<HTMLDialogElement | null>(null);
useModalDialog(dialogRef);

// Single dismissal path: every way of closing (Escape → native cancel →
// close, the "Got it" button, taking a tour) ends in the dialog's `close`
// event, which is the only place the store is told.
function close() {
  dialogRef.value?.close();
}

function handleClosed() {
  void whatsNewStore.dismiss(props.currentVersion);
}

async function takeTour(tourId: string, routePath?: string) {
  void analytics.track("whats_new_tour_link_clicked", { tour_id: tourId });
  close();
  if (routePath) {
    // router.push() resolves (doesn't reject) with a NavigationFailure on
    // failure — only bail on a genuine abort/cancel. A "duplicated" result
    // (already on that route) is fine; the tour should still start.
    const failure = await router.push(routePath);
    if (
      isNavigationFailure(failure, NavigationFailureType.aborted) ||
      isNavigationFailure(failure, NavigationFailureType.cancelled)
    ) {
      return;
    }
  }
  tourStore.start(tourId);
}
</script>

<template>
  <dialog ref="dialogRef" class="dialog" aria-labelledby="whats-new-title" @close="handleClosed">
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
      <button type="button" class="btn btn-primary" autofocus @click="close">Got it</button>
    </div>
  </dialog>
</template>

<style scoped>
/* showModal() puts the dialog in the top layer, above FeatureTourOverlay and
   everything else, and centres it (UA margin: auto) — no wrapper or z-index. */
.dialog::backdrop {
  background: rgba(0, 0, 0, 0.7);
}

.dialog {
  /* The global reset zeroes margins, which would pin the top-layer dialog to
     the corner — auto margins are what centre a modal <dialog>. */
  margin: auto;
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
