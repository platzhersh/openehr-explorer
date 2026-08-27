<script setup lang="ts">
// Reusable icon-only copy-to-clipboard button — see OEH-37. Originally built
// inline in JsonViewer.vue (ADR-0021); extracted here so every "Copy"/"copy"
// text button in the app can move to the same icon + "copied" checkmark
// pattern instead of reimplementing it.
import { ref } from "vue";

const props = withDefaults(
  defineProps<{
    /** The text copied to the clipboard on click. */
    text: string;
    /** Tooltip/aria-label shown before copying. */
    title?: string;
    /** `sm` fits inline in dense rows (list items, table cells); `md` matches a toolbar `.btn.btn-sm`. */
    size?: "sm" | "md";
    /** `ghost` has no border/background, for inline row use; `bordered` looks like a toolbar button. */
    variant?: "ghost" | "bordered";
  }>(),
  {
    title: "Copy to clipboard",
    size: "sm",
    variant: "ghost",
  },
);

const copied = ref(false);
let resetTimer: ReturnType<typeof setTimeout> | undefined;

async function copy() {
  await navigator.clipboard.writeText(props.text);
  copied.value = true;
  clearTimeout(resetTimer);
  resetTimer = setTimeout(() => (copied.value = false), 1200);
}
</script>

<template>
  <button
    type="button"
    class="copy-icon-btn"
    :class="[`size-${size}`, `variant-${variant}`, { copied }]"
    :title="copied ? 'Copied!' : title"
    :aria-label="copied ? 'Copied' : title"
    @click.stop="copy"
  >
    <svg
      v-if="!copied"
      viewBox="0 0 16 16"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
    >
      <rect x="5" y="5" width="9" height="9" rx="1.5" stroke="currentColor" stroke-width="1.3" />
      <path
        d="M3.5 10.5h-1a1 1 0 0 1-1-1v-7a1 1 0 0 1 1-1h7a1 1 0 0 1 1 1v1"
        stroke="currentColor"
        stroke-width="1.3"
      />
    </svg>
    <svg
      v-else
      viewBox="0 0 16 16"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
    >
      <path
        d="M3 8.5l3 3 7-7"
        stroke="currentColor"
        stroke-width="1.6"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </svg>
  </button>
</template>

<style scoped>
.copy-icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  padding: 0;
  cursor: pointer;
  transition: all 0.15s;
  color: var(--color-text-muted);
}
.copy-icon-btn svg {
  width: 100%;
  height: 100%;
}

.copy-icon-btn.size-sm {
  width: 20px;
  height: 20px;
}
.copy-icon-btn.size-sm svg {
  width: 12px;
  height: 12px;
}
.copy-icon-btn.size-md {
  width: 26px;
  height: 26px;
}
.copy-icon-btn.size-md svg {
  width: 14px;
  height: 14px;
}

.copy-icon-btn.variant-ghost {
  border: none;
  background: none;
  border-radius: 3px;
}
.copy-icon-btn.variant-ghost:hover {
  color: var(--color-primary);
  background: var(--color-surface);
}

.copy-icon-btn.variant-bordered {
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-surface);
}
.copy-icon-btn.variant-bordered:hover {
  color: var(--color-primary);
  border-color: var(--color-primary-dim);
  background: var(--color-surface-hover);
}

.copy-icon-btn.copied {
  color: var(--color-success);
}
.copy-icon-btn.variant-bordered.copied {
  border-color: var(--color-success);
}
</style>
