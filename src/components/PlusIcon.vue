<!--
  Plus icon for the app's "create new" buttons (Upload Template, New EHR,
  New Composition, Add Server, ...). A plain inline SVG rather than a
  literal "+" text glyph, so the mark renders as a crisp, consistently
  weighted line icon instead of a font-dependent character that looks
  slightly different across platforms. Matches the thin-stroke line-icon
  style used elsewhere (see CompassIcon.vue, FilterIcon.vue).
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
    <path d="M12 5v14M5 12h14" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" />
  </svg>
</template>
