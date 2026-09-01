<script setup lang="ts">
// Downloads-over-time sparkline: fetches the daily snapshot history
// recorded by .github/workflows/downloads-snapshot.yml. That workflow
// commits to the dedicated `data` branch (never `main`), so this reads
// it cross-origin straight off raw.githubusercontent.com — no backend
// needed, the site stays fully static. See OEH-25. Ported from the
// vanilla-JS IIFE that used to live at the bottom of index.html.
import { computed, onMounted, ref } from "vue";

const HISTORY_URL = "https://raw.githubusercontent.com/platzhersh/openehr-explorer/data/docs/downloads-history.json";
// A couple of days of noisy, near-flat snapshots isn't a curve worth
// showing — wait for a full week before the sparkline appears.
const MIN_POINTS = 7;
const WIDTH = 120;
const HEIGHT = 28;
// Room the tooltip needs above the curve (its own height plus the gap).
const TOOLTIP_CLEARANCE = 40;

interface Point {
  date: string;
  total: number;
}

interface Coord {
  x: number;
  y: number;
}

const visible = ref(false);
const linePath = ref("");
const areaPath = ref("");
const description = ref("");
const caption = ref("");
const points = ref<Point[]>([]);
const coords = ref<Coord[]>([]);
const hoverIndex = ref<number | null>(null);
// The sparkline sits near the top of the hero, but in embeds (Storybook,
// a narrow viewport) there may be nothing above it to draw into — flip
// the tooltip under the curve when the space above is too tight.
const tooltipBelow = ref(false);

const hovered = computed(() => {
  const i = hoverIndex.value;
  if (i === null) return null;
  const point = points.value[i];
  const coord = coords.value[i];
  if (!point || !coord) return null;
  return { point, coord };
});

function buildCoords(pts: Point[]): Coord[] {
  const totals = pts.map((p) => p.total);
  const min = Math.min(...totals);
  const max = Math.max(...totals);
  const range = max - min || 1; // avoid a divide-by-zero on a flat line

  return pts.map((p, i) => ({
    x: (i / (pts.length - 1)) * WIDTH,
    y: HEIGHT - ((p.total - min) / range) * HEIGHT,
  }));
}

function buildPaths(pts: Coord[]) {
  const line = `M${pts.map((c) => `${c.x.toFixed(1)},${c.y.toFixed(1)}`).join(" L")}`;
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
  const parsed = rawPoints.filter(isWellFormedPoint);
  if (parsed.length < MIN_POINTS) return;

  const pointCoords = buildCoords(parsed);
  const paths = buildPaths(pointCoords);
  points.value = parsed;
  coords.value = pointCoords;
  linePath.value = paths.line;
  areaPath.value = paths.area;

  const first = parsed[0];
  const last = parsed[parsed.length - 1];
  description.value = `Total downloads grew from ${first.total} on ${first.date} to ${last.total} on ${last.date}.`;
  caption.value = `since ${first.date}`;
  visible.value = true;
}

// The pointer lands anywhere along the curve; snap it to the nearest
// daily snapshot so the tooltip always reports a real recorded value
// rather than an interpolated one.
function onPointerMove(event: PointerEvent) {
  const svg = event.currentTarget as SVGSVGElement;
  const rect = svg.getBoundingClientRect();
  if (!rect.width || points.value.length === 0) return;

  const ratio = (event.clientX - rect.left) / rect.width;
  const index = Math.round(ratio * (points.value.length - 1));
  hoverIndex.value = Math.min(Math.max(index, 0), points.value.length - 1);
  tooltipBelow.value = rect.top < TOOLTIP_CLEARANCE;
}

function clearHover() {
  hoverIndex.value = null;
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
    <div class="downloads-sparkline-plot">
      <svg
        viewBox="0 0 120 28"
        preserveAspectRatio="none"
        role="img"
        aria-labelledby="downloads-sparkline-desc"
        @pointermove="onPointerMove"
        @pointerleave="clearHover"
        @pointercancel="clearHover"
      >
        <desc id="downloads-sparkline-desc">{{ description }}</desc>
        <path class="downloads-sparkline-area" :d="areaPath"></path>
        <path class="downloads-sparkline-line" :d="linePath"></path>
        <g v-if="hovered">
          <line
            class="downloads-sparkline-cursor"
            :x1="hovered.coord.x"
            :x2="hovered.coord.x"
            y1="0"
            :y2="HEIGHT"
          ></line>
          <circle class="downloads-sparkline-dot" :cx="hovered.coord.x" :cy="hovered.coord.y" r="2.5"></circle>
        </g>
        <!-- Transparent hit area: the paths alone leave gaps above the
             curve where pointermove would never fire. -->
        <rect class="downloads-sparkline-hit" x="0" y="0" :width="WIDTH" :height="HEIGHT"></rect>
      </svg>
      <div
        v-if="hovered"
        class="downloads-sparkline-tooltip"
        :class="{ 'is-below': tooltipBelow }"
        :style="{ left: `${(hovered.coord.x / WIDTH) * 100}%` }"
        role="status"
      >
        <strong>{{ hovered.point.total.toLocaleString() }}</strong>
        <span aria-hidden="true">·</span>
        <span>{{ hovered.point.date }}</span>
      </div>
    </div>
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
.downloads-sparkline-plot {
  position: relative;
  line-height: 0;
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
.downloads-sparkline-cursor {
  stroke: var(--primary);
  stroke-width: 1;
  opacity: 0.35;
}
.downloads-sparkline-dot {
  fill: var(--primary);
  stroke: var(--bg, #1a1a2e);
  stroke-width: 1;
  vector-effect: non-scaling-stroke;
}
.downloads-sparkline-hit {
  fill: transparent;
  stroke: none;
}
.downloads-sparkline-tooltip {
  position: absolute;
  bottom: calc(100% + 8px);
  transform: translateX(-50%);
  display: flex;
  align-items: baseline;
  gap: 5px;
  padding: 3px 8px;
  border: 1px solid var(--border, #2a3a5c);
  border-radius: 6px;
  background: var(--surface, #1e2a4a);
  color: var(--text, #e0e0e0);
  font-size: 0.72rem;
  line-height: 1.3;
  white-space: nowrap;
  pointer-events: none;
  z-index: 2;
}
.downloads-sparkline-tooltip.is-below {
  bottom: auto;
  top: calc(100% + 8px);
}
.downloads-sparkline-tooltip span {
  color: var(--text-muted);
  font-size: 0.7rem;
}
.downloads-sparkline-caption {
  color: var(--text-muted);
  font-size: 0.8rem;
}
</style>
