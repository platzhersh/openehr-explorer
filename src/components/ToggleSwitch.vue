<script setup lang="ts">
/**
 * ToggleSwitch — iOS/Android-style sliding pill toggle, built on top of a
 * real `<input type="checkbox">` so native keyboard navigation (Space to
 * toggle, Tab to focus) and form semantics just work. Visual-only wrapping
 * of the underlying checkbox — no custom focus/a11y plumbing needed.
 *
 * Uses v-model with a boolean.
 */

const model = defineModel<boolean>({ required: true });

defineProps<{
  /** Short label shown next to the switch. */
  label?: string;
  /** Disables interaction and greys out the visual. */
  disabled?: boolean;
  /** Optional id for <label for=…> association if you need it. */
  id?: string;
}>();
</script>

<template>
  <label class="toggle" :class="{ 'is-disabled': disabled }">
    <input :id="id" v-model="model" type="checkbox" :disabled="disabled" class="toggle-input" />
    <span class="toggle-track" aria-hidden="true">
      <span class="toggle-thumb"></span>
    </span>
    <span v-if="label" class="toggle-label">{{ label }}</span>
    <slot />
  </label>
</template>

<style scoped>
.toggle {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
  user-select: none;
  color: var(--color-text);
  font-weight: 500;
  font-size: 13px;
}

.toggle.is-disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

/* The real checkbox is visually hidden but still focusable and clickable
   via the wrapping <label>, so keyboard nav keeps working. */
.toggle-input {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

.toggle-track {
  position: relative;
  display: inline-block;
  width: 34px;
  height: 20px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 999px;
  transition:
    background 0.18s ease,
    border-color 0.18s ease;
  flex-shrink: 0;
}

.toggle-thumb {
  position: absolute;
  top: 1px;
  left: 1px;
  width: 16px;
  height: 16px;
  background: var(--color-text-muted);
  border-radius: 50%;
  transition:
    transform 0.18s ease,
    background 0.18s ease;
}

/* Checked state */
.toggle-input:checked + .toggle-track {
  background: var(--color-primary-dim);
  border-color: var(--color-primary);
}

.toggle-input:checked + .toggle-track .toggle-thumb {
  transform: translateX(14px);
  background: #fff;
}

/* Keyboard focus ring — lives on the hidden input so it follows the
   native :focus-visible state. */
.toggle-input:focus-visible + .toggle-track {
  outline: 2px solid var(--color-primary);
  outline-offset: 2px;
}

.toggle-label {
  line-height: 1.4;
}
</style>
