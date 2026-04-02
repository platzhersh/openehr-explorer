<script setup lang="ts">
import { computed } from "vue";
import { resolveNodeLabel } from "../lib/webtemplate";

interface Props {
  data: Record<string, unknown>;
  webTemplate: Record<string, unknown> | null;
  highlightedPath?: string | null;
  depth?: number;
}

const props = withDefaults(defineProps<Props>(), {
  highlightedPath: null,
  depth: 0,
});

interface TreeNode {
  key: string;
  label: string;
  archetypeId: string | null;
  rmType: string | null;
  value: unknown;
  children: TreeNode[];
  path: string;
}

function buildTree(
  data: unknown,
  parentPath: string = "",
  key: string = "root"
): TreeNode[] {
  if (data === null || data === undefined) return [];

  if (Array.isArray(data)) {
    return data.flatMap((item, i) =>
      buildTree(item, `${parentPath}[${i}]`, `${key}[${i}]`)
    );
  }

  if (typeof data === "object") {
    const obj = data as Record<string, unknown>;
    const name =
      (obj.name as any)?.value ?? (obj.name as string) ?? key;
    const archetypeId =
      (obj.archetype_node_id as string) ??
      (obj.archetype_details as any)?.archetype_id?.value ??
      null;
    const rmType = (obj._type as string) ?? null;

    // Resolve label from web template if available
    const label = props.webTemplate
      ? resolveNodeLabel(props.webTemplate, archetypeId, name)
      : name;

    const children: TreeNode[] = [];
    const valueFields: Record<string, unknown> = {};

    for (const [k, v] of Object.entries(obj)) {
      if (
        ["name", "_type", "archetype_node_id", "archetype_details", "uid"].includes(k)
      ) {
        continue;
      }

      if (v !== null && typeof v === "object" && !Array.isArray(v)) {
        const childObj = v as Record<string, unknown>;
        if (childObj._type || childObj.archetype_node_id) {
          children.push(...buildTree(v, `${parentPath}/${k}`, k));
          continue;
        }
      }

      if (Array.isArray(v) && v.length > 0 && typeof v[0] === "object") {
        children.push(...buildTree(v, `${parentPath}/${k}`, k));
        continue;
      }

      // Leaf value
      if (v !== null && v !== undefined) {
        valueFields[k] = v;
      }
    }

    const displayValue = formatValue(valueFields);

    return [
      {
        key,
        label: String(label),
        archetypeId,
        rmType,
        value: displayValue,
        children,
        path: parentPath || "/",
      },
    ];
  }

  return [
    {
      key,
      label: key,
      archetypeId: null,
      rmType: null,
      value: data,
      children: [],
      path: parentPath,
    },
  ];
}

function formatValue(fields: Record<string, unknown>): string | null {
  if (Object.keys(fields).length === 0) return null;

  // Common openEHR value patterns
  if ("magnitude" in fields && "units" in fields) {
    return `${fields.magnitude} ${fields.units}`;
  }
  if ("value" in fields && Object.keys(fields).length <= 2) {
    return String(fields.value);
  }
  if ("numerator" in fields && "denominator" in fields) {
    return `${fields.numerator}/${fields.denominator}`;
  }

  const parts = Object.entries(fields)
    .filter(([k]) => !k.startsWith("_"))
    .map(([k, v]) => `${k}: ${v}`);

  return parts.length > 0 ? parts.join(", ") : null;
}

const tree = computed(() => buildTree(props.data));
</script>

<template>
  <div class="composition-tree">
    <div v-for="node in tree" :key="node.path" class="tree-root">
      <TreeNodeComponent
        :node="node"
        :depth="0"
        :highlighted-path="highlightedPath"
      />
    </div>
    <div v-if="tree.length === 0" class="empty-state">
      <p>No data to display.</p>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, h, ref as vueRef, type PropType, type VNode } from "vue";

interface TreeNodeType {
  key: string;
  label: string;
  archetypeId: string | null;
  rmType: string | null;
  value: unknown;
  children: TreeNodeType[];
  path: string;
}

const TreeNodeComponent: ReturnType<typeof defineComponent> = defineComponent({
  name: "TreeNodeComponent",
  props: {
    node: { type: Object as PropType<TreeNodeType>, required: true },
    depth: { type: Number, default: 0 },
    highlightedPath: { type: String, default: null },
  },
  setup(props): () => VNode {
    const collapsed = vueRef(props.depth > 3);

    const toggle = () => {
      collapsed.value = !collapsed.value;
    };

    return (): VNode => {
      const node = props.node;
      const hasChildren = node.children.length > 0;

      const elements = [];

      // Node header
      const headerChildren = [];

      if (hasChildren) {
        headerChildren.push(
          h(
            "span",
            { class: "toggle", onClick: toggle },
            collapsed.value ? "\u25B6" : "\u25BC"
          )
        );
      } else {
        headerChildren.push(h("span", { class: "toggle-spacer" }));
      }

      headerChildren.push(h("span", { class: "node-label" }, node.label));

      if (node.rmType) {
        headerChildren.push(
          h("span", { class: "badge rm-type" }, node.rmType)
        );
      }

      if (node.archetypeId) {
        headerChildren.push(
          h("span", { class: "badge archetype-id" }, node.archetypeId)
        );
      }

      if (node.value !== null && node.value !== undefined) {
        headerChildren.push(
          h("span", { class: "node-value" }, String(node.value))
        );
      }

      elements.push(
        h(
          "div",
          {
            class: [
              "tree-node-header",
              { highlighted: props.highlightedPath === node.path },
            ],
            style: { paddingLeft: `${props.depth * 20}px` },
          },
          headerChildren
        )
      );

      // Children
      if (hasChildren && !collapsed.value) {
        elements.push(
          ...node.children.map((child) =>
            h(TreeNodeComponent, {
              node: child,
              depth: props.depth + 1,
              highlightedPath: props.highlightedPath,
            })
          )
        );
      }

      return h("div", { class: "tree-node" }, elements);
    };
  },
});
</script>

<style scoped>
.composition-tree {
  font-size: 13px;
}

:deep(.tree-node-header) {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  border-radius: 3px;
  cursor: default;
  transition: background 0.1s;
}
:deep(.tree-node-header:hover) {
  background: var(--color-surface);
}
:deep(.tree-node-header.highlighted) {
  background: rgba(100, 255, 218, 0.1);
  border-left: 2px solid var(--color-primary);
}

:deep(.toggle) {
  width: 16px;
  text-align: center;
  cursor: pointer;
  font-size: 10px;
  color: var(--color-text-muted);
  user-select: none;
}
:deep(.toggle-spacer) {
  width: 16px;
}

:deep(.node-label) {
  font-weight: 500;
}

:deep(.rm-type) {
  font-size: 10px;
  opacity: 0.6;
}
:deep(.archetype-id) {
  font-size: 10px;
  font-family: var(--font-mono);
  opacity: 0.4;
}

:deep(.node-value) {
  margin-left: 8px;
  color: var(--color-primary);
  font-family: var(--font-mono);
  font-size: 12px;
}
</style>
