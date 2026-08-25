<script setup lang="ts">
/**
 * Route-aware feature tour overlay — see PRD-0018.
 *
 * Renders nothing when no tour is active (`useTourStore().activeTour`).
 * Otherwise dims the screen, "spotlights" the current step's target
 * element via a `box-shadow` cutout, and shows a tooltip card with
 * Back/Next/Skip controls next to it.
 *
 * Step targets are resolved live via `document.querySelector` rather than
 * refs, since the target can live in any currently-mounted view — this
 * component is mounted once, globally, in `App.vue`.
 */
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useTourStore } from "../stores/tour";

const tourStore = useTourStore();

const targetRect = ref<DOMRect | null>(null);
const highlightStyle = ref<Record<string, string>>({});
const tooltipStyle = ref<Record<string, string>>({});

let retryTimer: ReturnType<typeof setTimeout> | null = null;
let retriesLeft = 0;

const HIGHLIGHT_PADDING = 6;
const TOOLTIP_WIDTH = 320;
const TOOLTIP_MARGIN = 14;

function clearRetry() {
  if (retryTimer) {
    clearTimeout(retryTimer);
    retryTimer = null;
  }
}

function locateTarget() {
  clearRetry();
  const step = tourStore.activeStep;
  if (!step) {
    targetRect.value = null;
    return;
  }
  const el = document.querySelector<HTMLElement>(step.target);
  if (!el) {
    if (retriesLeft > 0) {
      retriesLeft -= 1;
      retryTimer = setTimeout(locateTarget, 150);
    } else {
      // Target never showed up (collapsed panel, unmet precondition, …) —
      // skip this step instead of stalling the tour indefinitely.
      tourStore.skipStep();
    }
    return;
  }
  el.scrollIntoView({ block: "center", behavior: "smooth" });
  requestAnimationFrame(() => updateRect(el));
}

function updateRect(el: HTMLElement) {
  targetRect.value = el.getBoundingClientRect();
  computeStyles();
}

function computeStyles() {
  const rect = targetRect.value;
  if (!rect) return;

  highlightStyle.value = {
    top: `${rect.top - HIGHLIGHT_PADDING}px`,
    left: `${rect.left - HIGHLIGHT_PADDING}px`,
    width: `${rect.width + HIGHLIGHT_PADDING * 2}px`,
    height: `${rect.height + HIGHLIGHT_PADDING * 2}px`,
  };

  const spaceBelow = window.innerHeight - rect.bottom;
  const placeBelow = spaceBelow > 180 || spaceBelow > rect.top;

  const style: Record<string, string> = {
    left: `${Math.max(TOOLTIP_MARGIN, Math.min(rect.left, window.innerWidth - TOOLTIP_WIDTH - TOOLTIP_MARGIN))}px`,
    width: `${TOOLTIP_WIDTH}px`,
  };
  if (placeBelow) {
    style.top = `${rect.bottom + TOOLTIP_MARGIN}px`;
  } else {
    style.bottom = `${window.innerHeight - rect.top + TOOLTIP_MARGIN}px`;
  }
  tooltipStyle.value = style;
}

function handleReposition() {
  const step = tourStore.activeStep;
  if (!step) return;
  const el = document.querySelector<HTMLElement>(step.target);
  if (el) updateRect(el);
}

function handleKeydown(e: KeyboardEvent) {
  if (!tourStore.activeTour) return;
  if (e.key === "Escape") {
    e.preventDefault();
    void tourStore.skipTour();
  } else if (e.key === "ArrowRight" || e.key === "Enter") {
    e.preventDefault();
    void tourStore.next();
  } else if (e.key === "ArrowLeft") {
    e.preventDefault();
    tourStore.prev();
  }
}

watch(
  () => tourStore.activeStep,
  async (step) => {
    if (!step) {
      clearRetry();
      targetRect.value = null;
      return;
    }
    await nextTick();
    retriesLeft = 10;
    locateTarget();
  },
  { immediate: true },
);

onMounted(() => {
  window.addEventListener("resize", handleReposition);
  window.addEventListener("scroll", handleReposition, true);
  window.addEventListener("keydown", handleKeydown);
});
onUnmounted(() => {
  window.removeEventListener("resize", handleReposition);
  window.removeEventListener("scroll", handleReposition, true);
  window.removeEventListener("keydown", handleKeydown);
  clearRetry();
});
</script>

<template>
  <div v-if="tourStore.activeTour && tourStore.activeStep && targetRect" class="tour-overlay">
    <div class="tour-backdrop"></div>
    <div class="tour-highlight" :style="highlightStyle"></div>
    <div
      class="tour-tooltip"
      :style="tooltipStyle"
      role="dialog"
      aria-modal="true"
      aria-label="Feature tour"
    >
      <div class="tour-tooltip-header">
        <span class="tour-step-count">
          Step {{ tourStore.stepIndex + 1 }} of {{ tourStore.activeTour.steps.length }}
        </span>
        <button class="tour-close" type="button" @click="tourStore.skipTour" title="Skip tour">
          &times;
        </button>
      </div>
      <h4 class="tour-title">{{ tourStore.activeStep.title }}</h4>
      <p class="tour-body">{{ tourStore.activeStep.body }}</p>
      <div class="tour-actions">
        <button class="tour-skip-link" type="button" @click="tourStore.skipTour">Skip tour</button>
        <div class="tour-nav">
          <button
            v-if="!tourStore.isFirstStep"
            class="btn btn-sm"
            type="button"
            @click="tourStore.prev"
          >
            Back
          </button>
          <button class="btn btn-sm btn-primary" type="button" @click="tourStore.next">
            {{ tourStore.isLastStep ? "Done" : "Next" }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tour-overlay {
  position: fixed;
  inset: 0;
  z-index: 1600; /* above app content, below AnalyticsConsentDialog/WhatsNewModal (2000) */
}

.tour-backdrop {
  position: absolute;
  inset: 0;
  pointer-events: auto;
}

.tour-highlight {
  position: fixed;
  border-radius: 8px;
  box-shadow: 0 0 0 9999px rgba(10, 14, 26, 0.72);
  border: 2px solid var(--color-primary);
  pointer-events: none;
  transition:
    top 0.2s ease,
    left 0.2s ease,
    width 0.2s ease,
    height 0.2s ease;
}

.tour-tooltip {
  position: fixed;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.55);
  padding: 14px 16px 16px 16px;
}

.tour-tooltip-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.tour-step-count {
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.3px;
  color: var(--color-text-muted);
  font-family: var(--font-mono);
}

.tour-close {
  background: none;
  border: none;
  color: var(--color-text-muted);
  font-size: 18px;
  line-height: 1;
  cursor: pointer;
  padding: 0 2px;
}
.tour-close:hover {
  color: var(--color-text);
}

.tour-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-primary);
  margin-bottom: 6px;
}

.tour-body {
  font-size: 13px;
  color: var(--color-text-secondary);
  line-height: 1.5;
  margin-bottom: 14px;
}

.tour-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.tour-skip-link {
  background: none;
  border: none;
  color: var(--color-text-muted);
  font-size: 12px;
  cursor: pointer;
  padding: 0;
}
.tour-skip-link:hover {
  color: var(--color-text-secondary);
  text-decoration: underline;
}

.tour-nav {
  display: flex;
  gap: 8px;
}
</style>
