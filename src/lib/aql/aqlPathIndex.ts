export interface AqlPathEntry {
  aqlPath: string;
  label: string;
  rmType: string;
  localizedName?: string;
}

interface WebTemplateNode {
  id?: string;
  name?: string;
  localizedName?: string;
  rmType?: string;
  aqlPath?: string;
  children?: WebTemplateNode[];
}

/**
 * Extract an AQL path index from a Web Template JSON.
 * Recursively walks the tree and collects entries that have an `aqlPath` property.
 */
export function extractAqlPathIndex(webTemplate: Record<string, unknown>): AqlPathEntry[] {
  const tree = webTemplate.tree as WebTemplateNode | undefined;
  if (!tree) return [];

  const entries: AqlPathEntry[] = [];
  collectAqlPaths(tree, entries);
  return entries;
}

function collectAqlPaths(node: WebTemplateNode, entries: AqlPathEntry[]): void {
  if (node.aqlPath && node.name) {
    entries.push({
      aqlPath: node.aqlPath,
      label: node.localizedName || node.name,
      rmType: node.rmType || "unknown",
      localizedName: node.localizedName,
    });
  }

  if (node.children) {
    for (const child of node.children) {
      collectAqlPaths(child, entries);
    }
  }
}

/**
 * Given an archetype ID (e.g., "openEHR-EHR-OBSERVATION.blood_pressure.v2"),
 * find the subtree node that matches and return only paths under that node.
 */
export function extractAqlPathsForArchetype(
  webTemplate: Record<string, unknown>,
  archetypeId: string,
): AqlPathEntry[] {
  const tree = webTemplate.tree as WebTemplateNode | undefined;
  if (!tree) return [];

  const subtree = findNodeByArchetypeId(tree, archetypeId);
  if (!subtree) return [];

  const entries: AqlPathEntry[] = [];
  collectAqlPaths(subtree, entries);
  return entries;
}

function findNodeByArchetypeId(node: WebTemplateNode, archetypeId: string): WebTemplateNode | null {
  // Check the node's id field — in Web Templates, archetype nodes have their archetype ID as the id
  if (node.id === archetypeId) {
    return node;
  }

  if (node.children) {
    for (const child of node.children) {
      const found = findNodeByArchetypeId(child, archetypeId);
      if (found) return found;
    }
  }

  return null;
}
