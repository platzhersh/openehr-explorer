<script setup lang="ts">
/**
 * DirectoryAddItemModal — richer "add item reference" picker for the
 * DIRECTORY tree editor (OEH-46).
 *
 * Replaces a bare `<select>` limited to this EHR's own compositions with
 * two ways to add an OBJECT_REF:
 *  - a scannable list of this EHR's compositions as clickable rows (the
 *    common case: COMPOSITION / local / HIER_OBJECT_ID)
 *  - a manual-entry fallback (namespace / type / id scheme / id value) for
 *    any other OBJECT_REF — a composition in a different EHR, or a
 *    reference to a non-COMPOSITION object entirely.
 *
 * Stays open after adding from the composition list so several items can be
 * added in one pass; the caller (DirectoryTreeEditor.vue) owns `open` and
 * decides when to close it.
 */
import { ref, watch } from "vue";
import type { CompositionOption } from "../lib/directoryEdit";

const props = defineProps<{
  open: boolean;
  availableCompositions: CompositionOption[];
}>();

const emit = defineEmits<{
  close: [];
  add: [item: { id: string; type: string; namespace: string; idScheme: string }];
}>();

const manualNamespace = ref("local");
const manualType = ref("COMPOSITION");
const manualIdScheme = ref("HIER_OBJECT_ID");
const manualIdValue = ref("");
const manualError = ref<string | null>(null);

// Reset the manual-entry fields every time the modal opens, so a previous
// session's half-filled values don't linger into the next.
watch(
  () => props.open,
  (isOpen) => {
    if (!isOpen) return;
    manualNamespace.value = "local";
    manualType.value = "COMPOSITION";
    manualIdScheme.value = "HIER_OBJECT_ID";
    manualIdValue.value = "";
    manualError.value = null;
  },
);

function addComposition(composition: CompositionOption) {
  emit("add", {
    id: composition.uid,
    type: "COMPOSITION",
    namespace: "local",
    idScheme: "HIER_OBJECT_ID",
  });
}

function addManualReference() {
  const id = manualIdValue.value.trim();
  if (!id) {
    manualError.value = "Enter an id value.";
    return;
  }
  manualError.value = null;
  emit("add", {
    id,
    type: manualType.value.trim() || "COMPOSITION",
    namespace: manualNamespace.value.trim() || "local",
    idScheme: manualIdScheme.value.trim() || "HIER_OBJECT_ID",
  });
  manualIdValue.value = "";
}

function handleClose() {
  emit("close");
}
</script>

<template>
  <div v-if="open" class="dialog-overlay" @click.self="handleClose">
    <div class="dialog">
      <div class="dialog-header">
        <h2>Add item reference</h2>
        <button type="button" class="close-btn" title="Close" @click="handleClose">&times;</button>
      </div>

      <div class="dialog-body">
        <div class="form-section">
          <h3>This EHR's compositions</h3>
          <div v-if="availableCompositions.length === 0" class="empty-hint">
            No compositions found for this EHR yet.
          </div>
          <div v-else class="composition-list">
            <button
              v-for="composition in availableCompositions"
              :key="composition.uid"
              type="button"
              class="composition-row"
              :title="composition.uid"
              @click="addComposition(composition)"
            >
              <span class="composition-row-text">
                <span class="composition-row-label">{{ composition.label }}</span>
                <span class="composition-row-uid mono">{{ composition.uid }}</span>
              </span>
              <span class="composition-row-add" aria-hidden="true">+</span>
            </button>
          </div>
        </div>

        <div class="form-section">
          <h3>Or enter a reference manually</h3>
          <div class="form-row">
            <div class="form-field">
              <label for="add-item-namespace">Namespace</label>
              <input
                id="add-item-namespace"
                v-model="manualNamespace"
                type="text"
                class="input"
                placeholder="local"
                autocapitalize="off"
                autocorrect="off"
                spellcheck="false"
              />
            </div>
            <div class="form-field">
              <label for="add-item-type">Type</label>
              <input
                id="add-item-type"
                v-model="manualType"
                type="text"
                class="input"
                placeholder="COMPOSITION"
                autocapitalize="off"
                autocorrect="off"
                spellcheck="false"
              />
            </div>
          </div>
          <div class="form-row">
            <div class="form-field">
              <label for="add-item-id-scheme">Id scheme</label>
              <input
                id="add-item-id-scheme"
                v-model="manualIdScheme"
                type="text"
                class="input"
                placeholder="HIER_OBJECT_ID"
                autocapitalize="off"
                autocorrect="off"
                spellcheck="false"
              />
            </div>
            <div class="form-field">
              <label for="add-item-id-value">Id value</label>
              <input
                id="add-item-id-value"
                v-model="manualIdValue"
                type="text"
                class="input mono"
                placeholder="id value"
                autocapitalize="off"
                autocorrect="off"
                spellcheck="false"
                @keydown.enter="addManualReference"
              />
            </div>
          </div>
          <div v-if="manualError" class="error-banner">{{ manualError }}</div>
        </div>
      </div>

      <div class="dialog-actions">
        <button type="button" class="btn btn-secondary" @click="handleClose">Cancel</button>
        <button type="button" class="btn btn-primary" @click="addManualReference">
          + Add reference
        </button>
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
.form-section:last-child {
  margin-bottom: 0;
}

.form-section h3 {
  font-size: 14px;
  font-weight: 600;
  margin: 0 0 12px 0;
  color: var(--color-text-secondary);
}

.empty-hint {
  font-size: 12px;
  color: var(--color-text-muted);
  font-style: italic;
}

.composition-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 220px;
  overflow-y: auto;
}

.composition-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-surface);
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
  transition: all 0.15s;
}
.composition-row:hover {
  border-color: var(--color-primary-dim);
  background: var(--color-surface-hover);
}

.composition-row-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.composition-row-label {
  font-weight: 500;
  font-size: 13px;
}

.composition-row-uid.mono {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--color-text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.composition-row-add {
  flex-shrink: 0;
  font-size: 16px;
  line-height: 1;
  color: var(--color-primary);
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

.input.mono {
  font-family: var(--font-mono);
}

.error-banner {
  margin-top: 12px;
  padding: 12px;
  background: rgba(255, 90, 90, 0.1);
  border: 1px solid rgba(255, 90, 90, 0.3);
  border-radius: var(--radius);
  color: var(--color-error);
  font-size: 13px;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 16px 24px;
  border-top: 1px solid var(--color-border);
}
</style>
