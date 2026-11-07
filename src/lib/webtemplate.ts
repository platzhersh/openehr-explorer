/**
 * Resolve a human-readable label from a Web Template for a given archetype node ID.
 * Falls back to the provided default name if no match is found.
 */
export function resolveNodeLabel(
  webTemplate: Record<string, unknown>,
  archetypeNodeId: string | null,
  defaultName: string,
): string {
  if (!archetypeNodeId || !webTemplate) return defaultName;

  const tree = webTemplate.tree as Record<string, unknown> | undefined;
  if (!tree) return defaultName;

  const found = findNodeById(tree, archetypeNodeId);
  return found ?? defaultName;
}

function findNodeById(node: Record<string, unknown>, targetId: string): string | null {
  const nodeId = node.id as string | undefined;
  const nodeArchetypeId = node.node_id as string | undefined;

  if (nodeId === targetId || nodeArchetypeId === targetId) {
    return (node.name as string) ?? (node.localizedName as string) ?? null;
  }

  const children = node.children as Record<string, unknown>[] | undefined;
  if (children) {
    for (const child of children) {
      const found = findNodeById(child, targetId);
      if (found) return found;
    }
  }

  return null;
}

/**
 * Normalize a Web Template for medblocks-ui compatibility.
 * Ensures all nodes in the tree have an 'inputs' property (defaults to empty array).
 * This fixes the "undefined is not an object (evaluating 'n6.inputs.forEach')" error.
 */
export function normalizeWebTemplate(
  webTemplate: Record<string, unknown>,
): Record<string, unknown> {
  if (!webTemplate) return webTemplate;

  const normalized = camelCaseKeys(webTemplate) as Record<string, unknown>;
  const tree = normalized.tree as Record<string, unknown> | undefined;

  if (tree) {
    normalized.tree = normalizeNode(tree);
  }

  return normalized;
}

function humanizeId(id: unknown): string {
  const text = typeof id === "string" ? id.replace(/_/g, " ").trim() : "";
  return text ? text.charAt(0).toUpperCase() + text.slice(1) : "";
}

// Keys whose *values* are user-defined dictionaries (language codes,
// terminology ids, annotation names): the key itself is camelCased but the
// entries inside are left alone.
const OPAQUE_DICTIONARIES = new Set([
  "localizedNames",
  "localizedDescriptions",
  "termBindings",
  "annotations",
]);

function camelCaseKey(key: string): string {
  // Only plain snake_case identifiers; leave `|`-suffixes, ids, "en_US" etc. alone.
  if (!/^[a-z][a-z0-9]*(_[a-z0-9]+)+$/.test(key)) return key;
  return key.replace(/_([a-z0-9])/g, (_, c: string) => c.toUpperCase());
}

// Better's Web Template uses snake_case keys (`rm_type`, `in_context`,
// `node_id`, `min_op`, ...) where EHRBase uses camelCase, which is what
// medblocks-ui reads. Without this, medblocks sees no `rmType` and renders no
// fields for the node.
function camelCaseKeys(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(camelCaseKeys);
  if (value && typeof value === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
      const key = camelCaseKey(k);
      out[key] = OPAQUE_DICTIONARIES.has(key) ? v : camelCaseKeys(v);
    }
    return out;
  }
  return value;
}

function normalizeNode(rawNode: Record<string, unknown>): Record<string, unknown> {
  const normalized = { ...rawNode };

  // Ensure 'inputs' property exists (medblocks-ui expects this on all nodes)
  if (!normalized.inputs) {
    normalized.inputs = [];
  }

  // Better's Web Templates omit `name` on structural nodes such as `context`
  // and `category`; medblocks-ui renders that as the literal text "undefined".
  if (!normalized.name) {
    normalized.name = (normalized.localizedName as string | undefined) || humanizeId(normalized.id);
  }

  // Recursively normalize children
  const children = normalized.children as Record<string, unknown>[] | undefined;
  if (children && Array.isArray(children)) {
    normalized.children = children.map((child) => normalizeNode(child));
  }

  return normalized;
}

/**
 * Classify a DV_CODED_TEXT Web Template node as local or external terminology.
 * Returns:
 * - 'local' if the node has a pre-populated list of values (archetype-internal codes)
 * - A terminology name (e.g. 'SNOMED-CT', 'LOINC') if determinable from the template
 * - 'external' if external but the specific terminology is unknown
 * - null if the node is not a coded text node or cannot be classified
 */
export function classifyCodedTextNode(node: Record<string, unknown>): string | null {
  const rmType = node.rmType as string | undefined;
  if (rmType !== "DV_CODED_TEXT" && rmType !== "DV_TEXT") return null;

  const inputs = node.inputs as Array<Record<string, unknown>> | undefined;
  if (!inputs || inputs.length === 0) return null;

  const codeInput = inputs.find((i) => i.suffix === "code" || i.suffix === undefined);
  if (!codeInput) return null;

  const list = codeInput.list as Array<unknown> | undefined;
  if (list && list.length > 0) return "local";

  // Check for terminology suffix with defaultValue indicating the external system
  const termInput = inputs.find((i) => i.suffix === "terminology");
  if (termInput?.defaultValue) {
    return String(termInput.defaultValue);
  }

  return "external";
}

/**
 * Extract all FLAT paths from a Web Template tree.
 */
export function extractFlatPaths(webTemplate: Record<string, unknown>): string[] {
  const tree = webTemplate.tree as Record<string, unknown> | undefined;
  if (!tree) return [];

  const paths: string[] = [];
  collectPaths(tree, "", paths);
  return paths.sort();
}

function collectPaths(node: Record<string, unknown>, parentPath: string, paths: string[]): void {
  const id = node.id as string | undefined;
  const currentPath = parentPath ? `${parentPath}/${id ?? ""}` : (id ?? "");

  const children = node.children as Record<string, unknown>[] | undefined;
  if (children && children.length > 0) {
    for (const child of children) {
      collectPaths(child, currentPath, paths);
    }
  } else {
    if (currentPath) {
      paths.push(currentPath);
    }
  }
}
