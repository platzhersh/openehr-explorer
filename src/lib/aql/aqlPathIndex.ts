import { normalizeWebTemplate } from "../webtemplate";

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
  nodeId?: string;
  rmType?: string;
  aqlPath?: string;
  children?: WebTemplateNode[];
}

// Better returns snake_case keys (`aql_path`, `node_id`, `rm_type`).
function templateTree(webTemplate: Record<string, unknown>): WebTemplateNode | undefined {
  return normalizeWebTemplate(webTemplate)?.tree as WebTemplateNode | undefined;
}

/**
 * Extract an AQL path index from a Web Template JSON.
 * Recursively walks the tree and collects entries that have an `aqlPath` property.
 * Paths are rooted at the composition, e.g. `/content[...]/data[at0001]`.
 */
export function extractAqlPathIndex(webTemplate: Record<string, unknown>): AqlPathEntry[] {
  const tree = templateTree(webTemplate);
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
 * For every archetype in the template, the paths below it written relative to
 * its root: what follows `alias/` for an alias bound to that archetype. An
 * archetype used in several places contributes each distinct path once.
 */
export function buildArchetypePathMap(
  webTemplate: Record<string, unknown>,
): Map<string, AqlPathEntry[]> {
  const map = new Map<string, AqlPathEntry[]>();
  const tree = templateTree(webTemplate);
  if (tree) collectArchetypes(tree, map);
  return map;
}

function collectArchetypes(node: WebTemplateNode, map: Map<string, AqlPathEntry[]>): void {
  // `nodeId` holds the archetype ID on archetype roots (an at-code elsewhere).
  const archetypeId = node.nodeId;
  if (archetypeId?.startsWith("openEHR-")) {
    const entries = map.get(archetypeId) ?? [];
    const seen = new Set(entries.map((e) => e.aqlPath));
    for (const entry of relativePaths(node)) {
      if (!seen.has(entry.aqlPath)) {
        seen.add(entry.aqlPath);
        entries.push(entry);
      }
    }
    if (entries.length > 0) map.set(archetypeId, entries);
  }
  for (const child of node.children ?? []) collectArchetypes(child, map);
}

function relativePaths(root: WebTemplateNode): AqlPathEntry[] {
  const prefix = `${root.aqlPath ?? ""}/`;
  const entries: AqlPathEntry[] = [];
  for (const child of root.children ?? []) collectAqlPaths(child, entries);
  // RM attributes the template lists beside the archetype can lie outside it.
  return entries
    .filter((e) => e.aqlPath.startsWith(prefix))
    .map((e) => ({ ...e, aqlPath: e.aqlPath.slice(prefix.length) }));
}

/**
 * The AQL path of the template node a FLAT path writes to, or `null` when the
 * template has no such node (e.g. `ctx/` keys). Occurrence indexes (`:0`) and
 * attribute suffixes (`|magnitude`) are ignored, so the result addresses every
 * occurrence of the node's value.
 */
export function aqlPathForFlatPath(
  webTemplate: Record<string, unknown>,
  flatPath: string,
): string | null {
  return findByFlatPath(templateTree(webTemplate), flatPath);
}

/** `aqlPathForFlatPath` for many paths, normalizing the template once. */
export function aqlPathsForFlatPaths(
  webTemplate: Record<string, unknown>,
  flatPaths: string[],
): Record<string, string> {
  const tree = templateTree(webTemplate);
  const result: Record<string, string> = {};
  for (const flatPath of flatPaths) {
    const aqlPath = findByFlatPath(tree, flatPath);
    if (aqlPath) result[flatPath] = aqlPath;
  }
  return result;
}

function findByFlatPath(tree: WebTemplateNode | undefined, flatPath: string): string | null {
  const [rootId, ...segments] = flatPath
    .split("|")[0]
    .split("/")
    .map((segment) => segment.replace(/:\d+$/, ""));
  if (!tree || tree.id !== rootId) return null;

  let node: WebTemplateNode = tree;
  for (const segment of segments) {
    const child: WebTemplateNode | undefined = node.children?.find((c) => c.id === segment);
    if (!child) return null;
    node = child;
  }
  return node.aqlPath || null;
}
