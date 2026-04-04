<script setup lang="ts">
import { ref, computed } from "vue";

const props = defineProps<{
  label: string;
  value: unknown;
  depth: number;
  searchTerm?: string;
}>();

const expanded = ref(props.depth < 3);

const isObject = computed(
  () => props.value !== null && typeof props.value === "object" && !Array.isArray(props.value),
);
const isArray = computed(() => Array.isArray(props.value));
const isExpandable = computed(() => isObject.value || isArray.value);

const childEntries = computed(() => {
  if (isArray.value) {
    return (props.value as unknown[]).map((v, i) => ({ key: `[${i}]`, value: v }));
  }
  if (isObject.value) {
    return Object.entries(props.value as Record<string, unknown>).map(([k, v]) => ({
      key: k,
      value: v,
    }));
  }
  return [];
});

const childCount = computed(() => childEntries.value.length);

const valueType = computed(() => {
  if (props.value === null) return "null";
  if (typeof props.value === "boolean") return "boolean";
  if (typeof props.value === "number") return "number";
  if (typeof props.value === "string") return "string";
  return "other";
});

const displayValue = computed(() => {
  if (props.value === null) return "null";
  if (typeof props.value === "string") return `"${props.value}"`;
  return String(props.value);
});

// openEHR-aware rendering
const isOpenEhrType = computed(() => props.label === "_type" && typeof props.value === "string");
const isArchetypeNodeId = computed(
  () => props.label === "archetype_node_id" && typeof props.value === "string",
);
const isCopyableId = computed(
  () =>
    (props.label === "uid" || props.label === "object_id") && typeof props.value === "string",
);

// DV_QUANTITY enrichment: check if parent object has _type: DV_QUANTITY
const dvQuantityDisplay = computed(() => {
  if (!isObject.value) return null;
  const obj = props.value as Record<string, unknown>;
  if (obj._type !== "DV_QUANTITY") return null;
  const magnitude = obj.magnitude;
  const units = obj.units;
  if (magnitude !== undefined && units) return `${magnitude} ${units}`;
  return null;
});

const matchesSearch = computed(() => {
  if (!props.searchTerm) return false;
  const term = props.searchTerm.toLowerCase();
  if (props.label.toLowerCase().includes(term)) return true;
  if (!isExpandable.value && displayValue.value.toLowerCase().includes(term)) return true;
  return false;
});

function toggle() {
  expanded.value = !expanded.value;
}

async function copyValue(text: string) {
  await navigator.clipboard.writeText(text);
}
</script>

<template>
  <div class="tree-node" :class="{ 'search-match': matchesSearch }">
    <div class="node-row" @click="isExpandable ? toggle() : undefined">
      <span v-if="isExpandable" class="toggle">{{ expanded ? "\u25BC" : "\u25B6" }}</span>
      <span v-else class="toggle-placeholder" />

      <span class="node-label">{{ label }}</span>

      <template v-if="isExpandable">
        <span class="node-meta">
          {{ isArray ? `Array(${childCount})` : `{${childCount}}` }}
        </span>
        <span v-if="dvQuantityDisplay" class="dv-quantity-badge">{{ dvQuantityDisplay }}</span>
      </template>

      <template v-else>
        <span class="node-sep">: </span>
        <span v-if="isOpenEhrType" class="openehr-type-pill">{{ value }}</span>
        <span v-else-if="isArchetypeNodeId" class="archetype-id">{{ value }}</span>
        <span v-else :class="['node-value', `value-${valueType}`]">{{ displayValue }}</span>
        <button
          v-if="isCopyableId"
          class="copy-btn"
          title="Copy"
          @click.stop="copyValue(String(value))"
        >
          copy
        </button>
      </template>
    </div>

    <div v-if="isExpandable && expanded" class="node-children">
      <JsonTreeNode
        v-for="child in childEntries"
        :key="child.key"
        :label="child.key"
        :value="child.value"
        :depth="depth + 1"
        :search-term="searchTerm"
      />
    </div>
  </div>
</template>

<style scoped>
.tree-node {
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.6;
}

.node-row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 1px 4px;
  border-radius: 3px;
  cursor: default;
  white-space: nowrap;
}

.node-row:hover {
  background: var(--color-surface-hover);
}

.toggle {
  width: 14px;
  font-size: 10px;
  cursor: pointer;
  color: var(--color-text-muted);
  flex-shrink: 0;
  text-align: center;
}

.toggle-placeholder {
  width: 14px;
  flex-shrink: 0;
}

.node-label {
  color: var(--color-primary);
  flex-shrink: 0;
}

.node-sep {
  color: var(--color-text-muted);
}

.node-meta {
  color: var(--color-text-muted);
  font-size: 11px;
}

.node-value {
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 500px;
}

.value-string {
  color: #a8cc8c;
}
.value-number {
  color: #6cb6ff;
}
.value-boolean {
  color: #dbab79;
}
.value-null {
  color: #5a6a8a;
  font-style: italic;
}

.openehr-type-pill {
  display: inline-block;
  padding: 0 6px;
  border-radius: 8px;
  background: rgba(255, 134, 28, 0.15);
  color: #ff861c;
  font-size: 11px;
  font-weight: 500;
}

.archetype-id {
  color: #dbab79;
  font-style: italic;
}

.dv-quantity-badge {
  display: inline-block;
  padding: 0 6px;
  border-radius: 8px;
  background: rgba(100, 255, 218, 0.1);
  color: var(--color-primary);
  font-size: 11px;
  margin-left: 4px;
}

.node-children {
  margin-left: 18px;
  border-left: 1px solid var(--color-border);
  padding-left: 4px;
}

.search-match > .node-row {
  background: rgba(255, 217, 61, 0.15);
}

.copy-btn {
  background: none;
  border: none;
  color: var(--color-text-muted);
  cursor: pointer;
  padding: 0 4px;
  font-size: 10px;
  border-radius: 3px;
  margin-left: 4px;
}
.copy-btn:hover {
  color: var(--color-primary);
  background: var(--color-surface);
}
</style>
