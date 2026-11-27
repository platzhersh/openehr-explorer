<script setup lang="ts">
// Presentational drop zone shared by TemplateUploadModal.vue and the
// empty-server inline state in TemplateBrowser.vue — the actual upload
// logic lives in the useTemplateUpload composable; this component only
// owns the markup/CSS so neither caller has to duplicate it.
import { ref, watch } from "vue";
import TemplateOverwriteConfirmDialog from "./TemplateOverwriteConfirmDialog.vue";

const props = defineProps<{
  dragOver: boolean;
  uploading: boolean;
  uploadStatus: string | null;
  uploadError: string | null;
  /** True when the server rejected the upload as a duplicate and the user may overwrite. */
  canOverwrite?: boolean;
  /** For the overwrite confirmation: the template that would be replaced and where. */
  overwriteTemplateId?: string | null;
  overwriteServerName?: string | null;
}>();

const emit = defineEmits<{
  dragover: [];
  dragleave: [];
  drop: [event: DragEvent];
  chooseFile: [];
  overwrite: [];
  cancelOverwrite: [];
}>();

// Overwriting destroys the existing template, so it needs an explicit second step.
const confirming = ref(false);
watch(
  () => props.canOverwrite,
  (can) => {
    if (!can) confirming.value = false;
  },
);

function confirmOverwrite() {
  confirming.value = false;
  emit("overwrite");
}
</script>

<template>
  <div>
    <div
      class="upload-zone"
      :class="{ 'drag-over': dragOver }"
      @dragover.prevent="emit('dragover')"
      @dragleave="emit('dragleave')"
      @drop="emit('drop', $event)"
    >
      <p>Drop OPT file here to upload</p>
      <button type="button" class="btn btn-sm" :disabled="uploading" @click="emit('chooseFile')">
        {{ uploading ? "Uploading..." : "Or choose file..." }}
      </button>
    </div>
    <div v-if="uploadStatus" class="upload-msg success">{{ uploadStatus }}</div>
    <div v-if="uploadError" class="upload-msg error">{{ uploadError }}</div>
    <div v-if="canOverwrite" class="overwrite-actions">
      <button type="button" class="btn btn-sm" :disabled="uploading" @click="confirming = true">
        Overwrite existing template
      </button>
      <button
        type="button"
        class="btn btn-sm btn-secondary"
        :disabled="uploading"
        @click="emit('cancelOverwrite')"
      >
        Cancel
      </button>
    </div>
    <TemplateOverwriteConfirmDialog
      v-if="canOverwrite && confirming"
      :template-id="overwriteTemplateId ?? null"
      :server-name="overwriteServerName ?? null"
      @confirm="confirmOverwrite"
      @cancel="confirming = false"
    />
  </div>
</template>

<style scoped>
.upload-zone {
  padding: 24px;
  border: 2px dashed var(--color-border);
  border-radius: var(--radius);
  text-align: center;
  color: var(--color-text-muted);
  font-size: 13px;
  transition:
    border-color 0.15s,
    background-color 0.15s;
  display: flex;
  flex-direction: column;
  gap: 12px;
  align-items: center;
}
.upload-zone.drag-over {
  border-color: var(--color-primary);
  background: rgba(100, 255, 218, 0.05);
}
.upload-zone p {
  margin: 0;
}

.overwrite-actions {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}

.upload-msg {
  margin: 12px 0 0;
  padding: 8px 12px;
  border-radius: var(--radius);
  font-size: 12px;
}
.upload-msg.success {
  color: var(--color-success);
  background: rgba(100, 255, 218, 0.1);
  border: 1px solid rgba(100, 255, 218, 0.3);
}
.upload-msg.error {
  color: var(--color-error);
  background: rgba(255, 90, 90, 0.1);
  border: 1px solid rgba(255, 90, 90, 0.3);
}
</style>
