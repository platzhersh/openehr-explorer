<script setup lang="ts">
// Outcome of checking a composition without saving it: either that the server
// accepts it, or the server's own explanation of why it doesn't.
import type { CompositionValidation } from "../lib/compositionValidation";

defineProps<{ result: CompositionValidation }>();
</script>

<template>
  <div class="validation-result" :class="result.valid ? 'is-valid' : 'is-invalid'" role="status">
    <template v-if="result.valid">
      <strong>Valid.</strong> The server accepts this composition. Nothing was saved.
    </template>
    <template v-else>
      <p>
        <strong>Not valid.</strong>
        {{ result.message ?? "The server rejected the composition without giving a reason." }}
        <code v-if="result.code">{{ result.code }}</code>
      </p>
      <ul v-if="result.details.length">
        <li v-for="detail in result.details" :key="detail">{{ detail }}</li>
      </ul>
      <p>Nothing was saved.</p>
    </template>
  </div>
</template>

<style scoped>
.validation-result {
  padding: 12px 16px;
  border: 1px solid;
  border-radius: var(--radius);
  font-size: 13px;
  line-height: 1.5;
  margin-top: 16px;
  overflow-wrap: anywhere;
}

.is-valid {
  background: rgba(100, 255, 218, 0.1);
  border-color: rgba(100, 255, 218, 0.3);
  color: var(--color-success);
}

.is-invalid {
  background: rgba(255, 90, 90, 0.1);
  border-color: rgba(255, 90, 90, 0.3);
  color: var(--color-error);
}

p {
  margin: 0;
}

ul {
  margin: 8px 0;
  padding-left: 20px;
  font-family: var(--font-mono);
  font-size: 12px;
  white-space: pre-wrap;
}
</style>
