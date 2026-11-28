<script setup lang="ts">
// Warns that the composition on screen is deleted or superseded, with the way
// out: restore the delete, or open the latest version.
export interface CompositionState {
  deleted: boolean;
  is_latest: boolean;
  latest_uid: string;
}

defineProps<{
  state: CompositionState;
  restoring?: boolean;
  error?: string | null;
}>();

defineEmits<{ restore: []; viewLatest: [] }>();
</script>

<template>
  <div v-if="state.deleted || !state.is_latest" class="state-banner" role="status">
    <span v-if="state.deleted" class="badge">Deleted</span>
    <span v-if="!state.is_latest" class="badge">Not latest version</span>
    <span class="state-text">
      {{
        state.deleted
          ? "This composition has been deleted. Restoring it creates a new version."
          : "A newer version of this composition exists."
      }}
    </span>
    <button
      v-if="state.deleted"
      type="button"
      class="btn btn-sm"
      :disabled="restoring"
      @click="$emit('restore')"
    >
      {{ restoring ? "Restoring..." : "Restore" }}
    </button>
    <button v-else type="button" class="btn btn-sm" @click="$emit('viewLatest')">
      View latest version
    </button>
    <span v-if="error" class="state-error" role="alert">{{ error }}</span>
  </div>
</template>

<style scoped>
.state-banner {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-surface);
  font-size: 13px;
}

.state-text {
  color: var(--color-text-secondary);
}

.state-error {
  flex-basis: 100%;
  color: var(--color-error);
  overflow-wrap: anywhere;
}
</style>
