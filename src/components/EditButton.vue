<script setup lang="ts">
// Reusable icon-only edit button — see OEH-55. Companion to CopyButton.vue
// (OEH-37), RefreshButton.vue (OEH-52), and DeleteButton.vue (OEH-54): every
// bare "Edit" text button in the app moves to the same icon-button +
// tooltip pattern instead of reimplementing it.
withDefaults(
  defineProps<{
    /** Tooltip/aria-label. */
    title?: string;
    /** Disables the button. */
    disabled?: boolean;
    /** `sm` fits inline in dense rows (list items, table cells); `md` matches a toolbar `.btn.btn-sm`. */
    size?: "sm" | "md";
    /** `ghost` has no border/background, for inline row use; `bordered` looks like a toolbar button. */
    variant?: "ghost" | "bordered";
  }>(),
  {
    title: "Edit",
    disabled: false,
    size: "sm",
    variant: "ghost",
  },
);

defineEmits<{ click: [MouseEvent] }>();
</script>

<template>
  <span class="edit-icon-wrap">
    <button
      type="button"
      class="edit-icon-btn"
      :class="[`size-${size}`, `variant-${variant}`]"
      :disabled="disabled"
      :aria-label="title"
      @click.stop="$emit('click', $event)"
    >
      <svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
        <path
          d="M11.2 2.3a1.3 1.3 0 0 1 1.9 0l.6.6a1.3 1.3 0 0 1 0 1.9l-7.4 7.4-3 .7.7-3 7.2-7.6z"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          d="M9.8 3.7l2.5 2.5"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linecap="round"
        />
      </svg>
    </button>
    <span class="edit-tooltip" aria-hidden="true">{{ title }}</span>
  </span>
</template>

<style scoped>
.edit-icon-wrap {
  position: relative;
  display: inline-flex;
}

.edit-tooltip {
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

.edit-icon-wrap:hover .edit-tooltip,
.edit-icon-wrap:focus-within .edit-tooltip {
  opacity: 1;
  visibility: visible;
  transform: translateX(-50%) translateY(0);
}

.edit-icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  padding: 0;
  cursor: pointer;
  transition: all 0.15s;
  color: var(--color-text-muted);
}
.edit-icon-btn svg {
  width: 100%;
  height: 100%;
}

.edit-icon-btn.size-sm {
  width: 20px;
  height: 20px;
}
.edit-icon-btn.size-sm svg {
  width: 12px;
  height: 12px;
}
.edit-icon-btn.size-md {
  width: 26px;
  height: 26px;
}
.edit-icon-btn.size-md svg {
  width: 14px;
  height: 14px;
}

.edit-icon-btn.variant-ghost {
  border: none;
  background: none;
  border-radius: 3px;
}
.edit-icon-btn.variant-ghost:hover:not(:disabled) {
  color: var(--color-primary);
  background: var(--color-surface);
}

.edit-icon-btn.variant-bordered {
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-surface);
}
.edit-icon-btn.variant-bordered:hover:not(:disabled) {
  color: var(--color-primary);
  border-color: var(--color-primary-dim);
  background: var(--color-surface-hover);
}

.edit-icon-btn:disabled {
  cursor: default;
  opacity: 0.5;
}
</style>
