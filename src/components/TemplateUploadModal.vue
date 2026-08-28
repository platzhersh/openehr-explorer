<script setup lang="ts">
import { watch } from "vue";
import { useTemplateUpload } from "../composables/useTemplateUpload";
import TemplateUploadZone from "./TemplateUploadZone.vue";

const props = defineProps<{
  open: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();

const { dragOver, uploading, uploadStatus, uploadError, resetState, handleDrop, handleFileSelect } =
  useTemplateUpload();

// Reset transient state whenever the modal is (re)opened, so a previous
// upload's success/error message doesn't linger into the next visit.
watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) resetState();
  },
);

function handleClose() {
  emit("close");
}
</script>

<template>
  <div v-if="open" class="dialog-overlay" @click.self="handleClose">
    <div class="dialog">
      <div class="dialog-header">
        <h2>Upload Template</h2>
        <button type="button" class="close-btn" @click="handleClose">×</button>
      </div>

      <div class="dialog-body">
        <p class="dialog-intro">
          Publish an Operational Template (OPT) XML file to the active server.
        </p>

        <TemplateUploadZone
          :drag-over="dragOver"
          :uploading="uploading"
          :upload-status="uploadStatus"
          :upload-error="uploadError"
          @dragover="dragOver = true"
          @dragleave="dragOver = false"
          @drop="handleDrop"
          @choose-file="handleFileSelect"
        />

        <div class="dialog-actions">
          <button type="button" class="btn btn-secondary" @click="handleClose">Close</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.dialog {
  background: var(--color-bg);
  border-radius: var(--radius);
  width: 90%;
  max-width: 480px;
  max-height: 90vh;
  overflow: auto;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.4);
}

.dialog-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 24px;
  border-bottom: 1px solid var(--color-border);
}

.dialog-header h2 {
  font-size: 18px;
  font-weight: 600;
  margin: 0;
}

.close-btn {
  background: none;
  border: none;
  font-size: 28px;
  color: var(--color-text-muted);
  cursor: pointer;
  padding: 0;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
}
.close-btn:hover {
  background: var(--color-surface);
}

.dialog-body {
  padding: 24px;
}

.dialog-intro {
  margin: 0 0 16px;
  font-size: 13px;
  color: var(--color-text-secondary);
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 24px;
}
</style>
