<script setup lang="ts">
/**
 * Explicit confirmation before replacing a template on the server. Cancel is
 * focused by default so a stray Enter never overwrites, and Escape closes only
 * this dialog (not the upload modal behind it). It is rendered at the end of
 * <body> and the rest of the page is made `inert` while it is open, so neither
 * keyboard users nor assistive technology can reach the controls behind it.
 */
import { onBeforeUnmount, onMounted, ref } from "vue";

defineProps<{
  /** ID of the template that will be replaced; null if it couldn't be read from the OPT. */
  templateId: string | null;
  /** Name of the server the template lives on. */
  serverName: string | null;
}>();

const emit = defineEmits<{
  confirm: [];
  cancel: [];
}>();

const overlay = ref<HTMLElement | null>(null);
const dialog = ref<HTMLElement | null>(null);
const cancelButton = ref<HTMLButtonElement | null>(null);
let previouslyFocused: HTMLElement | null = null;
// Page content made inert while the dialog is open (only what wasn't already inert).
let inertedElements: Element[] = [];

function makeBackgroundInert() {
  inertedElements = Array.from(document.body.children).filter(
    (element) => element !== overlay.value && !element.hasAttribute("inert"),
  );
  for (const element of inertedElements) element.setAttribute("inert", "");
}

function restoreBackground() {
  for (const element of inertedElements) element.removeAttribute("inert");
  inertedElements = [];
}

// Handled in the capture phase on window so Escape closes only this dialog even when
// focus has drifted to the page body (the upload modal behind it listens on window
// in the bubble phase), and so Tab can't leave the dialog.
function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.stopPropagation();
    event.preventDefault();
    emit("cancel");
    return;
  }
  if (event.key !== "Tab" || !dialog.value) return;
  const focusable = Array.from(
    dialog.value.querySelectorAll<HTMLElement>("button:not([disabled])"),
  );
  if (focusable.length === 0) return;
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  const active = document.activeElement;
  const atEnd = event.shiftKey ? active === first : active === last;
  if (!dialog.value.contains(active) || (atEnd && !event.shiftKey)) {
    event.preventDefault();
    first.focus();
  } else if (atEnd) {
    event.preventDefault();
    last.focus();
  }
}

onMounted(() => {
  previouslyFocused = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  window.addEventListener("keydown", onKeydown, true);
  makeBackgroundInert();
  cancelButton.value?.focus();
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown, true);
  restoreBackground();
  if (previouslyFocused?.isConnected) previouslyFocused.focus();
});
</script>

<template>
  <Teleport to="body">
    <div
      ref="overlay"
      class="dialog-overlay"
      role="presentation"
      @click.self="emit('cancel')"
      @keydown.esc.stop="emit('cancel')"
    >
      <div
        ref="dialog"
        class="dialog"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="overwrite-title"
        aria-describedby="overwrite-desc"
      >
        <div class="dialog-header">
          <h2 id="overwrite-title">Overwrite existing template?</h2>
        </div>
        <div class="dialog-body" id="overwrite-desc">
          <p>
            <template v-if="templateId">
              The template <strong class="mono">{{ templateId }}</strong>
            </template>
            <template v-else>This template</template>
            already exists<template v-if="serverName">
              on <strong>{{ serverName }}</strong></template
            >. Overwriting <strong>replaces it with the file you uploaded</strong>.
          </p>
          <p class="warning">
            The existing template is lost and can't be restored from this app. Download it first if
            you may need it.
          </p>
        </div>
        <div class="dialog-actions">
          <button
            ref="cancelButton"
            type="button"
            class="btn btn-secondary"
            @click="emit('cancel')"
          >
            Cancel
          </button>
          <button type="button" class="btn btn-danger" @click="emit('confirm')">
            Overwrite template
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1100; /* above the upload modal (1000) it is opened from */
}

.dialog {
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  width: 90%;
  max-width: 440px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.6);
}

.dialog-header {
  padding: 20px 24px 0;
}
.dialog-header h2 {
  font-size: 18px;
  font-weight: 600;
  margin: 0;
}

.dialog-body {
  padding: 16px 24px 8px;
  font-size: 13px;
  line-height: 1.5;
}
.dialog-body p {
  margin: 0 0 12px;
}
.mono {
  font-family: var(--font-mono, monospace);
  word-break: break-all;
}
.warning {
  color: var(--color-error);
}
.hint {
  color: var(--color-text-secondary);
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 8px 24px 20px;
}
</style>
