<script setup lang="ts">
/**
 * Explicit confirmation before replacing a template on the server. Built on
 * ConfirmDialog (focus trap, Escape, inert background, Cancel focused by default).
 */
import ConfirmDialog from "./ConfirmDialog.vue";

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
</script>

<template>
  <ConfirmDialog
    title="Overwrite existing template?"
    confirm-label="Overwrite template"
    id-prefix="overwrite"
    @confirm="emit('confirm')"
    @cancel="emit('cancel')"
  >
    <p>
      <template v-if="templateId">
        The template <strong class="mono">{{ templateId }}</strong>
      </template>
      <template v-else>This template</template>
      already exists<template v-if="serverName"> on <strong>{{ serverName }}</strong></template
      >. Overwriting <strong>replaces it with the file you uploaded</strong>.
    </p>
    <p class="warning">
      The existing template is lost and can't be restored from this app. Download it first if you
      may need it.
    </p>
    <p class="hint">
      If the server refuses a plain upload, EHRbase can only replace a template that no composition
      uses, and needs admin rights.
    </p>
  </ConfirmDialog>
</template>

<style scoped>
p {
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
</style>
