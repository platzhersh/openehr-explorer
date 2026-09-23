<!--
  Database/cylinder icon for the Dashboard's "EHRs" stat card (OEH-57).
  A plain inline SVG in the same thin-stroke line-icon style used
  elsewhere (see CompassIcon.vue, AppSidebar.vue's Documentation and
  Settings icons) so it inherits `color` via `currentColor` rather than
  looking like a different design system depending on the platform's
  emoji font.
-->
<template>
  <svg
    width="18"
    height="18"
    viewBox="0 0 24 24"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-hidden="true"
  >
    <ellipse cx="12" cy="6" rx="7" ry="3" stroke="currentColor" stroke-width="1.6" />
    <path
      d="M5 6v12c0 1.66 3.13 3 7 3s7-1.34 7-3V6"
      stroke="currentColor"
      stroke-width="1.6"
      stroke-linecap="round"
    />
    <path
      d="M5 12c0 1.66 3.13 3 7 3s7-1.34 7-3"
      stroke="currentColor"
      stroke-width="1.6"
      stroke-linecap="round"
    />
  </svg>
</template>
