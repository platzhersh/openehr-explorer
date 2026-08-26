import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { useSettingsStore } from "./settings";
import { getTourById, getTourForRoute, type Tour } from "../lib/tours";

/**
 * Drives the route-aware feature tour overlay (`FeatureTourOverlay.vue`).
 * See PRD-0018.
 *
 * A tour is either auto-started once per route (via `maybeAutoStart`, gated
 * on the `tours_enabled` setting and the tour not already being in
 * `completed_tours`) or replayed on demand (via `start`, which ignores that
 * gate — a deliberate replay should always work).
 */
export const useTourStore = defineStore("tour", () => {
  const settingsStore = useSettingsStore();

  const activeTourId = ref<string | null>(null);
  const stepIndex = ref(0);

  const activeTour = computed<Tour | null>(() =>
    activeTourId.value ? (getTourById(activeTourId.value) ?? null) : null,
  );
  const activeStep = computed(() => activeTour.value?.steps[stepIndex.value] ?? null);
  const isFirstStep = computed(() => stepIndex.value === 0);
  const isLastStep = computed(
    () => !!activeTour.value && stepIndex.value === activeTour.value.steps.length - 1,
  );

  function isCompleted(tourId: string): boolean {
    return settingsStore.settings.completed_tours.includes(tourId);
  }

  /** Auto-start entry point — called on navigation. Respects user preferences. */
  function maybeAutoStart(routeName: string | null | undefined) {
    if (activeTourId.value) return; // never interrupt an in-progress tour
    if (!settingsStore.settings.tours_enabled) return;
    const tour = getTourForRoute(routeName);
    if (!tour || isCompleted(tour.id)) return;
    start(tour.id);
  }

  /** Explicit start — used by the manual "Take a tour" trigger and What's New links. Ignores completion state. */
  function start(tourId: string) {
    if (!getTourById(tourId)) return;
    activeTourId.value = tourId;
    stepIndex.value = 0;
  }

  function next() {
    if (!activeTour.value) return;
    if (isLastStep.value) {
      finish();
    } else {
      stepIndex.value += 1;
    }
  }

  function prev() {
    if (stepIndex.value > 0) stepIndex.value -= 1;
  }

  /** Advance past a step whose target element never appeared in the DOM. */
  function skipStep() {
    next();
  }

  async function finish() {
    try {
      if (activeTourId.value) await markCompleted(activeTourId.value);
    } finally {
      // Always close, even if persisting completion failed — otherwise a
      // save error (e.g. disk write failure) leaves the overlay stuck open
      // with its dismiss controls just retrying the same failing save.
      close();
    }
  }

  async function skipTour() {
    try {
      if (activeTourId.value) await markCompleted(activeTourId.value);
    } finally {
      close();
    }
  }

  function close() {
    activeTourId.value = null;
    stepIndex.value = 0;
  }

  async function markCompleted(tourId: string) {
    if (isCompleted(tourId)) return;
    await settingsStore.saveSettings({
      ...settingsStore.settings,
      completed_tours: [...settingsStore.settings.completed_tours, tourId],
    });
  }

  /** "Replay all tours" in Settings — clears completion state so every tour auto-starts again. */
  async function resetAllTours() {
    await settingsStore.saveSettings({
      ...settingsStore.settings,
      completed_tours: [],
    });
  }

  return {
    activeTourId,
    activeTour,
    activeStep,
    stepIndex,
    isFirstStep,
    isLastStep,
    isCompleted,
    maybeAutoStart,
    start,
    next,
    prev,
    skipStep,
    finish,
    skipTour,
    close,
    resetAllTours,
  };
});
