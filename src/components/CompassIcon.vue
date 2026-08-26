<!--
  Compass icon for the "take a tour" trigger button (see PRD-0018).
  A plain inline SVG rather than the 🧭 emoji, so it inherits the
  button's `color` via `currentColor` and matches the thin-stroke
  line-icon style used elsewhere (see AppSidebar.vue's Documentation
  and Settings icons) instead of looking like a different design
  system depending on the platform's emoji font.
-->
<template>
  <svg
    width="14"
    height="14"
    viewBox="0 0 24 24"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-hidden="true"
  >
    <circle cx="12" cy="12" r="9" stroke="currentColor" stroke-width="1.5" />
    <polygon
      points="15.5 8.5 13.2 13.2 8.5 15.5 10.8 10.8 15.5 8.5"
      stroke="currentColor"
      stroke-width="1.2"
      stroke-linejoin="round"
      fill="currentColor"
      fill-opacity="0.25"
    />
  </svg>
</template>
