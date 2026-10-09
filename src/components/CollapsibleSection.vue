<script setup lang="ts">
// Reusable disclosure: a toggle row with a caret icon that shows/hides its
// content (e.g. "Advanced settings" in the server form). Use it instead of
// hand-rolling a button + chevron + v-if each time, so every collapse/expand
// in the app looks and behaves the same: SVG caret that rotates, keyboard
// accessible (`aria-expanded`/`aria-controls`), reduced-motion aware.
//
// Bind with `v-model:open`; leave it unbound for an uncontrolled section that
// manages its own state. The body is not rendered while collapsed, so nothing
// inside it is focusable or announced.
import { useId } from "vue";

withDefaults(
  defineProps<{
    /** Label shown next to the caret. Use the `title` slot for richer content. */
    title?: string;
  }>(),
  { title: "" },
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
    </button>
    <div v-if="open" :id="contentId" class="collapsible-content">
      <slot />
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

.collapsible-content {
  margin-top: 10px;
}

@media (prefers-reduced-motion: reduce) {
  .collapsible-caret {
    transition: none;
  }
}
</style>
