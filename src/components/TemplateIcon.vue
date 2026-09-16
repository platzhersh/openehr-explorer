<!--
  Layout/template icon for the Dashboard's "Templates" stat card (OEH-57).
  A plain inline SVG in the same thin-stroke line-icon style used
  elsewhere (see CompassIcon.vue, AppSidebar.vue's Documentation and
  Settings icons) so it inherits `color` via `currentColor` rather than
  looking like a different design system depending on the platform's
  emoji font. A page split into a header band plus two content blocks —
  reads as "template layout" rather than a generic file, distinct from
  DocumentIcon's plain text lines.
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
    <rect x="3.5" y="3.5" width="17" height="17" rx="2" stroke="currentColor" stroke-width="1.6" />
    <path d="M3.5 8.5h17" stroke="currentColor" stroke-width="1.6" />
    <path d="M9.5 8.5v12" stroke="currentColor" stroke-width="1.6" />
  </svg>
</template>
