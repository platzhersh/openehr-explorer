<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useServerStore } from "../stores/server";
import { useEhrStore } from "../stores/ehr";
import { useTemplateStore } from "../stores/template";
import { useCompositionStore } from "../stores/composition";
import { invoke } from "@tauri-apps/api/core";
import EhrCreateDialog from "../components/EhrCreateDialog.vue";

const props = defineProps<{
  templateId?: string;
  ehrId?: string;
  compositionUid?: string;
}>();

const route = useRoute();
const router = useRouter();
const serverStore = useServerStore();
const ehrStore = useEhrStore();
const templateStore = useTemplateStore();
const compositionStore = useCompositionStore();

const selectedEhrId = ref("");
const composerName = ref("");
const language = ref("en");
const territory = ref("US");
const compositionTime = ref("");
const showEhrDialog = ref(false);
const showPreview = ref(false);
const isEditMode = ref(false);
const loading = ref(false);
const error = ref<string | null>(null);
const success = ref<string | null>(null);
const flatData = ref<Record<string, unknown>>({});
const requestDetails = ref<string>("");
const responseDetails = ref<string>("");

// medblocks-ui form ref
const mbFormRef = ref<HTMLElement | null>(null);

const isReady = computed(() => {
  return selectedEhrId.value && templateStore.selectedWebTemplate && serverStore.activeServerId;
});

// Initialize
onMounted(async () => {
  if (!serverStore.activeServerId) {
    error.value = "No server selected";
    return;
  }

  // Fetch EHRs for selector
  await ehrStore.fetchEhrs(serverStore.activeServerId, 0);

  // Pre-select EHR if provided
  if (props.ehrId) {
    selectedEhrId.value = props.ehrId;
  }

  // Load template
  const templateId = props.templateId || route.params.templateId as string;
  if (templateId) {
    await templateStore.fetchWebTemplate(serverStore.activeServerId, templateId);
  }

  // Edit mode: load existing composition
  if (props.compositionUid && props.ehrId) {
    isEditMode.value = true;
    await loadCompositionForEdit();
  }

  // Set default time
  compositionTime.value = new Date().toISOString().slice(0, 16);

  // Load draft if exists
  loadDraft();
});

async function loadCompositionForEdit() {
  if (!props.ehrId || !props.compositionUid || !serverStore.activeServerId) return;

  try {
    loading.value = true;
    const flatComp = await invoke<Record<string, unknown>>("get_composition_flat", {
      serverId: serverStore.activeServerId,
      ehrId: props.ehrId,
      compositionUid: props.compositionUid,
    });

    // Pre-populate form with FLAT data
    if (mbFormRef.value && flatComp) {
      // Extract context fields
      composerName.value = (flatComp["ctx/composer_name"] as string) || "";
      language.value = (flatComp["ctx/language"] as string) || "en";
      territory.value = (flatComp["ctx/territory"] as string) || "US";

      // Set form value
      setTimeout(() => {
        if (mbFormRef.value) {
          (mbFormRef.value as any).value = flatComp;
        }
      }, 100);
    }
  } catch (e) {
    error.value = `Could not load composition in FLAT format: ${e}`;
  } finally {
    loading.value = false;
  }
}

function handleMbSubmit(event: Event) {
  const customEvent = event as CustomEvent;
  flatData.value = customEvent.detail || {};
}

function buildFlatPayload(): Record<string, unknown> {
  const payload = { ...flatData.value };

  // Add context fields
  payload["ctx/language"] = language.value;
  payload["ctx/territory"] = territory.value;
  payload["ctx/composer_name"] = composerName.value;
  payload["ctx/time"] = compositionTime.value || new Date().toISOString();

  return payload;
}

const previewJson = computed(() => {
  return JSON.stringify(buildFlatPayload(), null, 2);
});

async function handleSubmit() {
  if (!selectedEhrId.value || !serverStore.activeServerId) {
    error.value = "Please select an EHR";
    return;
  }

  error.value = null;
  success.value = null;
  loading.value = true;

  try {
    const payload = buildFlatPayload();

    // Build request details
    const method = isEditMode.value ? "PUT" : "POST";
    const url = isEditMode.value
      ? `/rest/openehr/v1/ehr/${selectedEhrId.value}/composition/${props.compositionUid}`
      : `/rest/openehr/v1/ehr/${selectedEhrId.value}/composition`;

    requestDetails.value = `${method} ${url}\nContent-Type: application/openehr.wt.flat.schema+json\n\n${JSON.stringify(payload, null, 2)}`;

    let result: string;
    if (isEditMode.value && props.compositionUid) {
      result = await compositionStore.updateComposition(
        serverStore.activeServerId,
        selectedEhrId.value,
        props.compositionUid,
        payload
      );
      success.value = `Composition updated successfully! New version: ${result}`;
    } else {
      result = await compositionStore.createComposition(
        serverStore.activeServerId,
        selectedEhrId.value,
        payload
      );
      success.value = `Composition created successfully! UID: ${result}`;
    }

    responseDetails.value = `HTTP 201 Created\n\n${JSON.stringify({ uid: { value: result } }, null, 2)}`;

    // Clear draft
    clearDraft();

    // Navigate to composition view after short delay
    setTimeout(() => {
      router.push({
        name: "composition",
        params: { ehrId: selectedEhrId.value, compositionUid: result },
      });
    }, 2000);
  } catch (e) {
    error.value = String(e);
    responseDetails.value = `Error: ${e}`;
  } finally {
    loading.value = false;
  }
}

function handleReset() {
  if (mbFormRef.value) {
    (mbFormRef.value as any).reset?.();
  }
  composerName.value = "";
  language.value = "en";
  territory.value = "US";
  compositionTime.value = new Date().toISOString().slice(0, 16);
  flatData.value = {};
  error.value = null;
  success.value = null;
  clearDraft();
}

function handleEhrCreated(newEhrId: string) {
  showEhrDialog.value = false;
  selectedEhrId.value = newEhrId;
  // Refresh EHR list
  if (serverStore.activeServerId) {
    ehrStore.fetchEhrs(serverStore.activeServerId, 0);
  }
}

// Draft persistence
const draftKey = computed(() => {
  const templateId = props.templateId || route.params.templateId as string;
  return `composition_draft_${templateId}_${selectedEhrId.value}`;
});

function saveDraft() {
  if (!selectedEhrId.value) return;

  const draft = {
    ehrId: selectedEhrId.value,
    composerName: composerName.value,
    language: language.value,
    territory: territory.value,
    compositionTime: compositionTime.value,
    flatData: flatData.value,
    timestamp: Date.now(),
  };

  localStorage.setItem(draftKey.value, JSON.stringify(draft));
}

function loadDraft() {
  const saved = localStorage.getItem(draftKey.value);
  if (!saved) return;

  try {
    const draft = JSON.parse(saved);
    // Only load if less than 24 hours old
    if (Date.now() - draft.timestamp < 24 * 60 * 60 * 1000) {
      selectedEhrId.value = draft.ehrId;
      composerName.value = draft.composerName;
      language.value = draft.language;
      territory.value = draft.territory;
      compositionTime.value = draft.compositionTime;
      flatData.value = draft.flatData;
    }
  } catch (e) {
    console.error("Failed to load draft:", e);
  }
}

function clearDraft() {
  localStorage.removeItem(draftKey.value);
}

async function copyPreviewJson() {
  await navigator.clipboard.writeText(previewJson.value);
}

// Auto-save draft every 30 seconds
onMounted(() => {
  const draftInterval = setInterval(saveDraft, 30000);

  // Clean up interval on unmount
  onUnmounted(() => {
    if (draftInterval) {
      clearInterval(draftInterval);
    }
  });
});

watch(() => [selectedEhrId.value, composerName.value, flatData.value], saveDraft, { deep: true });
</script>

<template>
  <div class="composition-form">
    <div class="form-container">
      <div class="form-header">
        <h1>{{ isEditMode ? "Edit" : "New" }} Composition</h1>
        <div class="header-actions">
          <button class="btn btn-sm" @click="showPreview = !showPreview">
            {{ showPreview ? "Hide" : "Show" }} FLAT Preview
          </button>
          <button class="btn btn-sm" @click="handleReset">Reset</button>
        </div>
      </div>

      <div v-if="error" class="error-banner">{{ error }}</div>
      <div v-if="success" class="success-banner">{{ success }}</div>

      <!-- EHR Selector -->
      <div class="form-section">
        <h3>Select EHR</h3>
        <div class="ehr-selector">
          <select v-model="selectedEhrId" class="input" :disabled="isEditMode">
            <option value="">-- Select EHR --</option>
            <option v-for="ehr in ehrStore.ehrs" :key="ehr.ehr_id" :value="ehr.ehr_id">
              {{ ehr.ehr_id.substring(0, 8) }}... {{ ehr.subject_id ? `(${ehr.subject_id})` : "" }}
            </option>
          </select>
          <button v-if="!isEditMode" class="btn btn-sm btn-primary" @click="showEhrDialog = true">
            + Create New EHR
          </button>
        </div>
      </div>

      <!-- Context Fields -->
      <div class="form-section">
        <h3>Context</h3>
        <div class="context-grid">
          <div class="form-field">
            <label>Composer Name *</label>
            <input v-model="composerName" type="text" class="input" placeholder="e.g., Dr. Smith" required />
          </div>
          <div class="form-field">
            <label>Language</label>
            <select v-model="language" class="input">
              <option value="en">English</option>
              <option value="de">German</option>
              <option value="fr">French</option>
              <option value="es">Spanish</option>
            </select>
          </div>
          <div class="form-field">
            <label>Territory</label>
            <input v-model="territory" type="text" class="input" placeholder="e.g., US, GB, DE" />
          </div>
          <div class="form-field">
            <label>Time</label>
            <input v-model="compositionTime" type="datetime-local" class="input" />
          </div>
        </div>
      </div>

      <!-- medblocks-ui Form -->
      <div v-if="isReady" class="form-section">
        <h3>Composition Data</h3>
        <mb-form
          ref="mbFormRef"
          :webTemplate="templateStore.selectedWebTemplate"
          @mb-submit="handleMbSubmit"
        />
      </div>

      <div v-else class="loading">
        Loading template...
      </div>

      <!-- Submit -->
      <div class="form-actions">
        <button class="btn btn-secondary" @click="router.back()">Cancel</button>
        <button
          class="btn btn-primary"
          @click="handleSubmit"
          :disabled="!isReady || !composerName || loading"
        >
          {{ loading ? "Submitting..." : isEditMode ? "Update Composition" : "Create Composition" }}
        </button>
      </div>

      <!-- Request/Response Details -->
      <div v-if="requestDetails" class="details-section">
        <h3>Request</h3>
        <pre class="details-pre">{{ requestDetails }}</pre>
      </div>
      <div v-if="responseDetails" class="details-section">
        <h3>Response</h3>
        <pre class="details-pre">{{ responseDetails }}</pre>
      </div>
    </div>

    <!-- FLAT Preview Panel -->
    <div v-if="showPreview" class="preview-panel">
      <div class="preview-header">
        <h3>FLAT JSON Preview</h3>
        <button class="btn btn-sm" @click="copyPreviewJson">
          Copy JSON
        </button>
      </div>
      <pre class="preview-json">{{ previewJson }}</pre>
    </div>

    <!-- EHR Create Dialog -->
    <EhrCreateDialog
      :open="showEhrDialog"
      @close="showEhrDialog = false"
      @created="handleEhrCreated"
    />
  </div>
</template>

<style scoped>
.composition-form {
  display: flex;
  height: 100%;
  overflow: hidden;
}

.form-container {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
}

.form-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--color-border);
}

.form-header h1 {
  font-size: 24px;
  font-weight: 600;
  margin: 0;
}

.header-actions {
  display: flex;
  gap: 8px;
}

.error-banner {
  padding: 12px 16px;
  background: rgba(255, 90, 90, 0.1);
  border: 1px solid rgba(255, 90, 90, 0.3);
  border-radius: var(--radius);
  color: var(--color-error);
  font-size: 13px;
  margin-bottom: 16px;
}

.success-banner {
  padding: 12px 16px;
  background: rgba(100, 255, 218, 0.1);
  border: 1px solid rgba(100, 255, 218, 0.3);
  border-radius: var(--radius);
  color: var(--color-success);
  font-size: 13px;
  margin-bottom: 16px;
}

.form-section {
  margin-bottom: 32px;
}

.form-section h3 {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 16px;
  color: var(--color-text-secondary);
}

.ehr-selector {
  display: flex;
  gap: 12px;
  align-items: center;
}

.ehr-selector select {
  flex: 1;
}

.context-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 16px;
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.form-field label {
  font-size: 13px;
  font-weight: 500;
  color: var(--color-text-secondary);
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 32px;
  padding-top: 24px;
  border-top: 1px solid var(--color-border);
}

.details-section {
  margin-top: 24px;
  padding-top: 24px;
  border-top: 1px solid var(--color-border);
}

.details-section h3 {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 12px;
  color: var(--color-text-secondary);
}

.details-pre {
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
  background: var(--color-surface);
  padding: 16px;
  border-radius: var(--radius);
  border: 1px solid var(--color-border);
  max-height: 300px;
  overflow-y: auto;
}

.preview-panel {
  width: 400px;
  border-left: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  background: var(--color-surface);
}

.preview-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  border-bottom: 1px solid var(--color-border);
}

.preview-header h3 {
  font-size: 14px;
  font-weight: 600;
  margin: 0;
}

.preview-json {
  flex: 1;
  overflow-y: auto;
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.6;
  padding: 16px;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
}

.loading {
  text-align: center;
  padding: 48px;
  color: var(--color-text-muted);
}
</style>
