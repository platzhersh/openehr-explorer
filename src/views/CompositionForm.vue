<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useServerStore } from "../stores/server";
import { useEhrStore } from "../stores/ehr";
import { useTemplateStore } from "../stores/template";
import { useCompositionStore } from "../stores/composition";
import { invoke } from "@tauri-apps/api/core";
import EhrCreateDialog from "../components/EhrCreateDialog.vue";
import { normalizeWebTemplate } from "../lib/webtemplate";

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
  // Form can be shown if template is loaded - EHR selection is only required for submission
  return templateStore.selectedWebTemplate && serverStore.activeServerId;
});

// Sorted EHRs (newest first, matching EHR Browser order)
const sortedEhrs = computed(() => {
  return [...ehrStore.ehrs].sort((a, b) => {
    const timeA = a.time_created ? new Date(a.time_created).getTime() : 0;
    const timeB = b.time_created ? new Date(b.time_created).getTime() : 0;
    return timeB - timeA; // Descending order (newest first)
  });
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
  const templateId = props.templateId || (route.params.templateId as string);
  if (templateId) {
    try {
      await templateStore.fetchWebTemplate(serverStore.activeServerId, templateId);
    } catch (e) {
      console.error("Failed to fetch web template:", e);
      error.value = `Failed to load template: ${e}`;
    }
  } else {
    error.value = "No template ID provided";
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

  // Ensure webTemplate is set on the mb-auto-form element when it becomes available
  setTimeout(async () => {
    if (mbFormRef.value && templateStore.selectedWebTemplate) {
      const normalized = normalizeWebTemplate(templateStore.selectedWebTemplate);
      (mbFormRef.value as any).webTemplate = normalized;
      console.log("Web Template loaded (normalized):", normalized);

      // Fetch example FLAT composition from EHRBase (source of truth per Medium article)
      const templateId = props.templateId || (route.params.templateId as string);
      if (templateId && serverStore.activeServerId) {
        try {
          const example = await invoke("get_template_example", {
            serverId: serverStore.activeServerId,
            templateId,
          });
          console.log("EHRBase FLAT example (source of truth):", example);
        } catch (e) {
          console.warn("Could not fetch template example:", e);
        }
      }
    }
  }, 100);
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

function handleMbChange(event: Event) {
  const customEvent = event as CustomEvent;
  console.log("mb-change event received:", customEvent.detail);
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

// Trigger for manual refresh
const previewRefreshTrigger = ref(0);

const previewJson = computed(() => {
  // Access trigger to force re-computation
  previewRefreshTrigger.value;

  // Try to get live data from mb-auto-form if available
  if (mbFormRef.value) {
    try {
      const rawData = (mbFormRef.value as any).export?.() || {};

      // Transform paths (same logic as in handleSubmit)
      const formData: Record<string, unknown> = {};
      for (const [key, value] of Object.entries(rawData)) {
        // Skip ctx/ fields - they're added separately
        if (key.startsWith("ctx/")) {
          continue;
        }

        // Skip context-related fields that medblocks exports
        if (key.match(/\/(language|territory|composer|encoding)\|/)) {
          continue;
        }

        // Skip name fields (these are metadata, not actual content)
        if (key.endsWith("/name")) {
          continue;
        }

        // Skip structural metadata fields
        if (key.match(/event_series\/(duration|period)$/)) {
          continue;
        }

        // Transform to match oehrpy validator expectations
        let transformedKey = key;

        // Step 1: Keep ":0" - EHRBase example shows it's needed

        // Step 2: Handle special case for value paths
        transformedKey = transformedKey.replace(/\/text\/value$/, "/name|value");

        // Step 3: Convert /attribute to |attribute for data value attributes only
        const lastSlashIndex = transformedKey.lastIndexOf("/");
        if (lastSlashIndex !== -1) {
          const lastSegment = transformedKey.substring(lastSlashIndex + 1);
          const pipeAttributes = ["value", "magnitude", "unit", "code", "terminology"];
          if (pipeAttributes.includes(lastSegment)) {
            transformedKey = transformedKey.substring(0, lastSlashIndex) + "|" + lastSegment;
          }
        }

        formData[transformedKey] = value;
      }

      // Add context fields using ctx/ shortcuts
      const isoTime = compositionTime.value
        ? new Date(compositionTime.value).toISOString()
        : new Date().toISOString();

      const payload = {
        ...formData,

        // Context shortcuts - EHRBase expands these automatically
        "ctx/language": language.value,
        "ctx/territory": territory.value,
        "ctx/composer_name": composerName.value,
        "ctx/time": isoTime,
      };

      return JSON.stringify(payload, null, 2);
    } catch (e) {
      console.error("Failed to export form data:", e);
    }
  }

  // Fallback to buildFlatPayload
  return JSON.stringify(buildFlatPayload(), null, 2);
});

function refreshPreview() {
  previewRefreshTrigger.value++;
}

async function handleSubmit() {
  console.log("handleSubmit called");

  if (!selectedEhrId.value || !serverStore.activeServerId) {
    error.value = "Please select an EHR";
    console.error("Validation failed: no EHR or server selected");
    return;
  }

  if (!composerName.value) {
    error.value = "Please enter a composer name";
    console.error("Validation failed: no composer name");
    return;
  }

  error.value = null;
  success.value = null;
  loading.value = true;

  try {
    // Get form data using export() method
    let formData: Record<string, unknown> = {};
    if (mbFormRef.value) {
      try {
        const rawData = (mbFormRef.value as any).export?.() || {};
        console.log("Raw exported data:", rawData);

        // Transform medblocks-ui paths to EHRBase FLAT format
        // EHRBase expects paths like: "template_id/content/field"
        // medblocks-ui exports paths like: "template_short_name/archetype_name:0/field"

        formData = {} as Record<string, unknown>;
        for (const [key, value] of Object.entries(rawData)) {
          // Skip ctx/ fields - they're added separately
          if (key.startsWith("ctx/")) {
            continue;
          }

          // Skip context-related fields that medblocks exports
          // These are handled separately via ctx/* fields
          // Match both "minimal/language|code" and "minimal/minimal:0/language|code"
          if (key.match(/\/(language|territory|composer|encoding)\|/)) {
            continue;
          }

          // Skip name fields (these are metadata, not actual content)
          // FLAT format typically doesn't include name fields
          if (key.endsWith("/name")) {
            continue;
          }

          // Skip structural metadata fields like duration and period at event_series level
          if (key.match(/event_series\/(duration|period)$/)) {
            continue;
          }

          // medblocks-ui paths follow pattern: "template_short/archetype:0/path/to/field"
          // We need to remove "template_short/archetype:0/" and replace with "template_id/"
          // Example: "minimal/minimal:0/event_series/..." -> "minimal_observation.en.v1/event_series/..."

          let transformedKey = key;

          // Based on oehrpy validator, the correct format is:
          // Input: "minimal/minimal:0/event_series/cualquier_evento/arbol/text/value"
          // Output: "minimal/minimal/event_series/cualquier_evento/arbol/value|value"

          // Step 1: Keep ":0" for archetype instances (EHRBase example shows minimal:0)
          // Don't remove :0 - EHRBase needs it!

          // Step 2: Handle the special case for value paths
          // "arbol/text/value" should become "arbol/name|value"
          transformedKey = transformedKey.replace(/\/text\/value$/, "/name|value");

          // Step 3: Convert /attribute to |attribute for known attributes at the end
          // But NOT for structural paths like /time in the middle
          const lastSlashIndex = transformedKey.lastIndexOf("/");
          if (lastSlashIndex !== -1) {
            const lastSegment = transformedKey.substring(lastSlashIndex + 1);
            // Only convert to pipe notation for data value attributes, not structural time
            const pipeAttributes = ["value", "magnitude", "unit", "code", "terminology"];
            if (pipeAttributes.includes(lastSegment)) {
              transformedKey = transformedKey.substring(0, lastSlashIndex) + "|" + lastSegment;
            }
          }

          formData[transformedKey] = value;
        }

        console.log("Form data (transformed):", formData);
      } catch (e) {
        console.error("Failed to export form data:", e);
        throw new Error(`Form export failed: ${e}`);
      }
    } else {
      console.error("mbFormRef is null");
      throw new Error("Form reference not available");
    }

    // Get template ID
    const templateId = props.templateId || (route.params.templateId as string);
    if (!templateId) {
      throw new Error("Template ID not found");
    }

    // Build payload following EHRBase 2.x actual format
    const isoTime = compositionTime.value
      ? new Date(compositionTime.value).toISOString()
      : new Date().toISOString();

    // Build EHRBase-compatible FLAT payload using the transformed form data
    const payload = {
      // Add form data fields
      ...formData,

      // Context shortcuts - EHRBase expands these automatically
      "ctx/language": language.value,
      "ctx/territory": territory.value,
      "ctx/composer_name": composerName.value,
      "ctx/time": isoTime,
    };

    console.log("Final payload:", payload);
    console.log("Template ID:", templateId);

    // Build request details
    const method = isEditMode.value ? "PUT" : "POST";
    const url = isEditMode.value
      ? `/rest/openehr/v1/ehr/${selectedEhrId.value}/composition/${props.compositionUid}`
      : `/rest/openehr/v1/ehr/${selectedEhrId.value}/composition`;

    requestDetails.value = `${method} ${url}\nContent-Type: application/openehr.wt.flat.schema+json\n\n${JSON.stringify(payload, null, 2)}`;

    console.log("Submitting composition...");

    let result: string;
    if (isEditMode.value && props.compositionUid) {
      console.log("Update mode");
      result = await compositionStore.updateComposition(
        serverStore.activeServerId,
        selectedEhrId.value,
        props.compositionUid,
        payload,
      );
      success.value = `Composition updated successfully! New version: ${result}`;
      console.log("Update successful:", result);
    } else {
      console.log("Create mode");
      result = await compositionStore.createComposition(
        serverStore.activeServerId,
        selectedEhrId.value,
        templateId,
        payload,
      );
      success.value = `Composition created successfully! UID: ${result}`;
      console.log("Create successful:", result);
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
    console.error("Composition submission error:", e);
    error.value = String(e);
    responseDetails.value = `Error: ${e}`;
  } finally {
    loading.value = false;
    console.log("handleSubmit completed");
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
  const templateId = props.templateId || (route.params.templateId as string);
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

// Watch for template changes and reset form state
watch(
  () => route.params.templateId,
  async (newTemplateId, oldTemplateId) => {
    if (newTemplateId && newTemplateId !== oldTemplateId) {
      console.log(`Template changed from ${oldTemplateId} to ${newTemplateId}, resetting form`);

      // Reset form state
      flatData.value = {};
      error.value = null;
      success.value = null;
      compositionTime.value = new Date().toISOString().slice(0, 16);

      // Reset medblocks form
      if (mbFormRef.value) {
        (mbFormRef.value as any).reset?.();
      }

      // Load new template
      if (serverStore.activeServerId) {
        try {
          await templateStore.fetchWebTemplate(serverStore.activeServerId, newTemplateId as string);

          // Update webTemplate on form
          setTimeout(() => {
            if (mbFormRef.value && templateStore.selectedWebTemplate) {
              const normalized = normalizeWebTemplate(templateStore.selectedWebTemplate);
              (mbFormRef.value as any).webTemplate = normalized;
              console.log("New Web Template loaded (normalized):", normalized);
            }
          }, 100);

          // Load draft for new template
          loadDraft();
        } catch (e) {
          console.error("Failed to fetch web template:", e);
          error.value = `Failed to load template: ${e}`;
        }
      }
    }
  },
);
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
            <option v-for="ehr in sortedEhrs" :key="ehr.ehr_id" :value="ehr.ehr_id">
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
            <input
              v-model="composerName"
              type="text"
              class="input"
              placeholder="e.g., Dr. Smith"
              required
            />
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
        <mb-auto-form ref="mbFormRef" @mb-submit="handleMbSubmit" @mb-change="handleMbChange" />
      </div>

      <div v-else class="loading">
        <p v-if="!serverStore.activeServerId">No server selected</p>
        <p v-else-if="!templateStore.selectedWebTemplate">Loading template...</p>
        <p v-else-if="!selectedEhrId">Please select an EHR above</p>
        <p v-else>Initializing form...</p>
      </div>

      <!-- Submit -->
      <div class="form-actions">
        <button class="btn btn-secondary" @click="router.back()">Cancel</button>
        <button
          class="btn btn-primary"
          @click="handleSubmit"
          :disabled="!selectedEhrId || !composerName || loading"
          :title="
            !selectedEhrId
              ? 'Please select an EHR'
              : !composerName
                ? 'Please enter composer name'
                : ''
          "
        >
          {{ loading ? "Submitting..." : isEditMode ? "Update Composition" : "Create Composition" }}
        </button>
      </div>

      <!-- Validation messages -->
      <div v-if="!selectedEhrId" class="validation-warning">
        ⚠️ Please select an EHR above before submitting
      </div>
      <div v-else-if="!composerName || composerName.trim() === ''" class="validation-warning">
        ⚠️ Please enter a composer name in the Context section (currently: "{{ composerName }}")
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
        <div class="preview-actions">
          <button class="btn btn-sm" @click="refreshPreview" title="Refresh preview">↻</button>
          <button class="btn btn-sm" @click="copyPreviewJson">Copy JSON</button>
        </div>
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

.preview-actions {
  display: flex;
  gap: 8px;
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

.validation-warning {
  padding: 12px 16px;
  background: rgba(251, 191, 36, 0.1);
  border: 1px solid rgba(251, 191, 36, 0.3);
  border-radius: var(--radius);
  color: #fbbf24;
  font-size: 13px;
  margin-top: 16px;
}
</style>
