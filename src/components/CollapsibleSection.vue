<script setup lang="ts">
// Reusable disclosure: a toggle row with a caret icon that shows/hides its
// content (e.g. "Advanced settings" in the server form). Use it instead of
// hand-rolling a button + chevron + v-if each time, so every collapse/expand
// in the app looks and behaves the same: SVG caret that rotates, keyboard
// accessible (`aria-expanded`/`aria-controls`), reduced-motion aware.
//
// Bind with `v-model:open`; leave it unbound for an uncontrolled section that
// manages its own state. The body is not rendered while collapsed, so nothing
// inside it is focusable or announced; its (empty, hidden) container stays in
// the DOM so the toggle's `aria-controls` always points at a real element.
// Expanded content is indented under a thin rule so it reads as "inside" the
// section. `summary` (or the `summary` slot) is a short value shown on the
// toggle row while collapsed, so the current setting is visible without
// opening the section.
import { useId } from "vue";

withDefaults(
  defineProps<{
    /** Label shown next to the caret. Use the `title` slot for richer content. */
    title?: string;
    /** Short current-value hint shown next to the title while collapsed. */
    summary?: string;
  }>(),
  { title: "", summary: "" },
);

const open = defineModel<boolean>("open", { default: false });
const contentId = `collapsible-${useId()}`;
</script>

<template>
  <div class="collapsible-section">
    <button
      type="button"
      class="collapsible-toggle"
      :aria-expanded="open"
      :aria-controls="contentId"
      @click="open = !open"
    >
      <svg
        class="collapsible-caret"
        :class="{ open }"
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
      <slot name="title">{{ title }}</slot>
      <span v-if="!open && (summary || $slots.summary)" class="collapsible-summary">
        <slot name="summary">{{ summary }}</slot>
      </span>
    </button>
    <div v-show="open" :id="contentId" class="collapsible-content">
      <slot v-if="open" />
    </div>
  </div>
</template>

<style scoped>
.collapsible-toggle {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 0;
  background: none;
  border: none;
  border-radius: var(--radius);
  color: var(--color-text-secondary);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}

.collapsible-toggle:hover {
  color: var(--color-text);
}

.collapsible-toggle:focus-visible {
  outline: 2px solid var(--color-primary);
  outline-offset: 2px;
}

.collapsible-caret {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  transition: transform 0.15s ease;
}

.collapsible-caret.open {
  transform: rotate(90deg);
}

.collapsible-summary {
  margin-left: 6px;
  color: var(--color-text-muted);
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 400;
}

/* Thin rule under the caret's centre (caret is 14px wide) + indent, so the
   content visibly belongs to the toggle above it. */
.collapsible-content {
  margin-top: 8px;
  margin-left: 6px;
  padding-left: 16px;
  border-left: 2px solid var(--color-border);
}

@media (prefers-reduced-motion: reduce) {
  .collapsible-caret {
    transition: none;
  }
}
</style>
