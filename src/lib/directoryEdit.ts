import type { InjectionKey } from "vue";

/**
 * Editing model for the DIRECTORY (FOLDER hierarchy) — see PRD/OEH-27.
 *
 * The wire format returned by `get_directory` (and expected by
 * `create_directory`/`update_directory`) is raw openEHR RM JSON: each FOLDER
 * has `_type`, `name: { value }`, `items: OBJECT_REF[]`, and nested
 * `folders`. That shape suits read-only rendering (`DirectoryTree.vue` stays
 * loosely typed against it, matching `CompositionTree`'s RM walking) but is
 * awkward to bind form inputs to directly — inputs want plain strings, not
 * `{ value }` wrappers, and RM housekeeping fields (`_type`,
 * `archetype_details`, ...) have no business appearing in an edit form.
 *
 * `EditableFolder`/`EditableItem` are the plain-field shape the edit UI
 * (`DirectoryTreeEditor.vue`) binds to; `fromWireFolder`/`toWireFolder`
 * convert to and from RM JSON at the edges — loading an existing DIRECTORY
 * into edit mode, and building the request body to send back.
 *
 * `DirectoryTreeEditor.vue` only ever *reads* the `EditableFolder` it's
 * given as a prop — every write goes through `DIRECTORY_MUTATIONS_KEY`
 * (injected from the component that owns the tree, e.g. EhrBrowser.vue's
 * `editableDirectory`), addressed by `path` rather than mutating the prop
 * in place. `getFolderAtPath` resolves a `path` back to the `EditableFolder`
 * the owner should mutate.
 */

/** Default OBJECT_REF type/namespace for a newly added DIRECTORY item —
 *  almost always a COMPOSITION stored alongside the EHR that owns this
 *  DIRECTORY. */
const DEFAULT_ITEM_TYPE = "COMPOSITION";
const DEFAULT_ITEM_NAMESPACE = "local";

export interface EditableItem {
  /** Local editing key — not part of the wire format. Needed for a stable
   *  `:key` in the item list since `id` is itself an editable text field. */
  key: string;
  id: string;
  type: string;
  namespace: string;
}

export interface EditableFolder {
  /** Local editing key — not part of the wire format (see `EditableItem.key`). */
  key: string;
  name: string;
  items: EditableItem[];
  folders: EditableFolder[];
}

/** A composition available to reference from a DIRECTORY item, for the
 *  "add composition" picker in `DirectoryTreeEditor.vue`. */
export interface CompositionOption {
  uid: string;
  label: string;
}

let keyCounter = 0;
function nextKey(prefix: string): string {
  keyCounter += 1;
  return `${prefix}-${keyCounter}`;
}

/** Creates a new, empty folder for "Create Directory" / "+ Subfolder". */
export function emptyFolder(name = ""): EditableFolder {
  return { key: nextKey("folder"), name, items: [], folders: [] };
}

/** Converts a DIRECTORY FOLDER (as returned by `get_directory`) into the
 *  editable shape. Recurses into nested `folders`. Missing/malformed fields
 *  fall back to empty defaults rather than throwing — the server's shape is
 *  data-driven and not something this UI can fully validate up front. */
export function fromWireFolder(folder: Record<string, unknown>): EditableFolder {
  const f = folder as {
    name?: { value?: string };
    items?: Array<{ id?: { value?: string }; type?: string; namespace?: string }>;
    folders?: Record<string, unknown>[];
  };

  return {
    key: nextKey("folder"),
    name: f.name?.value ?? "",
    items: (f.items ?? []).map((item) => ({
      key: nextKey("item"),
      id: item.id?.value ?? "",
      type: item.type ?? DEFAULT_ITEM_TYPE,
      namespace: item.namespace ?? DEFAULT_ITEM_NAMESPACE,
    })),
    folders: (f.folders ?? []).map(fromWireFolder),
  };
}

/** Converts an editable folder tree back into DIRECTORY FOLDER RM JSON.
 *
 * Items with a blank UID are dropped rather than sent — an OBJECT_REF with
 * no `id` isn't valid RM data, and a half-filled "add item" row shouldn't
 * block saving the rest of the tree.
 *
 * Only the root FOLDER gets `archetype_details`: it's the archetype root of
 * the DIRECTORY structure (per the RM's `Archetyped_valid` invariant), the
 * same reasoning `build_ehr_status_json` documents on the Rust side for
 * EHR_STATUS. Nested FOLDERs share that archetype and aren't roots, so
 * stricter CDRs don't require it on them.
 */
const FOLDER_ARCHETYPE_ID = "openEHR-EHR-FOLDER.generic.v1";

export function toWireFolder(folder: EditableFolder, isRoot = true): Record<string, unknown> {
  const wire: Record<string, unknown> = {
    _type: "FOLDER",
    name: { value: folder.name.trim() || "(unnamed folder)" },
    items: folder.items
      .filter((item) => item.id.trim().length > 0)
      .map((item) => ({
        _type: "OBJECT_REF",
        id: { _type: "HIER_OBJECT_ID", value: item.id.trim() },
        namespace: item.namespace.trim() || DEFAULT_ITEM_NAMESPACE,
        type: item.type.trim() || DEFAULT_ITEM_TYPE,
      })),
    folders: folder.folders.map((sub) => toWireFolder(sub, false)),
  };

  if (isRoot) {
    wire.archetype_node_id = FOLDER_ARCHETYPE_ID;
    wire.archetype_details = {
      _type: "ARCHETYPED",
      archetype_id: {
        _type: "ARCHETYPE_ID",
        value: FOLDER_ARCHETYPE_ID,
      },
      rm_version: "1.0.4",
    };
  }

  return wire;
}

export function addSubfolder(folder: EditableFolder, name = "New folder"): void {
  folder.folders.push(emptyFolder(name));
}

export function addItem(folder: EditableFolder, id: string, type = DEFAULT_ITEM_TYPE): void {
  folder.items.push({ key: nextKey("item"), id, type, namespace: DEFAULT_ITEM_NAMESPACE });
}

export function removeSubfolder(folder: EditableFolder, key: string): void {
  folder.folders = folder.folders.filter((sub) => sub.key !== key);
}

export function removeItem(folder: EditableFolder, key: string): void {
  folder.items = folder.items.filter((item) => item.key !== key);
}

/** Resolves `path` (a sequence of `folders[]` indices from the root) to the
 *  `EditableFolder` at that location. `path: []` is the root itself.
 *
 *  This is how `DirectoryTreeEditor.vue` addresses a folder to mutate
 *  without ever writing to its own `folder` prop: it emits `path` up to
 *  whichever component owns the tree (via `DIRECTORY_MUTATIONS_KEY`), which
 *  resolves it back to a real `EditableFolder` and calls the mutators above
 *  on that — a value it owns, not a prop.
 *
 *  Every `path` in practice comes straight from an index this module handed
 *  out while iterating `folders[]` (see `DirectoryTreeEditor.vue`), so it
 *  should always resolve — but indexing `folders[]` directly would silently
 *  hand back `undefined` on a stale path (e.g. a folder removed out from
 *  under an in-flight edit) and defer the failure to whatever the caller
 *  does next. Failing here instead, with the path that didn't resolve,
 *  turns that into a clear error at the point of the mistake. */
export function getFolderAtPath(root: EditableFolder, path: readonly number[]): EditableFolder {
  let current = root;
  for (const index of path) {
    const next: EditableFolder | undefined = current.folders[index];
    if (!next) {
      throw new Error(`getFolderAtPath: no folder at index ${index} in path [${path.join(", ")}]`);
    }
    current = next;
  }
  return current;
}

/** The mutation API a DIRECTORY-tree owner (e.g. `EhrBrowser.vue`'s
 *  `editableDirectory`) provides via Vue's `provide`/`inject` under
 *  `DIRECTORY_MUTATIONS_KEY`. `DirectoryTreeEditor.vue` — at any depth —
 *  injects this instead of mutating the `EditableFolder` it receives as a
 *  prop, so every write lands on the tree the owner actually holds.
 *
 *  `path` addresses the folder the action applies to; `removeSubfolder`/
 *  `removeItem` take the *parent's* path plus the child's `key`, matching
 *  the `(folder, key)` shape of the plain `removeSubfolder`/`removeItem`
 *  functions above. */
export interface DirectoryMutations {
  renameFolder(path: readonly number[], name: string): void;
  renameItemId(path: readonly number[], key: string, id: string): void;
  addSubfolder(path: readonly number[]): void;
  addItem(path: readonly number[], compositionUid: string): void;
  removeSubfolder(parentPath: readonly number[], key: string): void;
  removeItem(parentPath: readonly number[], key: string): void;
}

export const DIRECTORY_MUTATIONS_KEY: InjectionKey<DirectoryMutations> =
  Symbol("directoryMutations");
