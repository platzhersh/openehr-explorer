<!--
  Document icon for the Dashboard's "Compositions" stat card (OEH-57).
  A plain inline SVG in the same thin-stroke line-icon style used
  elsewhere (see CompassIcon.vue, AppSidebar.vue's Documentation and
  Settings icons) so it inherits `color` via `currentColor` rather than
  looking like a different design system depending on the platform's
  emoji font. Distinct from the sidebar's inline "Documentation" glyph
  (folded corner instead of a plain rectangle) so the two read as
  different concepts at a glance.
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
    <path
      d="M7 2.5h7l4 4v13a1 1 0 01-1 1H7a1 1 0 01-1-1v-16a1 1 0 011-1z"
      stroke="currentColor"
      stroke-width="1.6"
      stroke-linejoin="round"
    />
    <path d="M14 2.5v4h4" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" />
    <path
      d="M8.5 12.5h7M8.5 15.5h7M8.5 18.5h4.5"
      stroke="currentColor"
      stroke-width="1.6"
      stroke-linecap="round"
    />
  </svg>
</template>
