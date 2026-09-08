<script setup lang="ts">
// Reusable icon-only search button — opens a panel's search/filter overlay
// the same way Ctrl/Cmd+F does. Companion to CopyButton.vue (OEH-37),
// RefreshButton.vue (OEH-52), DeleteButton.vue (OEH-54) and EditButton.vue
// (OEH-55), and deliberately the same shape and sizes, since it usually
// sits right next to a CopyButton in a viewer's floating action row.
//
// The keyboard shortcut is discoverable only by trying it, so a panel that
// supports search needs a visible affordance too — this is it. Unlike its
// companions it ships no tooltip markup of its own: it uses the shared
// `[data-tooltip]` utility in shared-utilities.css, which renders the same
// chip without a wrapper span.
withDefaults(
  defineProps<{
    /** Tooltip/aria-label. Mention the shortcut, since the button teaches it. */
    title?: string;
    disabled?: boolean;
    /** `sm` fits inline in dense rows; `md` matches a toolbar `.btn.btn-sm`. */
    size?: "sm" | "md";
    /** `ghost` has no border/background, for inline row use; `bordered` looks like a toolbar button. */
    variant?: "ghost" | "bordered";
  }>(),
  {
    title: "Search (Ctrl+F)",
    disabled: false,
    size: "md",
    variant: "bordered",
  },
);

defineEmits<{ click: [MouseEvent] }>();
</script>

<template>
  <button
    type="button"
    class="search-icon-btn"
    :class="[`size-${size}`, `variant-${variant}`]"
    :disabled="disabled"
    :aria-label="title"
    :data-tooltip="title"
    @click.stop="$emit('click', $event)"
  >
    <svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
      <path
        d="M7 12a5 5 0 1 0 0-10 5 5 0 0 0 0 10Z"
        stroke="currentColor"
        stroke-width="1.5"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
      <path
        d="M10.5 10.5 14 14"
        stroke="currentColor"
        stroke-width="1.5"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </svg>
  </button>
</template>

<style scoped>
.search-icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  padding: 0;
  cursor: pointer;
  transition: all 0.15s;
  color: var(--color-text-muted);
}
.search-icon-btn svg {
  width: 100%;
  height: 100%;
}

.search-icon-btn.size-sm {
  width: 20px;
  height: 20px;
}
.search-icon-btn.size-sm svg {
  width: 12px;
  height: 12px;
}
.search-icon-btn.size-md {
  width: 26px;
  height: 26px;
}
.search-icon-btn.size-md svg {
  width: 14px;
  height: 14px;
}

.search-icon-btn.variant-ghost {
  border: none;
  background: none;
  border-radius: 3px;
}
.search-icon-btn.variant-ghost:hover:not(:disabled) {
  color: var(--color-primary);
  background: var(--color-surface);
}

.search-icon-btn.variant-bordered {
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-surface);
}
.search-icon-btn.variant-bordered:hover:not(:disabled) {
  color: var(--color-primary);
  border-color: var(--color-primary-dim);
  background: var(--color-surface-hover);
}

.search-icon-btn:disabled {
  cursor: default;
  opacity: 0.5;
}
</style>
