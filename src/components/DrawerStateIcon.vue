<!--
  Height-state icon for the Request Inspector drawer's state buttons
  (see RequestInspector.vue). A plain inline SVG rather than text arrows,
  matching the thin-stroke line-icon style used elsewhere (see
  CompassIcon.vue) — a small window outline with a filled strip at the
  bottom showing how much of it the drawer occupies in each state, so the
  three buttons read as a group rather than as ambiguous arrows.
-->
<script setup lang="ts">
defineProps<{
  state: "collapsed" | "half" | "expanded";
}>();

// Bottom "drawer" strip geometry per state, inside the 3..21 window body.
const DRAWER_RECT: Record<"collapsed" | "half" | "expanded", { y: number; height: number }> = {
  collapsed: { y: 18, height: 3 },
  half: { y: 12, height: 9 },
  expanded: { y: 5, height: 16 },
};
</script>

<template>
  <svg
    width="14"
    height="14"
    viewBox="0 0 24 24"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-hidden="true"
  >
    <rect x="3" y="3" width="18" height="18" rx="2" stroke="currentColor" stroke-width="1.5" />
    <rect
      x="3"
      :y="DRAWER_RECT[state].y"
      width="18"
      :height="DRAWER_RECT[state].height"
      fill="currentColor"
      fill-opacity="0.35"
    />
    <line
      x1="3"
      :x2="21"
      :y1="DRAWER_RECT[state].y"
      :y2="DRAWER_RECT[state].y"
      stroke="currentColor"
      stroke-width="1.5"
    />
  </svg>
</template>
