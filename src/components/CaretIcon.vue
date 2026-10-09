<script setup lang="ts">
// Shared expand/collapse caret (OEH-107): one SVG chevron used by every
// disclosure in the app instead of per-component `▶ ▼ ▾` text glyphs, which
// render differently per OS/font and can't be sized or aligned consistently.
//
// Convention: `right` = collapsed, `down` = expanded. The icon rotates between
// the two (rather than swapping glyphs), and the rotation is skipped for
// `prefers-reduced-motion`. It draws in `currentColor`, so callers colour it
// with theme tokens, and is decorative (`aria-hidden`): whatever toggles must
// expose its own `aria-expanded` / accessible name.
withDefaults(
  defineProps<{
    /** `right` while collapsed, `down` while expanded. */
    direction?: "right" | "down";
    /** Width/height in px. */
    size?: number;
  }>(),
  { direction: "right", size: 14 },
);
</script>

<template>
  <svg
    class="caret-icon"
    :class="{ down: direction === 'down' }"
    :width="size"
    :height="size"
    viewBox="0 0 16 16"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-hidden="true"
  >
    <path
      d="M6 3.5 10.5 8 6 12.5"
      stroke="currentColor"
      stroke-width="1.6"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
</template>

<style scoped>
.caret-icon {
  display: block;
  flex-shrink: 0;
  transition: transform 0.15s ease;
}

.caret-icon.down {
  transform: rotate(90deg);
}

@media (prefers-reduced-motion: reduce) {
  .caret-icon {
    transition: none;
  }
}
</style>
