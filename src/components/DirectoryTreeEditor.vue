<script setup lang="ts">
import { ref } from "vue";
import type { CompositionOption, EditableFolder } from "../lib/directoryEdit";
import { addItem, addSubfolder, removeItem, removeSubfolder } from "../lib/directoryEdit";

// Editable counterpart to DirectoryTree.vue — same FOLDER/OBJECT_REF
// recursion, but bound to plain-field inputs instead of rendering the RM
// JSON read-only. See `src/lib/directoryEdit.ts` for the editable model and
// the conversion to/from the wire FOLDER shape.
const props = defineProps<{
  folder: EditableFolder;
  depth: number;
  /** The root folder can't remove itself — the "remove" button is hidden
   *  for it, and it's the only node that gets archetype_details on save. */
  isRoot?: boolean;
  availableCompositions: CompositionOption[];
}>();

defineEmits<{
  (e: "remove"): void;
}>();

const expanded = ref(true);
const selectedCompositionUid = ref("");

function toggle() {
  expanded.value = !expanded.value;
}

function handleAddSubfolder() {
  addSubfolder(props.folder);
  expanded.value = true;
}

function handleAddItem() {
  if (!selectedCompositionUid.value) return;
  addItem(props.folder, selectedCompositionUid.value);
  selectedCompositionUid.value = "";
}

function handleRemoveItem(key: string) {
  removeItem(props.folder, key);
}

function handleRemoveSubfolder(key: string) {
  removeSubfolder(props.folder, key);
}

function compositionLabel(uid: string): string {
  return props.availableCompositions.find((c) => c.uid === uid)?.label ?? uid;
}
</script>

<template>
  <div class="dir-edit-node">
    <div class="dir-edit-row">
      <button type="button" class="toggle" @click="toggle">{{ expanded ? "▼" : "▶" }}</button>
      <span class="folder-icon">📁</span>
      <input
        v-model="folder.name"
        type="text"
        class="folder-name-input"
        placeholder="Folder name"
        :aria-label="`Folder name (depth ${depth})`"
      />
      <button
        v-if="!isRoot"
        type="button"
        class="btn-icon-remove"
        title="Remove this folder"
        @click="$emit('remove')"
      >
        ✕
      </button>
    </div>

    <div v-if="expanded" class="dir-edit-children">
      <DirectoryTreeEditor
        v-for="sub in folder.folders"
        :key="sub.key"
        :folder="sub"
        :depth="depth + 1"
        :available-compositions="availableCompositions"
        @remove="handleRemoveSubfolder(sub.key)"
      />

      <div v-for="item in folder.items" :key="item.key" class="dir-edit-item">
        <span class="item-icon">📄</span>
        <span class="item-type">{{ item.type }}</span>
        <input
          v-model="item.id"
          type="text"
          class="item-id-input mono"
          placeholder="Composition UID"
          :title="compositionLabel(item.id)"
        />
        <button
          type="button"
          class="btn-icon-remove"
          title="Remove item"
          @click="handleRemoveItem(item.key)"
        >
          ✕
        </button>
      </div>

      <div class="dir-edit-actions">
        <button type="button" class="btn btn-sm" @click="handleAddSubfolder">+ Subfolder</button>
        <select v-model="selectedCompositionUid" class="composition-picker">
          <option value="">Add composition…</option>
          <option v-for="c in availableCompositions" :key="c.uid" :value="c.uid">
            {{ c.label }}
          </option>
        </select>
        <button
          type="button"
          class="btn btn-sm"
          :disabled="!selectedCompositionUid"
          @click="handleAddItem"
        >
          + Item
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dir-edit-node {
  font-size: 13px;
}

.dir-edit-row {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  padding: 4px 6px;
}

.toggle {
  width: 16px;
  height: 20px;
  font-size: 10px;
  color: var(--color-text-muted);
  flex-shrink: 0;
  text-align: center;
  border: none;
  background: none;
  cursor: pointer;
}

.folder-icon {
  flex-shrink: 0;
}

.folder-name-input {
  flex: 1;
  min-width: 0;
  padding: 3px 6px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-bg);
  color: inherit;
  font: inherit;
  font-weight: 500;
}
.folder-name-input:focus {
  outline: none;
  border-color: var(--color-primary);
}

.dir-edit-children {
  margin-left: 16px;
  padding-left: 8px;
  border-left: 1px solid var(--color-border);
}

.dir-edit-item {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  padding: 3px 6px 3px 22px;
}

.item-icon {
  flex-shrink: 0;
}

.item-type {
  font-size: 11px;
  color: var(--color-text-muted);
  flex-shrink: 0;
}

.item-id-input {
  flex: 1;
  min-width: 0;
  padding: 3px 6px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-bg);
  color: inherit;
  font-size: 12px;
}
.item-id-input:focus {
  outline: none;
  border-color: var(--color-primary);
}
.item-id-input.mono {
  font-family: var(--font-mono);
}

.btn-icon-remove {
  flex-shrink: 0;
  width: 20px;
  height: 20px;
  border: none;
  background: none;
  color: var(--color-text-muted);
  cursor: pointer;
  border-radius: var(--radius);
  font-size: 12px;
  line-height: 1;
}
.btn-icon-remove:hover {
  color: var(--color-error);
  background: rgba(255, 90, 90, 0.1);
}

.dir-edit-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px;
  flex-wrap: wrap;
}

.composition-picker {
  flex: 1;
  min-width: 120px;
  padding: 3px 6px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-bg);
  color: inherit;
  font-size: 12px;
}
</style>
