<script setup lang="ts">
import type { EhrSearchResult, EhrSummary } from "../stores/ehr";
import CopyButton from "./CopyButton.vue";

defineProps<{
  /** Paginated-list rows are `EhrSummary`; search rows also carry `subject_namespace`. */
  ehr: EhrSummary | EhrSearchResult;
  active: boolean;
}>();

defineEmits<{
  select: [ehrId: string];
}>();
</script>

<template>
  <div class="ehr-item" :class="{ active }">
    <div class="ehr-id">
      <button type="button" class="id-text row-action" @click="$emit('select', ehr.ehr_id)">
        {{ ehr.ehr_id }}
      </button>
      <CopyButton class="row-control" :text="ehr.ehr_id" title="Copy full ID" @click.stop />
    </div>
    <div class="ehr-meta">
      <span v-if="ehr.time_created" class="meta-item">{{ ehr.time_created }}</span>
      <span v-if="ehr.subject_id" class="meta-item">Subject: {{ ehr.subject_id }}</span>
      <span v-if="'subject_namespace' in ehr && ehr.subject_namespace" class="meta-item"
        >NS: {{ ehr.subject_namespace }}</span
      >
    </div>
  </div>
</template>

<style scoped>
.ehr-item {
  position: relative;
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border);
  cursor: pointer;
  transition: background 0.15s;
}
.ehr-item:hover {
  background: var(--color-surface);
}
.ehr-item.active {
  background: var(--color-surface);
  border-left: 3px solid var(--color-primary);
}

.ehr-id {
  display: flex;
  align-items: center;
  gap: 8px;
}
.id-text {
  font-family: var(--font-mono);
  font-size: 13px;
}
.ehr-meta {
  margin-top: 4px;
  display: flex;
  gap: 12px;
}
.meta-item {
  font-size: 11px;
  color: var(--color-text-muted);
}
</style>
