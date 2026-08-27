<script setup lang="ts">
import { ref, watch } from "vue";
import { useServerStore } from "../stores/server";
import { useTemplateStore } from "../stores/template";
import { useAnalytics } from "../composables/useAnalytics";
import { open } from "@tauri-apps/plugin-dialog";
import { readTextFile } from "@tauri-apps/plugin-fs";

const props = defineProps<{
  open: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();

const serverStore = useServerStore();
const templateStore = useTemplateStore();
const analytics = useAnalytics();

const dragOver = ref(false);
const uploading = ref(false);
const uploadStatus = ref<string | null>(null);
const uploadError = ref<string | null>(null);

// Reset transient state whenever the modal is (re)opened, so a previous
// upload's success/error message doesn't linger into the next visit.
watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) resetState();
  },
);

function resetState() {
  dragOver.value = false;
  uploading.value = false;
  uploadStatus.value = null;
  uploadError.value = null;
}

async function uploadFile(file: File) {
  if (!serverStore.activeServerId) return;

  uploading.value = true;
  uploadStatus.value = null;
  uploadError.value = null;

  try {
    const text = await file.text();
    const result = await templateStore.uploadTemplate(serverStore.activeServerId, text);
    uploadStatus.value = result;
    void analytics.track("template_uploaded");
    templateStore.fetchTemplates(serverStore.activeServerId);
  } catch (e) {
    uploadError.value = String(e);
  } finally {
    uploading.value = false;
  }
}

async function handleDrop(event: DragEvent) {
  event.preventDefault();
  dragOver.value = false;
  const file = event.dataTransfer?.files[0];
  if (!file) return;

  await uploadFile(file);
}

async function handleFileSelect() {
  try {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: "OPT Files",
          extensions: ["opt", "xml"],
        },
      ],
    });

    if (selected && typeof selected === "string") {
      if (!serverStore.activeServerId) return;

      uploading.value = true;
      uploadStatus.value = null;
      uploadError.value = null;

      try {
        // Read file using Tauri's FS plugin
        const text = await readTextFile(selected);

        const result = await templateStore.uploadTemplate(serverStore.activeServerId, text);
        uploadStatus.value = result;
        void analytics.track("template_uploaded");
        templateStore.fetchTemplates(serverStore.activeServerId);
      } catch (e) {
        uploadError.value = String(e);
      } finally {
        uploading.value = false;
      }
    }
  } catch (e) {
    uploadError.value = String(e);
  }
}

function handleClose() {
  emit("close");
}
</script>

<template>
  <div v-if="open" class="dialog-overlay" @click.self="handleClose">
    <div class="dialog">
      <div class="dialog-header">
        <h2>Upload Template</h2>
        <button class="close-btn" @click="handleClose">×</button>
      </div>

      <div class="dialog-body">
        <p class="dialog-intro">
          Publish an Operational Template (OPT) XML file to the active server.
        </p>

        <div
          class="upload-zone"
          :class="{ 'drag-over': dragOver }"
          @dragover.prevent="dragOver = true"
          @dragleave="dragOver = false"
          @drop="handleDrop"
        >
          <p>Drop OPT file here to upload</p>
          <button class="btn btn-sm" :disabled="uploading" @click="handleFileSelect">
            {{ uploading ? "Uploading..." : "Or choose file..." }}
          </button>
        </div>
        <div v-if="uploadStatus" class="upload-msg success">{{ uploadStatus }}</div>
        <div v-if="uploadError" class="upload-msg error">{{ uploadError }}</div>

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
  margin: 16px 0 0;
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

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 24px;
}
</style>
