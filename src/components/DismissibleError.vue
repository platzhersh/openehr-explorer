<!--
  Request error banner (`.error-msg`) with a close button, so a stale failure
  can be clicked away instead of lingering until the next successful request.
  The parent owns the error state and clears it on `dismiss`.
-->
<script setup lang="ts">
defineEmits<{ dismiss: [] }>();
</script>

<template>
  <div class="error-msg dismissible-error" role="alert">
    <div class="dismissible-error-body"><slot /></div>
    <button
      type="button"
      class="dismissible-error-close"
      aria-label="Dismiss error"
      title="Dismiss"
      @click="$emit('dismiss')"
    >
      <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
        <path
          d="M2 2l8 8M10 2l-8 8"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linecap="round"
        />
      </svg>
    </button>
  </div>
</template>

<style scoped>
.dismissible-error {
  display: flex;
  align-items: flex-start;
  gap: 8px;
}
.dismissible-error-body {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
}
.dismissible-error-close {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  padding: 0;
  background: transparent;
  border: none;
  border-radius: var(--radius);
  color: inherit;
  cursor: pointer;
}
.dismissible-error-close:hover {
  background: rgba(255, 107, 107, 0.2);
}
</style>
