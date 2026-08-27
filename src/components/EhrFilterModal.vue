<script setup lang="ts">
/**
 * EhrFilterModal — structured, form-based builder for EHR search criteria.
 *
 * Replaces having to hand-type the colon-syntax query string (`subject:...
 * namespace:... hasCompositions:true`) with real form controls: text inputs
 * for the "contains"/"exact match" fields and tri-state selects for the
 * boolean ones. Seeds itself from whatever criteria are already active so
 * reopening the modal shows the current filter state, and emits a plain
 * `EhrSearchCriteria` object on apply — the caller decides how to run it.
 */
import { ref, watch } from "vue";
import type { EhrSearchCriteria } from "../stores/ehr";

const props = defineProps<{
  open: boolean;
  criteria: EhrSearchCriteria;
}>();

const emit = defineEmits<{
  close: [];
  apply: [criteria: EhrSearchCriteria];
}>();

type TriState = "" | "true" | "false";

const ehrIdPrefix = ref("");
const subjectId = ref("");
const subjectNamespace = ref("");
const systemId = ref("");
const modifiable = ref<TriState>("");
// Only "true" is offered — hasCompositions:false isn't supported (AQL has
// no clean way to express "EHR with zero compositions" — see
// build_ehr_search_aql in src-tauri/src/commands/ehr.rs).
const hasCompositions = ref<"" | "true">("");
// Both directions work here since it's checked per-EHR after the search
// runs, not expressed as an AQL predicate.
const hasDirectory = ref<TriState>("");
const formError = ref<string | null>(null);
// Collapsed shortcut-syntax reference for the quick search box's
// colon-syntax (subject:, hasDirectory:true, ...) — folded away since most
// people just use the form fields above, but kept one click away for
// anyone who'd rather type. Replaces what used to be a separate "?" help
// button/modal next to the search box.
const showSyntaxHelp = ref(false);

function boolToTriState(value: boolean | undefined): TriState {
  if (value === true) return "true";
  if (value === false) return "false";
  return "";
}

function seedFromCriteria(criteria: EhrSearchCriteria) {
  ehrIdPrefix.value = criteria.ehr_id_prefix ?? "";
  subjectId.value = criteria.subject_id ?? "";
  subjectNamespace.value = criteria.subject_namespace ?? "";
  systemId.value = criteria.system_id ?? "";
  modifiable.value = boolToTriState(criteria.modifiable);
  hasCompositions.value = criteria.has_compositions === true ? "true" : "";
  hasDirectory.value = boolToTriState(criteria.has_directory);
  formError.value = null;
  showSyntaxHelp.value = false;
}

// Re-seed every time the modal opens, so it always reflects the filters
// currently applied to the list rather than whatever was last typed in.
watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) seedFromCriteria(props.criteria);
  },
);

function buildCriteria(): EhrSearchCriteria {
  const criteria: EhrSearchCriteria = {};
  if (ehrIdPrefix.value.trim()) criteria.ehr_id_prefix = ehrIdPrefix.value.trim();
  if (subjectId.value.trim()) criteria.subject_id = subjectId.value.trim();
  if (subjectNamespace.value.trim()) criteria.subject_namespace = subjectNamespace.value.trim();
  if (systemId.value.trim()) criteria.system_id = systemId.value.trim();
  if (modifiable.value) criteria.modifiable = modifiable.value === "true";
  if (hasCompositions.value) criteria.has_compositions = true;
  if (hasDirectory.value) criteria.has_directory = hasDirectory.value === "true";
  return criteria;
}

function handleApply() {
  const criteria = buildCriteria();
  if (Object.keys(criteria).length === 0) {
    formError.value = "Add at least one filter, or close this dialog to browse without filters.";
    return;
  }
  formError.value = null;
  emit("apply", criteria);
}

function handleClear() {
  ehrIdPrefix.value = "";
  subjectId.value = "";
  subjectNamespace.value = "";
  systemId.value = "";
  modifiable.value = "";
  hasCompositions.value = "";
  hasDirectory.value = "";
  formError.value = null;
}

function handleClose() {
  emit("close");
}
</script>

<template>
  <div v-if="open" class="dialog-overlay" @click.self="handleClose">
    <div class="dialog">
      <div class="dialog-header">
        <h2>Filter EHRs</h2>
        <button type="button" class="close-btn" title="Close" @click="handleClose">&times;</button>
      </div>

      <div class="dialog-body">
        <form @submit.prevent="handleApply">
          <div class="form-section">
            <h3>Identity</h3>
            <div class="form-row">
              <div class="form-field">
                <label for="filter-ehr-id">EHR ID prefix</label>
                <input
                  id="filter-ehr-id"
                  v-model="ehrIdPrefix"
                  type="text"
                  class="input"
                  placeholder="e.g. fde80e0e"
                  autocapitalize="off"
                  autocorrect="off"
                  spellcheck="false"
                />
              </div>
              <div class="form-field">
                <label for="filter-system-id">System ID</label>
                <input
                  id="filter-system-id"
                  v-model="systemId"
                  type="text"
                  class="input"
                  placeholder="Exact match"
                  autocapitalize="off"
                  autocorrect="off"
                  spellcheck="false"
                />
              </div>
            </div>
            <div class="form-row">
              <div class="form-field">
                <label for="filter-subject-id">Subject ID</label>
                <input
                  id="filter-subject-id"
                  v-model="subjectId"
                  type="text"
                  class="input"
                  placeholder="Contains..."
                  autocapitalize="off"
                  autocorrect="off"
                  spellcheck="false"
                />
              </div>
              <div class="form-field">
                <label for="filter-subject-namespace">Subject namespace</label>
                <input
                  id="filter-subject-namespace"
                  v-model="subjectNamespace"
                  type="text"
                  class="input"
                  placeholder="Exact match, e.g. ch.ahv"
                  autocapitalize="off"
                  autocorrect="off"
                  spellcheck="false"
                />
              </div>
            </div>
          </div>

          <div class="form-section">
            <h3>Status</h3>
            <div class="form-row">
              <div class="form-field">
                <label for="filter-modifiable">Modifiable</label>
                <select id="filter-modifiable" v-model="modifiable" class="input">
                  <option value="">Any</option>
                  <option value="true">Yes</option>
                  <option value="false">No</option>
                </select>
              </div>
              <div class="form-field">
                <label for="filter-has-compositions">Has compositions</label>
                <select id="filter-has-compositions" v-model="hasCompositions" class="input">
                  <option value="">Any</option>
                  <option value="true">Yes</option>
                </select>
                <span class="field-hint"
                  >"No" isn't supported yet — AQL has no clean way to say it</span
                >
              </div>
            </div>
            <div class="form-row">
              <div class="form-field">
                <label for="filter-has-directory">Has directory entries</label>
                <select id="filter-has-directory" v-model="hasDirectory" class="input">
                  <option value="">Any</option>
                  <option value="true">Yes</option>
                  <option value="false">No</option>
                </select>
                <span class="field-hint">
                  Checks each matching EHR individually, so this can be slower on large result sets.
                </span>
              </div>
            </div>
          </div>

          <div class="syntax-help">
            <button type="button" class="syntax-toggle" @click="showSyntaxHelp = !showSyntaxHelp">
              {{ showSyntaxHelp ? "Hide" : "Show" }} shortcut syntax for the quick search box
            </button>
            <div v-if="showSyntaxHelp" class="syntax-help-body">
              <table class="help-table">
                <thead>
                  <tr>
                    <th scope="col">Syntax</th>
                    <th scope="col">Meaning</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td class="help-example">fde80e0e...</td>
                    <td>EHR ID prefix match</td>
                  </tr>
                  <tr>
                    <td class="help-example">subject:value</td>
                    <td>Subject ID contains match</td>
                  </tr>
                  <tr>
                    <td class="help-example">namespace:value</td>
                    <td>Subject namespace exact match</td>
                  </tr>
                  <tr>
                    <td class="help-example">system:value</td>
                    <td>System ID exact match</td>
                  </tr>
                  <tr>
                    <td class="help-example">modifiable:true|false</td>
                    <td>EHR status is_modifiable</td>
                  </tr>
                  <tr>
                    <td class="help-example">hasCompositions:true</td>
                    <td>Has compositions (false not supported)</td>
                  </tr>
                  <tr>
                    <td class="help-example">hasDirectory:true|false</td>
                    <td>Has a DIRECTORY (checked per matching EHR)</td>
                  </tr>
                </tbody>
              </table>
              <p class="help-note">Combine terms with spaces (implicit AND).</p>
              <p class="help-note">
                Press Enter or wait 600ms to search. All searches use AQL and appear in the Request
                Inspector.
              </p>
            </div>
          </div>

          <div v-if="formError" class="error-banner">{{ formError }}</div>

          <div class="dialog-actions">
            <button type="button" class="btn btn-secondary" @click="handleClear">Clear all</button>
            <div class="dialog-actions-right">
              <button type="button" class="btn btn-secondary" @click="handleClose">Cancel</button>
              <button type="submit" class="btn btn-primary">Apply filters</button>
            </div>
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
  max-width: 560px;
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
.form-row:not(:last-child) {
  margin-bottom: 12px;
}

.form-field {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}

.form-field label {
  font-size: 13px;
  font-weight: 500;
  color: var(--color-text-secondary);
}

.field-hint {
  font-size: 11px;
  color: var(--color-text-muted);
  line-height: 1.4;
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

.syntax-help {
  margin-bottom: 8px;
}
.syntax-toggle {
  background: none;
  border: none;
  padding: 0;
  font-size: 12px;
  color: var(--color-text-secondary);
  text-decoration: underline;
  cursor: pointer;
}
.syntax-toggle:hover {
  color: var(--color-text);
}
.syntax-help-body {
  margin-top: 10px;
  padding: 12px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
}

.help-table {
  width: 100%;
  font-size: 12px;
  border-collapse: collapse;
}
.help-table td,
.help-table th {
  padding: 3px 8px;
  border-bottom: 1px solid var(--color-border);
}
.help-table th {
  text-align: left;
  font-weight: 600;
  color: var(--color-text-secondary);
}
.help-example {
  font-family: var(--font-mono);
  color: var(--color-primary);
  white-space: nowrap;
}
.help-note {
  font-size: 11px;
  color: var(--color-text-muted);
  margin: 6px 0 0;
}

.dialog-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  margin-top: 8px;
}

.dialog-actions-right {
  display: flex;
  gap: 12px;
}
</style>
