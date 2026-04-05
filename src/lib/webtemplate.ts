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

  const normalized = { ...webTemplate };
  const tree = normalized.tree as Record<string, unknown> | undefined;

  if (tree) {
    normalized.tree = normalizeNode(tree);
  }

  return normalized;
}

function normalizeNode(node: Record<string, unknown>): Record<string, unknown> {
  const normalized = { ...node };

  // Ensure 'inputs' property exists (medblocks-ui expects this on all nodes)
  if (!normalized.inputs) {
    normalized.inputs = [];
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
