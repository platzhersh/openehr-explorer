<script setup lang="ts">
import { ref } from "vue";
import { useServerStore } from "../stores/server";
import { useEhrStore } from "../stores/ehr";

defineProps<{
  open: boolean;
}>();

const emit = defineEmits<{
  close: [];
  created: [ehrId: string];
}>();

const serverStore = useServerStore();
const ehrStore = useEhrStore();

const subjectNamespace = ref("");
const subjectId = ref("");
const isQueryable = ref(true);
const isModifiable = ref(true);
const customEhrId = ref("");
const loading = ref(false);
const error = ref<string | null>(null);
const success = ref(false);
const createdEhrId = ref("");

async function handleSubmit() {
  if (!serverStore.activeServerId) {
    error.value = "No active server selected";
    return;
  }

  loading.value = true;
  error.value = null;
  success.value = false;

  try {
    const result = await ehrStore.createEhr(serverStore.activeServerId, {
      subject_namespace: subjectNamespace.value || undefined,
      subject_id: subjectId.value || undefined,
      is_queryable: isQueryable.value,
      is_modifiable: isModifiable.value,
      ehr_id: customEhrId.value || undefined,
    });

    createdEhrId.value = result.ehr_id;
    success.value = true;
    emit("created", result.ehr_id);

    // Reset form after short delay
    setTimeout(() => {
      emit("close");
      resetForm();
    }, 2000);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

function resetForm() {
  subjectNamespace.value = "";
  subjectId.value = "";
  isQueryable.value = true;
  isModifiable.value = true;
  customEhrId.value = "";
  error.value = null;
  success.value = false;
  createdEhrId.value = "";
}

function handleClose() {
  emit("close");
  resetForm();
}

async function copyEhrId() {
  if (createdEhrId.value) {
    await navigator.clipboard.writeText(createdEhrId.value);
  }
}
</script>

<template>
  <div v-if="open" class="dialog-overlay" @click.self="handleClose">
    <div class="dialog">
      <div class="dialog-header">
        <h2>Create New EHR</h2>
        <button class="close-btn" @click="handleClose">×</button>
      </div>

      <div class="dialog-body">
        <form @submit.prevent="handleSubmit">
          <div class="form-section">
            <h3>Subject Identity (Optional)</h3>
            <div class="form-row">
              <div class="form-field">
                <label>Namespace</label>
                <input
                  v-model="subjectNamespace"
                  type="text"
                  class="input"
                  placeholder="e.g., uk.nhs.nhs_number, ch.ahv"
                />
                <span class="field-hint">e.g., uk.nhs.nhs_number, ch.ahv</span>
              </div>
              <div class="form-field">
                <label>External ID</label>
                <input
                  v-model="subjectId"
                  type="text"
                  class="input"
                  placeholder="The identifier value"
                />
              </div>
            </div>
          </div>

          <div class="form-section">
            <h3>EHR Flags</h3>
            <div class="form-row">
              <label class="checkbox-label">
                <input v-model="isQueryable" type="checkbox" />
                <span>Is Queryable</span>
              </label>
              <label class="checkbox-label">
                <input v-model="isModifiable" type="checkbox" />
                <span>Is Modifiable</span>
              </label>
            </div>
          </div>

          <div class="form-section">
            <h3>Custom EHR ID (Optional)</h3>
            <div class="form-field">
              <input
                v-model="customEhrId"
                type="text"
                class="input"
                placeholder="Leave blank to auto-generate"
              />
              <span class="field-hint">Leave blank to let the server generate a UUID</span>
            </div>
          </div>

          <div v-if="error" class="error-banner">{{ error }}</div>
          <div v-if="success" class="success-banner">
            EHR created successfully!
            <div class="ehr-id-display">
              <code>{{ createdEhrId }}</code>
              <button type="button" class="btn btn-sm" @click="copyEhrId">Copy</button>
            </div>
          </div>

          <div class="dialog-actions">
            <button type="button" class="btn btn-secondary" @click="handleClose">
              Cancel
            </button>
            <button type="submit" class="btn btn-primary" :disabled="loading">
              {{ loading ? "Creating..." : "Create EHR" }}
            </button>
          </div>
        </form>
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
  max-width: 600px;
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

.form-section {
  margin-bottom: 24px;
}

.form-section h3 {
  font-size: 14px;
  font-weight: 600;
  margin: 0 0 12px 0;
  color: var(--color-text-secondary);
}

.form-row {
  display: flex;
  gap: 16px;
}

.form-field {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.form-field label {
  font-size: 13px;
  font-weight: 500;
  color: var(--color-text-secondary);
}

.field-hint {
  font-size: 11px;
  color: var(--color-text-muted);
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  cursor: pointer;
}

.checkbox-label input[type="checkbox"] {
  cursor: pointer;
}

.error-banner {
  padding: 12px;
  background: rgba(255, 90, 90, 0.1);
  border: 1px solid rgba(255, 90, 90, 0.3);
  border-radius: var(--radius);
  color: var(--color-error);
  font-size: 13px;
  margin-bottom: 16px;
}

.success-banner {
  padding: 12px;
  background: rgba(100, 255, 218, 0.1);
  border: 1px solid rgba(100, 255, 218, 0.3);
  border-radius: var(--radius);
  color: var(--color-success);
  font-size: 13px;
  margin-bottom: 16px;
}

.ehr-id-display {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
}

.ehr-id-display code {
  flex: 1;
  font-family: var(--font-mono);
  font-size: 12px;
  padding: 6px 8px;
  background: rgba(100, 255, 218, 0.05);
  border-radius: 4px;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 24px;
}
</style>
