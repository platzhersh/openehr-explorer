<script setup lang="ts">
// Downloads-over-time sparkline: fetches the daily snapshot history
// recorded by .github/workflows/downloads-snapshot.yml. That workflow
// commits to the dedicated `data` branch (never `main`), so this reads
// it cross-origin straight off raw.githubusercontent.com — no backend
// needed, the site stays fully static. See OEH-25. Ported from the
// vanilla-JS IIFE that used to live at the bottom of index.html.
import { onMounted, ref } from "vue";

const HISTORY_URL = "https://raw.githubusercontent.com/platzhersh/openehr-explorer/data/docs/downloads-history.json";
// A couple of days of noisy, near-flat snapshots isn't a curve worth
// showing — wait for a full week before the sparkline appears.
const MIN_POINTS = 7;
const WIDTH = 120;
const HEIGHT = 28;

interface Point {
  date: string;
  total: number;
}

const visible = ref(false);
const linePath = ref("");
const areaPath = ref("");
const description = ref("");
const caption = ref("");

function buildPaths(points: Point[]) {
  const totals = points.map((p) => p.total);
  const min = Math.min(...totals);
  const max = Math.max(...totals);
  const range = max - min || 1; // avoid a divide-by-zero on a flat line

  const coords = points.map((p, i) => {
    const x = (i / (points.length - 1)) * WIDTH;
    const y = HEIGHT - ((p.total - min) / range) * HEIGHT;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });

  const line = `M${coords.join(" L")}`;
  const area = `${line} L${WIDTH},${HEIGHT} L0,${HEIGHT} Z`;
  return { line, area };
}

// The history file comes from a cross-origin fetch, not a schema this
// component controls — a malformed entry (missing/non-numeric `total`)
// would otherwise flow into Math.min/max as NaN and silently draw
// nothing while still marking the sparkline "visible".
function isWellFormedPoint(value: unknown): value is Point {
  const point = value as Partial<Point> | null;
  return typeof point?.date === "string" && Number.isFinite(point?.total);
}

function render(rawPoints: unknown[]) {
  const points = rawPoints.filter(isWellFormedPoint);
  if (points.length < MIN_POINTS) return;

  const paths = buildPaths(points);
  linePath.value = paths.line;
  areaPath.value = paths.area;

  const first = points[0];
  const last = points[points.length - 1];
  description.value = `Total downloads grew from ${first.total} on ${first.date} to ${last.total} on ${last.date}.`;
  caption.value = `since ${first.date}`;
  visible.value = true;
}

onMounted(() => {
  fetch(HISTORY_URL)
    .then((res) => (res.ok ? res.json() : null))
    .then((data) => {
      if (Array.isArray(data)) render(data);
    })
    .catch((err) => {
      // No history yet (or offline) — the shields.io badge above still
      // shows the live total, so just leave this hidden.
      console.debug("downloads sparkline: history unavailable", err);
    });
});
</script>

<template>
  <div class="downloads-sparkline" v-show="visible">
    <svg viewBox="0 0 120 28" preserveAspectRatio="none" role="img" aria-labelledby="downloads-sparkline-desc">
      <desc id="downloads-sparkline-desc">{{ description }}</desc>
      <path class="downloads-sparkline-area" :d="areaPath"></path>
      <path class="downloads-sparkline-line" :d="linePath"></path>
    </svg>
    <span class="downloads-sparkline-caption">{{ caption }}</span>
  </div>
</template>

<style>
.downloads-sparkline {
  margin-top: 10px;
  display: inline-flex;
  align-items: center;
  gap: 8px;
}
.downloads-sparkline svg {
  width: 120px;
  height: 28px;
  display: block;
  overflow: visible;
}
.downloads-sparkline-area {
  fill: var(--primary);
  opacity: 0.12;
  stroke: none;
}
.downloads-sparkline-line {
  fill: none;
  stroke: var(--primary);
  stroke-width: 1.5;
  stroke-linejoin: round;
  stroke-linecap: round;
}
.downloads-sparkline-caption {
  color: var(--text-muted);
  font-size: 0.8rem;
}
</style>
