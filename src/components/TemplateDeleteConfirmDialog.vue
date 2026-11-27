<script setup lang="ts">
/** Explicit confirmation before permanently deleting a template from the server. */
import ConfirmDialog from "./ConfirmDialog.vue";

defineProps<{
  templateId: string;
  serverName: string | null;
  /** Shown inside the dialog when the server refused the deletion. */
  error?: string | null;
  busy?: boolean;
}>();

const emit = defineEmits<{
  confirm: [];
  cancel: [];
}>();
</script>

<template>
  <ConfirmDialog
    title="Delete template?"
    :confirm-label="busy ? 'Deleting…' : 'Delete template'"
    id-prefix="delete-template"
    @confirm="!busy && emit('confirm')"
    @cancel="emit('cancel')"
  >
    <p>
      The template <strong class="mono">{{ templateId }}</strong>
      <template v-if="serverName"> on <strong>{{ serverName }}</strong></template> will be
      <strong>permanently removed</strong>.
    </p>
    <p class="warning">
      This can't be undone from this app. Download the OPT first if you may need it again.
    </p>
    <p class="hint">
      Needs the server's Admin API and admin rights. The server refuses if a composition still uses
      the template.
    </p>
    <p v-if="error" class="warning" role="alert">{{ error }}</p>
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
