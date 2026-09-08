<!--
  Closed-padlock icon for the app's "this is stored securely" markers (the
  credential-backend badge on a server profile card, the "stored securely"
  hints in the server form). A plain inline SVG rather than the 🔒 emoji,
  which each platform draws in its own style and palette — a chunky
  gold-and-grey glyph that ignores the surrounding text color and looks
  nothing like the app's other, stroke-drawn icons. This inherits `color`
  via `currentColor`, so it takes on whatever green/muted tone the badge or
  hint it sits in already uses. Same idea as PlusIcon.vue.
-->
<template>
  <svg
    width="12"
    height="12"
    viewBox="0 0 16 16"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-hidden="true"
  >
    <rect
      x="3.15"
      y="7.3"
      width="9.7"
      height="6.55"
      rx="1.6"
      stroke="currentColor"
      stroke-width="1.3"
    />
    <path
      d="M5.75 7.3V5.05a2.25 2.25 0 0 1 4.5 0V7.3"
      stroke="currentColor"
      stroke-width="1.3"
      stroke-linecap="round"
    />
  </svg>
</template>
