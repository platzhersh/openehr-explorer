<script setup lang="ts">
// Reusable icon-only refresh button — see OEH-51. Companion to CopyButton.vue
// (OEH-37): every "Refresh" text button in the app moves to the same
// icon-button + tooltip pattern instead of reimplementing it. Shows a small
// tooltip on hover ("Refresh" or the custom `title`); while `loading` is
// true the icon spins and the tooltip switches to "Refreshing…".
withDefaults(
  defineProps<{
    /** Tooltip/aria-label shown before and after refreshing. */
    title?: string;
    /** Spins the icon and disables the button while a refresh is in flight. */
    loading?: boolean;
    /** Disables the button (in addition to `loading`). */
    disabled?: boolean;
    /** `sm` fits inline in dense rows (list items, table cells); `md` matches a toolbar `.btn.btn-sm`. */
    size?: "sm" | "md";
    /** `ghost` has no border/background, for inline row use; `bordered` looks like a toolbar button. */
    variant?: "ghost" | "bordered";
  }>(),
  {
    title: "Refresh",
    loading: false,
    disabled: false,
    size: "sm",
    variant: "ghost",
  },
);

defineEmits<{ click: [MouseEvent] }>();
</script>

<template>
  <span class="refresh-icon-wrap">
    <button
      type="button"
      class="refresh-icon-btn"
      :class="[`size-${size}`, `variant-${variant}`, { loading }]"
      :disabled="disabled || loading"
      :aria-label="loading ? 'Refreshing' : title"
      @click.stop="$emit('click', $event)"
    >
      <svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
        <path
          d="M13 4.5a5.5 5.5 0 1 0 1.2 3.4"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linecap="round"
        />
        <path
          d="M13 2v3h-3"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
    <span class="refresh-tooltip" aria-hidden="true">
      {{ loading ? "Refreshing…" : title }}
    </span>
  </span>
</template>

<style scoped>
.refresh-icon-wrap {
  position: relative;
  display: inline-flex;
}

.refresh-tooltip {
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

.refresh-icon-wrap:hover .refresh-tooltip,
.refresh-icon-wrap:focus-within .refresh-tooltip {
  opacity: 1;
  visibility: visible;
  transform: translateX(-50%) translateY(0);
}

.refresh-icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  padding: 0;
  cursor: pointer;
  transition: all 0.15s;
  color: var(--color-text-muted);
}
.refresh-icon-btn svg {
  width: 100%;
  height: 100%;
}

.refresh-icon-btn.size-sm {
  width: 20px;
  height: 20px;
}
.refresh-icon-btn.size-sm svg {
  width: 12px;
  height: 12px;
}
.refresh-icon-btn.size-md {
  width: 26px;
  height: 26px;
}
.refresh-icon-btn.size-md svg {
  width: 14px;
  height: 14px;
}

.refresh-icon-btn.variant-ghost {
  border: none;
  background: none;
  border-radius: 3px;
}
.refresh-icon-btn.variant-ghost:hover:not(:disabled) {
  color: var(--color-primary);
  background: var(--color-surface);
}

.refresh-icon-btn.variant-bordered {
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-surface);
}
.refresh-icon-btn.variant-bordered:hover:not(:disabled) {
  color: var(--color-primary);
  border-color: var(--color-primary-dim);
  background: var(--color-surface-hover);
}

.refresh-icon-btn:disabled {
  cursor: default;
  opacity: 0.6;
}

.refresh-icon-btn.loading svg {
  animation: refresh-spin 0.8s linear infinite;
  color: var(--color-primary);
}

@keyframes refresh-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
