<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useServerStore } from "../stores/server";
import { useEhrStore } from "../stores/ehr";
import { useTemplateStore } from "../stores/template";
import { useCompositionStore } from "../stores/composition";
import { useAnalytics } from "../composables/useAnalytics";
import { invoke } from "@tauri-apps/api/core";
import EhrCreateDialog from "../components/EhrCreateDialog.vue";
import JsonViewer from "../components/JsonViewer.vue";
import { normalizeWebTemplate } from "../lib/webtemplate";

const analytics = useAnalytics();

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
// The Request/Response debug panel shows a plain-text summary line (method
// + URL, or HTTP status) alongside the actual JSON payload, rendered with
// JsonViewer — see ADR-0021. `*Payload` is null until there's something to
// show (or when the response was a bare error string with no JSON body).
const requestSummaryLine = ref<string>("");
const requestPayload = ref<unknown>(null);
const responseSummaryLine = ref<string>("");
const responsePayload = ref<unknown>(null);
// Template ID for edit mode is resolved from the existing composition since
// the /edit/:ehrId/:compositionUid route carries no templateId param.
const resolvedTemplateId = ref<string>("");
// Snapshot of the composition as originally loaded for editing, captured
// once by loadCompositionForEdit. `composerName`/`language`/`territory`/
// `flatData` above track live (possibly user-edited) form state, so Reset
// needs this separate copy to revert to the last-saved composition instead
// of wiping the form blank (see handleReset).
const originalEditState = ref<{
  composerName: string;
  language: string;
  territory: string;
  flatData: Record<string, unknown>;
  compositionTime: string;
} | null>(null);

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
  let templateId = props.templateId || (route.params.templateId as string);

  // Edit mode: the route carries no templateId, so resolve it from the
  // existing composition's archetype details before loading the template.
  // Keep the fetched (structured) composition around — loadCompositionForEdit
  // needs it too, for composer/language/territory (see there for why).
  let existingComposition: Record<string, unknown> | null = null;
  if (!templateId && props.compositionUid && props.ehrId) {
    try {
      existingComposition = await invoke<Record<string, unknown>>("get_composition", {
        serverId: serverStore.activeServerId,
        ehrId: props.ehrId,
        compositionUid: props.compositionUid,
      });
      templateId =
        ((existingComposition as any)?.archetype_details?.template_id?.value as string) ||
        ((existingComposition as any)?.archetype_node_id as string) ||
        "";
    } catch (e) {
      console.error("Failed to resolve template ID from composition:", e);
    }
  }
  resolvedTemplateId.value = templateId || "";

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
    await loadCompositionForEdit(existingComposition);
  }

  // Set default time — edit mode already restored the composition's own
  // context/start_time above; only a genuinely new composition wants "now".
  if (!isEditMode.value) {
    compositionTime.value = toDatetimeLocalValue(new Date());
  }

  // Load draft if exists
  loadDraft();

  // Push the web template — and, in edit mode, the composition's existing
  // FLAT data — into the mb-auto-form element once it has mounted.
  hydrateMbForm();
});

async function loadCompositionForEdit(existingComposition: Record<string, unknown> | null) {
  if (!props.ehrId || !props.compositionUid || !serverStore.activeServerId) return;

  try {
    loading.value = true;

    // composer/language/territory are RM attributes on the composition
    // itself, not archetype content — EHRbase's FLAT representation (fetched
    // below) omits them entirely, so they have to come from the structured
    // composition instead. Reuse the one already fetched for template-ID
    // resolution when we have it; otherwise fetch it fresh.
    const composition =
      existingComposition ??
      (await invoke<Record<string, unknown>>("get_composition", {
        serverId: serverStore.activeServerId,
        ehrId: props.ehrId,
        compositionUid: props.compositionUid,
      }));
    composerName.value = ((composition as any)?.composer?.name as string) || "";
    language.value = ((composition as any)?.language?.code_string as string) || "en";
    territory.value = ((composition as any)?.territory?.code_string as string) || "US";
    // context/start_time is another RM attribute the FLAT representation
    // omits — read it from the structured composition too. This also feeds
    // any archetype paths the web template defaults from ctx/time (e.g. a
    // HISTORY's origin): leaving compositionTime at its "now" default on
    // edit would resubmit those paths as freshly "now" while paths carried
    // through unchanged from the imported FLAT data (like an already-set
    // history origin) stay at their original, much older time — a mismatch
    // servers reject (e.g. EHRBase/FerroEHR's HISTORY.Events_valid check).
    const startTime = (composition as any)?.context?.start_time?.value as string | undefined;
    if (startTime) {
      compositionTime.value = toDatetimeLocalValue(new Date(startTime));
    }

    const flatComp = await invoke<Record<string, unknown>>("get_composition_flat", {
      serverId: serverStore.activeServerId,
      ehrId: props.ehrId,
      compositionUid: props.compositionUid,
    });

    if (!flatComp) return;
    // hydrateMbForm() pushes this into the mb-auto-form once it has mounted
    // — after its webTemplate, so it knows what to do with these paths.
    flatData.value = flatComp;
    originalEditState.value = {
      composerName: composerName.value,
      language: language.value,
      territory: territory.value,
      flatData: flatComp,
      compositionTime: compositionTime.value,
    };
  } catch (e) {
    error.value = `Could not load composition in FLAT format: ${e}`;
  } finally {
    loading.value = false;
  }
}

// Pushes the web template — and, in edit mode, the composition's existing
// FLAT data (via `flatData`, set by loadCompositionForEdit) — into the
// mb-auto-form element.
//
// mb-auto-form (medblocks-ui) has no settable `value`/`data` property —
// only `templateId`, `ctx`, `config`, `webTemplate`, `variant`, and
// `addContext` are reactive properties. Pre-population instead goes through
// its `import(composition)` method, which rebuilds its internal form DOM
// (including repeatable groups) from `webTemplate` before binding values
// into it. That DOM rebuild happens on a Lit update cycle triggered by
// setting `.webTemplate`, so `import()` has to wait for `updateComplete`
// or it binds against a form that isn't there yet.
//
// Retries briefly since the element itself only mounts once `isReady` has
// flowed through a render pass.
async function hydrateMbForm(attempt = 0) {
  if (!mbFormRef.value || !templateStore.selectedWebTemplate) {
    if (attempt >= 20) {
      console.warn("mb-auto-form did not mount in time; could not initialize the form");
      return;
    }
    setTimeout(() => hydrateMbForm(attempt + 1), 100);
    return;
  }

  const formEl = mbFormRef.value as any;
  const normalized = normalizeWebTemplate(templateStore.selectedWebTemplate);
  formEl.webTemplate = normalized;
  console.log("Web Template loaded (normalized):", normalized);

  if (isEditMode.value && Object.keys(flatData.value).length > 0) {
    if (typeof formEl.updateComplete?.then === "function") {
      await formEl.updateComplete;
    }
    if (typeof formEl.import === "function") {
      formEl.import(flatData.value);
    } else {
      console.warn("mb-auto-form has no import() method; could not pre-populate composition data");
    }
  }

  // Fetch example FLAT composition from EHRBase (source of truth per Medium article)
  const templateId =
    resolvedTemplateId.value || props.templateId || (route.params.templateId as string);
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

// A medblocks-ui field we skip when transforming its export() output —
// either owned by the ctx/* shortcuts we add ourselves, or metadata rather
// than actual composition content.
function shouldSkipMedblocksField(key: string): boolean {
  if (key.startsWith("ctx/")) return true;
  // Context-related fields medblocks exports alongside ctx/* (e.g.
  // "minimal/language|code" or "minimal/minimal:0/language|code")
  if (key.match(/\/(language|territory|composer|encoding)\|/)) return true;
  // Name fields are metadata, not actual content — FLAT format omits them
  if (key.endsWith("/name")) return true;
  // Structural metadata at the event_series level
  if (key.match(/event_series\/(duration|period)$/)) return true;
  return false;
}

// Rewrites one medblocks-ui export() path into EHRBase FLAT format, per the
// oehrpy validator's expectations: "arbol/text/value" -> "arbol/name|value",
// and a trailing /attribute -> |attribute for known data-value attributes
// only (not structural paths like /time in the middle).
function transformMedblocksKey(key: string): string {
  const transformedKey = key.replace(/\/text\/value$/, "/name|value");

  const lastSlashIndex = transformedKey.lastIndexOf("/");
  if (lastSlashIndex === -1) return transformedKey;

  const lastSegment = transformedKey.substring(lastSlashIndex + 1);
  const pipeAttributes = ["value", "magnitude", "unit", "code", "terminology"];
  if (!pipeAttributes.includes(lastSegment)) return transformedKey;

  return transformedKey.substring(0, lastSlashIndex) + "|" + lastSegment;
}

// Transforms a raw medblocks-ui export() object into an EHRBase-compatible
// FLAT payload (minus context fields — see withContextFields). Shared by
// the live preview and the actual submit path so they can't drift apart.
function transformMedblocksExport(rawData: Record<string, unknown>): Record<string, unknown> {
  const formData: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(rawData)) {
    if (shouldSkipMedblocksField(key)) continue;
    formData[transformMedblocksKey(key)] = value;
  }
  return formData;
}

// Converts an absolute instant to the value format <input type="datetime-local">
// expects: a timezone-naive "YYYY-MM-DDTHH:mm" string read as the *local*
// timezone. date.toISOString() returns UTC, so slicing that directly (as this
// code used to) silently reinterprets a UTC wall-clock time as local — only
// correct for UTC browsers, off by the local offset everywhere else, which
// then compounds when the field round-trips back through `new Date(value)`
// on submit.
function toDatetimeLocalValue(date: Date): string {
  const localMs = date.getTime() - date.getTimezoneOffset() * 60000;
  return new Date(localMs).toISOString().slice(0, 16);
}

// Merges the ctx/* shortcuts (which EHRBase expands automatically) onto a
// transformed FLAT payload.
function withContextFields(
  formData: Record<string, unknown>,
  isoTime: string,
): Record<string, unknown> {
  return {
    ...formData,
    "ctx/language": language.value,
    "ctx/territory": territory.value,
    "ctx/composer_name": composerName.value,
    "ctx/time": isoTime,
  };
}

// Mirrors flat_composition_content_type() in src-tauri/src/commands/composition.rs —
// keep the two in sync. FerroEHR only accepts the openEHR REST spec's
// current FLAT media type (application/openehr.wt.flat+json, per its own
// 415 error message); EHRBase's actual deployed endpoint only recognizes
// the older application/openehr.wt.flat.schema+json variant. Better
// Platform / generic servers keep the `.schema` value this app has always
// sent them.
function flatCompositionContentType(): string {
  return serverStore.activeServer?.server_type === "ferro_ehr"
    ? "application/openehr.wt.flat+json"
    : "application/openehr.wt.flat.schema+json";
}

// Trigger for manual refresh
const previewRefreshTrigger = ref(0);

const previewData = computed<unknown>(() => {
  // Access trigger to force re-computation
  void previewRefreshTrigger.value;

  // Try to get live data from mb-auto-form if available
  if (mbFormRef.value) {
    try {
      const rawData = (mbFormRef.value as any).export?.() || {};
      const formData = transformMedblocksExport(rawData);
      const isoTime = compositionTime.value
        ? new Date(compositionTime.value).toISOString()
        : new Date().toISOString();
      return withContextFields(formData, isoTime);
    } catch (e) {
      console.error("Failed to export form data:", e);
    }
  }

  // Fallback to buildFlatPayload
  return buildFlatPayload();
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
    // Get form data using export() method. medblocks-ui paths follow the
    // pattern "template_short/archetype:0/path/to/field" and need
    // transforming to EHRBase FLAT format — see transformMedblocksExport.
    let formData: Record<string, unknown> = {};
    if (mbFormRef.value) {
      try {
        const rawData = (mbFormRef.value as any).export?.() || {};
        console.log("Raw exported data:", rawData);
        formData = transformMedblocksExport(rawData);
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
    const templateId =
      resolvedTemplateId.value || props.templateId || (route.params.templateId as string);
    if (!templateId) {
      throw new Error("Template ID not found");
    }

    // Build payload following EHRBase 2.x actual format
    const isoTime = compositionTime.value
      ? new Date(compositionTime.value).toISOString()
      : new Date().toISOString();
    const payload = withContextFields(formData, isoTime);

    console.log("Final payload:", payload);
    console.log("Template ID:", templateId);

    // Build request details
    const method = isEditMode.value ? "PUT" : "POST";
    // The PUT path takes only the bare VERSIONED_OBJECT uid — EHRBase 404s
    // ("only UUID-type versionedObjectUids are supported") if the
    // ::system::version suffix is left on; that full string belongs in
    // If-Match instead. Mirrors update_composition's versioned_object_uid.
    const url = isEditMode.value
      ? `/rest/openehr/v1/ehr/${selectedEhrId.value}/composition/${props.compositionUid?.split("::")[0]}`
      : `/rest/openehr/v1/ehr/${selectedEhrId.value}/composition?templateId=${encodeURIComponent(templateId)}`;

    const ifMatchLine = isEditMode.value ? `\nIf-Match: "${props.compositionUid}"` : "";
    requestSummaryLine.value = `${method} ${url}\nContent-Type: ${flatCompositionContentType()}\nopenehr-template-id: ${templateId}${ifMatchLine}`;
    requestPayload.value = payload;

    console.log("Submitting composition...");

    let result: string;
    if (isEditMode.value && props.compositionUid) {
      console.log("Update mode");
      result = await compositionStore.updateComposition(
        serverStore.activeServerId,
        selectedEhrId.value,
        props.compositionUid,
        templateId,
        payload,
      );
      success.value = `Composition updated successfully! New version: ${result}`;
      console.log("Update successful:", result);
      void analytics.track("composition_edited");
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
      void analytics.track("composition_created");
    }

    responseSummaryLine.value = "HTTP 201 Created";
    responsePayload.value = { uid: { value: result } };

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
    responseSummaryLine.value = `Error: ${e}`;
    responsePayload.value = null;
  } finally {
    loading.value = false;
    console.log("handleSubmit completed");
  }
}

function handleReset() {
  // Editing an existing composition: revert to the composition as loaded,
  // not a blank form — the user is undoing their in-progress edits, not
  // starting a new composition.
  if (isEditMode.value && originalEditState.value) {
    const original = originalEditState.value;
    composerName.value = original.composerName;
    language.value = original.language;
    territory.value = original.territory;
    flatData.value = original.flatData;
    compositionTime.value = original.compositionTime;
    // mb-auto-form has no reset()/value setter — import() is how its data
    // is (re-)populated (see hydrateMbForm for why .value isn't real here).
    (mbFormRef.value as any)?.import?.(original.flatData);
  } else {
    // New composition: clear() is mb-auto-form's equivalent of reset().
    (mbFormRef.value as any)?.clear?.();
    composerName.value = "";
    language.value = "en";
    territory.value = "US";
    flatData.value = {};
    compositionTime.value = toDatetimeLocalValue(new Date());
  }
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
  const templateId =
    resolvedTemplateId.value || props.templateId || (route.params.templateId as string);
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
      compositionTime.value = toDatetimeLocalValue(new Date());

      // Reset medblocks form (mb-auto-form's method is clear(), not reset())
      if (mbFormRef.value) {
        (mbFormRef.value as any).clear?.();
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
          <button class="btn btn-sm" @click="router.back()">Cancel</button>
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
      <div v-if="requestSummaryLine" class="details-section">
        <h3>Request</h3>
        <pre class="details-summary-line">{{ requestSummaryLine }}</pre>
        <JsonViewer v-if="requestPayload !== null" :value="requestPayload" />
      </div>
      <div v-if="responseSummaryLine" class="details-section">
        <h3>Response</h3>
        <pre class="details-summary-line">{{ responseSummaryLine }}</pre>
        <JsonViewer v-if="responsePayload !== null" :value="responsePayload" />
      </div>
    </div>

    <!-- FLAT Preview Panel -->
    <div v-if="showPreview" class="preview-panel">
      <div class="preview-header">
        <h3>FLAT JSON Preview</h3>
        <div class="preview-actions">
          <button class="btn btn-sm" @click="refreshPreview" title="Refresh preview">↻</button>
        </div>
      </div>
      <div class="preview-json">
        <JsonViewer :value="previewData" />
      </div>
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

.details-summary-line {
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--color-text-secondary);
  margin-bottom: 8px;
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
  padding: 16px;
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
