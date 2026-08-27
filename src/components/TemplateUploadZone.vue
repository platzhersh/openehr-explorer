<script setup lang="ts">
// Presentational drop zone shared by TemplateUploadModal.vue and the
// empty-server inline state in TemplateBrowser.vue — the actual upload
// logic lives in the useTemplateUpload composable; this component only
// owns the markup/CSS so neither caller has to duplicate it.
defineProps<{
  dragOver: boolean;
  uploading: boolean;
  uploadStatus: string | null;
  uploadError: string | null;
}>();

const emit = defineEmits<{
  dragover: [];
  dragleave: [];
  drop: [event: DragEvent];
  chooseFile: [];
}>();
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
  transition: all 0.15s;
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
