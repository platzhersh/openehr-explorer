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
 */

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
      type: item.type ?? "COMPOSITION",
      namespace: item.namespace ?? "local",
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
export function toWireFolder(folder: EditableFolder, isRoot = true): Record<string, unknown> {
  const wire: Record<string, unknown> = {
    _type: "FOLDER",
    name: { value: folder.name.trim() || "(unnamed folder)" },
    items: folder.items
      .filter((item) => item.id.trim().length > 0)
      .map((item) => ({
        _type: "OBJECT_REF",
        id: { _type: "HIER_OBJECT_ID", value: item.id.trim() },
        namespace: item.namespace.trim() || "local",
        type: item.type.trim() || "COMPOSITION",
      })),
    folders: folder.folders.map((sub) => toWireFolder(sub, false)),
  };

  if (isRoot) {
    wire.archetype_node_id = "openEHR-EHR-FOLDER.generic.v1";
    wire.archetype_details = {
      _type: "ARCHETYPED",
      archetype_id: {
        _type: "ARCHETYPE_ID",
        value: "openEHR-EHR-FOLDER.generic.v1",
      },
      rm_version: "1.0.4",
    };
  }

  return wire;
}

export function addSubfolder(folder: EditableFolder, name = "New folder"): void {
  folder.folders.push(emptyFolder(name));
}

export function addItem(folder: EditableFolder, id: string, type = "COMPOSITION"): void {
  folder.items.push({ key: nextKey("item"), id, type, namespace: "local" });
}

export function removeSubfolder(folder: EditableFolder, key: string): void {
  folder.folders = folder.folders.filter((sub) => sub.key !== key);
}

export function removeItem(folder: EditableFolder, key: string): void {
  folder.items = folder.items.filter((item) => item.key !== key);
}
