<script setup lang="ts">
import { ref, computed } from "vue";

// The openEHR DIRECTORY resource is a FOLDER tree: each FOLDER has a name,
// optional nested `folders`, and `items` (OBJECT_REFs pointing at versioned
// objects — almost always COMPOSITIONs). Depth and shape are data-driven, so
// (like CompositionTree's RM walking) this stays loosely typed rather than
// mirroring a full Rust struct.
interface ObjectRef {
  id?: { value?: string };
  namespace?: string;
  type?: string;
}

interface FolderNode {
  name?: { value?: string };
  uid?: { value?: string };
  items?: ObjectRef[];
  folders?: FolderNode[];
}

const props = defineProps<{
  // Raw JSON from the backend (see `useEhrStore().directory`) — loosely
  // typed like `CompositionTree`'s `data` prop, then narrowed below.
  folder: Record<string, unknown>;
  depth: number;
}>();

const emit = defineEmits<{
  (e: "open-item", objectRef: ObjectRef): void;
}>();

const folder = computed(() => props.folder as FolderNode);

const expanded = ref(props.depth < 2);

const label = computed(() => folder.value.name?.value ?? "(unnamed folder)");
const items = computed(() => folder.value.items ?? []);
const subfolders = computed(() => folder.value.folders ?? []);
const childCount = computed(() => items.value.length + subfolders.value.length);

function toggle() {
  expanded.value = !expanded.value;
}

function itemLabel(objectRef: ObjectRef): string {
  const id = objectRef.id?.value;
  if (!id) return "(unknown)";
  return id.length > 12 ? `${id.substring(0, 8)}...` : id;
}

function isCompositionRef(objectRef: ObjectRef): boolean {
  return objectRef.type === "COMPOSITION";
}

// Recursion re-enters through the same loosely-typed `Record<string, unknown>`
// prop (see the comment on `folder` above), so each subfolder is cast back
// down before being passed to the next level.
function asRecord(sub: FolderNode): Record<string, unknown> {
  return sub as unknown as Record<string, unknown>;
}

function onItemClick(objectRef: ObjectRef) {
  if (isCompositionRef(objectRef)) {
    emit("open-item", objectRef);
  }
}
</script>

<template>
  <div class="dir-node">
    <button type="button" class="dir-row" @click="toggle">
      <span class="toggle">{{ expanded ? "▼" : "▶" }}</span>
      <span class="folder-icon">📁</span>
      <span class="folder-label">{{ label }}</span>
      <span class="folder-meta">{{ childCount }}</span>
    </button>

    <div v-if="expanded" class="dir-children">
      <DirectoryTree
        v-for="(sub, i) in subfolders"
        :key="sub.uid?.value ?? i"
        :folder="asRecord(sub)"
        :depth="depth + 1"
        @open-item="(objectRef) => emit('open-item', objectRef)"
      />
      <component
        :is="isCompositionRef(item) ? 'button' : 'div'"
        v-for="(item, i) in items"
        :key="item.id?.value ?? i"
        v-bind="isCompositionRef(item) ? { type: 'button' } : {}"
        class="dir-item"
        :class="{ clickable: isCompositionRef(item) }"
        :title="isCompositionRef(item) ? 'Open composition' : undefined"
        @click.stop="onItemClick(item)"
      >
        <span class="item-icon">📄</span>
        <span class="item-type">{{ item.type ?? "OBJECT_REF" }}</span>
        <span class="item-id mono">{{ itemLabel(item) }}</span>
      </component>
      <div v-if="childCount === 0" class="dir-empty">Empty folder</div>
    </div>
  </div>
</template>

<style scoped>
.dir-node {
  font-size: 13px;
}

.dir-row {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  padding: 4px 6px;
  border: none;
  border-radius: var(--radius);
  background: none;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.dir-row:hover {
  background: var(--color-surface);
}

.toggle {
  width: 12px;
  font-size: 10px;
  color: var(--color-text-muted);
  flex-shrink: 0;
  text-align: center;
}

.folder-icon {
  flex-shrink: 0;
}

.folder-label {
  font-weight: 500;
}

.folder-meta {
  font-size: 11px;
  color: var(--color-text-muted);
}

.dir-children {
  margin-left: 16px;
  padding-left: 8px;
  border-left: 1px solid var(--color-border);
}

.dir-item {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  padding: 3px 6px;
  border: none;
  background: none;
  font: inherit;
  text-align: left;
  color: var(--color-text-secondary);
}
.dir-item.clickable {
  cursor: pointer;
}
.dir-item.clickable:hover {
  background: var(--color-surface);
  color: var(--color-text);
  border-radius: var(--radius);
}

.item-icon {
  flex-shrink: 0;
}

.item-type {
  font-size: 11px;
  color: var(--color-text-muted);
}

.item-id.mono {
  font-family: var(--font-mono);
  font-size: 12px;
}

.dir-empty {
  padding: 3px 6px;
  font-size: 12px;
  color: var(--color-text-muted);
  font-style: italic;
}
</style>
