<!--
  Funnel/filter icon for the EHR Browser's "build filters" trigger button.
  A plain inline SVG rather than an emoji glyph, so it inherits the
  button's `color` via `currentColor` and matches the thin-stroke
  line-icon style used elsewhere (see CompassIcon.vue, AppSidebar.vue's
  Documentation and Settings icons).
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
    <polygon
      points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"
      stroke="currentColor"
      stroke-width="1.8"
      stroke-linejoin="round"
      stroke-linecap="round"
      fill="currentColor"
      fill-opacity="0.15"
    />
  </svg>
</template>
