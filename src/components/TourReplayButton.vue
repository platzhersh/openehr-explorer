<!--
  Manual "Take a tour" trigger button (see PRD-0018) — a compass icon that
  restarts a given tour immediately, ignoring its completion state.

  Every tour-enabled view wires up its own identical `replayTour` handler
  (track the `tour_replayed` analytics event, then `tourStore.start(id)`)
  behind an otherwise-identical button; this component exists to give new
  views (and any view whose trigger has no extra local state to manage,
  unlike e.g. `RequestInspector.vue`'s drawer-expand step) a single place to
  reuse that pattern instead of re-typing it.
-->
<script setup lang="ts">
import { useTourStore } from "../stores/tour";
import { useAnalytics } from "../composables/useAnalytics";
import CompassIcon from "./CompassIcon.vue";

const props = defineProps<{
  /** Tour id from `src/lib/tours.ts`, passed to `tourStore.start` and the analytics event. */
  tourId: string;
  /** Human-readable view name, e.g. "Terminology Browser" — used in the button's title as "Take a tour of the ___". */
  viewLabel: string;
}>();

const tourStore = useTourStore();
const analytics = useAnalytics();

function replayTour() {
  void analytics.track("tour_replayed", { tour_id: props.tourId });
  tourStore.start(props.tourId);
}
</script>

<template>
  <button
    type="button"
    class="tour-trigger-btn"
    :title="`Take a tour of the ${viewLabel}`"
    @click="replayTour"
  >
    <CompassIcon />
  </button>
</template>
