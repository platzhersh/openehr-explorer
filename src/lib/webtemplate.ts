/**
 * Resolve a human-readable label from a Web Template for a given archetype node ID.
 * Falls back to the provided default name if no match is found.
 */
export function resolveNodeLabel(
  webTemplate: Record<string, unknown>,
  archetypeNodeId: string | null,
  defaultName: string
): string {
  if (!archetypeNodeId || !webTemplate) return defaultName;

  const tree = webTemplate.tree as Record<string, unknown> | undefined;
  if (!tree) return defaultName;

  const found = findNodeById(tree, archetypeNodeId);
  return found ?? defaultName;
}

function findNodeById(
  node: Record<string, unknown>,
  targetId: string
): string | null {
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
 * Extract all FLAT paths from a Web Template tree.
 */
export function extractFlatPaths(
  webTemplate: Record<string, unknown>
): string[] {
  const tree = webTemplate.tree as Record<string, unknown> | undefined;
  if (!tree) return [];

  const paths: string[] = [];
  collectPaths(tree, "", paths);
  return paths.sort();
}

function collectPaths(
  node: Record<string, unknown>,
  parentPath: string,
  paths: string[]
): void {
  const id = node.id as string | undefined;
  const currentPath = parentPath ? `${parentPath}/${id ?? ""}` : id ?? "";

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
