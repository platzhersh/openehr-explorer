<!--
  Plus icon for the app's "create new" buttons (Upload Template, New EHR,
  New Composition, Add Server, ...). A plain inline SVG rather than a
  literal "+" text glyph, so the mark renders as a crisp, consistently
  weighted icon instead of a font-dependent character that looks slightly
  different across platforms. A filled circle behind the plus mark reads
  as a small badge, more distinct at a glance than a bare "+" alongside
  the button's own label text.

  The plus mark is a genuine hole cut out of the circle via an SVG mask,
  rather than a stroke painted in a hardcoded `--color-bg` — that hardcoded
  color used to match the app's own background, but not a button's *hover*
  background (e.g. `.btn-primary:hover` swaps both the button background
  and its text/icon color to the primary accent + `--color-bg`, which made
  the circle and the "cut-out" collapse to the same color and the plus mark
  vanish into a solid dot). A mask always reveals whatever is actually
  behind the icon, so it stays legible across every button state/theme.
  The mask id is per-instance so multiple icons on one page don't collide.
-->
<script setup lang="ts">
import { useId } from "vue";

const maskId = `plus-icon-cutout-${useId()}`;
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
    <mask :id="maskId">
      <rect x="0" y="0" width="24" height="24" fill="white" />
      <path d="M12 7.5v9M7.5 12h9" stroke="black" stroke-width="1.8" stroke-linecap="round" />
    </mask>
    <circle
      cx="12"
      cy="12"
      r="10"
      fill="currentColor"
      fill-opacity="0.9"
      :mask="`url(#${maskId})`"
    />
  </svg>
</template>
