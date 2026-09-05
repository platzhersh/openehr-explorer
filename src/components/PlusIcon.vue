<!--
  Plus icon for the app's "create new" buttons (Upload Template, New EHR,
  New Composition, Add Server, ...). A plain inline SVG rather than a
  literal "+" text glyph, so the mark renders as a crisp, consistently
  weighted icon instead of a font-dependent character that looks slightly
  different across platforms. A filled circle behind the plus mark reads
  as a small badge, more distinct at a glance than a bare "+" alongside
  the button's own label text.
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
    <circle cx="12" cy="12" r="10" fill="currentColor" fill-opacity="0.9" />
    <path
      d="M12 7.5v9M7.5 12h9"
      stroke="var(--color-bg, #1a1a2e)"
      stroke-width="1.8"
      stroke-linecap="round"
    />
  </svg>
</template>
