<script setup lang="ts">
import type { DownloadToastState } from "../composables/useFileDownload";

defineProps<{
  toast: DownloadToastState | null;
}>();

const emit = defineEmits<{
  reveal: [];
  dismiss: [];
}>();
</script>

<template>
  <Transition name="download-toast">
    <div
      v-if="toast"
      class="download-toast"
      :class="{ 'download-toast--error': toast.isError }"
      role="status"
    >
      <svg
        v-if="!toast.isError"
        class="download-toast-icon"
        width="16"
        height="16"
        viewBox="0 0 20 20"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        aria-hidden="true"
      >
        <circle cx="10" cy="10" r="8" stroke="currentColor" stroke-width="1.5" />
        <path
          d="M6.5 10.5 8.75 12.75 13.5 7.5"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
      <svg
        v-else
        class="download-toast-icon"
        width="16"
        height="16"
        viewBox="0 0 20 20"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        aria-hidden="true"
      >
        <circle cx="10" cy="10" r="8" stroke="currentColor" stroke-width="1.5" />
        <path d="M10 6V10.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
        <circle cx="10" cy="13.5" r="0.75" fill="currentColor" />
      </svg>
      <span class="download-toast-message">{{ toast.message }}</span>
      <button
        v-if="!toast.isError"
        type="button"
        class="download-toast-action"
        @click="emit('reveal')"
      >
        Show in Folder
      </button>
      <button
        type="button"
        class="download-toast-close"
        aria-label="Dismiss"
        @click="emit('dismiss')"
      >
        ✕
      </button>
    </div>
  </Transition>
</template>

<style scoped>
.download-toast {
  position: fixed;
  bottom: 1.5rem;
  right: 1.5rem;
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.65rem 0.85rem;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
  color: var(--color-text);
  font-size: 0.85rem;
  z-index: 2000;
  max-width: 26rem;
}

.download-toast-icon {
  flex-shrink: 0;
  color: var(--color-success);
}

.download-toast--error .download-toast-icon {
  color: var(--color-error);
}

.download-toast-message {
  flex: 1;
  min-width: 0;
  overflow-wrap: break-word;
}

.download-toast-action {
  flex-shrink: 0;
  background: transparent;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  color: var(--color-primary);
  font-size: 0.8rem;
  padding: 0.3rem 0.6rem;
  cursor: pointer;
  white-space: nowrap;
}

.download-toast-action:hover {
  background: var(--color-surface-hover);
}

.download-toast-close {
  flex-shrink: 0;
  background: transparent;
  border: none;
  color: var(--color-text-secondary);
  cursor: pointer;
  font-size: 0.75rem;
  padding: 0.2rem;
  line-height: 1;
}

.download-toast-close:hover {
  color: var(--color-text);
}

.download-toast-enter-active,
.download-toast-leave-active {
  transition:
    opacity 0.18s ease,
    transform 0.18s ease;
}

.download-toast-enter-from,
.download-toast-leave-to {
  opacity: 0;
  transform: translateY(0.5rem);
}
</style>
