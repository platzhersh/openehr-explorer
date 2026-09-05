<script setup lang="ts">
// Reusable icon-only delete button — see OEH-54. Companion to CopyButton.vue
// (OEH-37) and RefreshButton.vue (OEH-52): every bare "Delete" text button
// in the app moves to the same icon-button + tooltip pattern instead of
// reimplementing it. Unlike Copy/Refresh, the icon reads as "danger"
// (--color-error) even at rest, matching the `.btn-danger` styling the text
// buttons it replaces used — a delete action should read as destructive
// before the pointer ever reaches it.
withDefaults(
  defineProps<{
    /** Tooltip/aria-label. */
    title?: string;
    /** Disables the button while a delete is in flight. */
    loading?: boolean;
    /** Disables the button (in addition to `loading`). */
    disabled?: boolean;
    /** `sm` fits inline in dense rows (list items, table cells); `md` matches a toolbar `.btn.btn-sm`. */
    size?: "sm" | "md";
    /** `ghost` has no border/background, for inline row use; `bordered` looks like a toolbar button. */
    variant?: "ghost" | "bordered";
  }>(),
  {
    title: "Delete",
    loading: false,
    disabled: false,
    size: "sm",
    variant: "ghost",
  },
);

defineEmits<{ click: [MouseEvent] }>();
</script>

<template>
  <span class="delete-icon-wrap">
    <button
      type="button"
      class="delete-icon-btn"
      :class="[`size-${size}`, `variant-${variant}`]"
      :disabled="disabled || loading"
      :aria-label="title"
      @click.stop="$emit('click', $event)"
    >
      <svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
        <path d="M3 5h10" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
        <path
          d="M6.5 5V3.5a1 1 0 0 1 1-1h1a1 1 0 0 1 1 1V5"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          d="M4.5 5l.6 8a1 1 0 0 0 1 .9h3.8a1 1 0 0 0 1-.9l.6-8"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          d="M6.5 7.5v4M9.5 7.5v4"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linecap="round"
        />
      </svg>
    </button>
    <span class="delete-tooltip" aria-hidden="true">{{ title }}</span>
  </span>
</template>

<style scoped>
.delete-icon-wrap {
  position: relative;
  display: inline-flex;
}

.delete-tooltip {
  position: absolute;
  top: calc(100% + 6px);
  left: 50%;
  transform: translateX(-50%) translateY(-2px);
  padding: 4px 8px;
  border-radius: 6px;
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  color: var(--color-text);
  font-size: 11px;
  font-weight: 500;
  line-height: 1.4;
  white-space: nowrap;
  pointer-events: none;
  opacity: 0;
  visibility: hidden;
  transition:
    opacity 0.12s ease,
    transform 0.12s ease,
    visibility 0.12s;
  z-index: 20;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.35);
}

.delete-icon-wrap:hover .delete-tooltip,
.delete-icon-wrap:focus-within .delete-tooltip {
  opacity: 1;
  visibility: visible;
  transform: translateX(-50%) translateY(0);
}

.delete-icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  padding: 0;
  cursor: pointer;
  transition: all 0.15s;
  color: var(--color-error);
}
.delete-icon-btn svg {
  width: 100%;
  height: 100%;
}

.delete-icon-btn.size-sm {
  width: 20px;
  height: 20px;
}
.delete-icon-btn.size-sm svg {
  width: 12px;
  height: 12px;
}
.delete-icon-btn.size-md {
  width: 26px;
  height: 26px;
}
.delete-icon-btn.size-md svg {
  width: 14px;
  height: 14px;
}

.delete-icon-btn.variant-ghost {
  border: none;
  background: none;
  border-radius: 3px;
}
.delete-icon-btn.variant-ghost:hover:not(:disabled) {
  background: rgba(255, 90, 90, 0.15);
}

.delete-icon-btn.variant-bordered {
  border: 1px solid rgba(255, 90, 90, 0.3);
  border-radius: var(--radius);
  background: rgba(255, 90, 90, 0.1);
}
.delete-icon-btn.variant-bordered:hover:not(:disabled) {
  background: rgba(255, 90, 90, 0.2);
}

.delete-icon-btn:disabled {
  cursor: default;
  opacity: 0.5;
}
</style>
