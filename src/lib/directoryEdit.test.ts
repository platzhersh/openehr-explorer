import { describe, expect, it } from "vitest";
import {
  addItem,
  addSubfolder,
  emptyFolder,
  fromWireFolder,
  getFolderAtPath,
  removeItem,
  removeSubfolder,
  toWireFolder,
} from "./directoryEdit";

describe("emptyFolder", () => {
  it("creates a folder with no items or subfolders", () => {
    const folder = emptyFolder("Root");
    expect(folder.name).toBe("Root");
    expect(folder.items).toEqual([]);
    expect(folder.folders).toEqual([]);
  });

  it("gives each call a distinct key", () => {
    const a = emptyFolder();
    const b = emptyFolder();
    expect(a.key).not.toBe(b.key);
  });
});

describe("fromWireFolder", () => {
  it("extracts name, items, and nested folders", () => {
    const wire = {
      name: { value: "Encounters" },
      items: [{ id: { value: "abc-123" }, type: "COMPOSITION", namespace: "local" }],
      folders: [{ name: { value: "2026" }, items: [], folders: [] }],
    };
    const editable = fromWireFolder(wire);
    expect(editable.name).toBe("Encounters");
    expect(editable.items).toHaveLength(1);
    expect(editable.items[0]).toMatchObject({
      id: "abc-123",
      type: "COMPOSITION",
      namespace: "local",
    });
    expect(editable.folders).toHaveLength(1);
    expect(editable.folders[0].name).toBe("2026");
  });

  it("falls back to empty defaults for missing fields", () => {
    const editable = fromWireFolder({});
    expect(editable.name).toBe("");
    expect(editable.items).toEqual([]);
    expect(editable.folders).toEqual([]);
  });

  it("defaults item type/namespace when the server omits them", () => {
    const editable = fromWireFolder({
      name: { value: "Root" },
      items: [{ id: { value: "abc-123" } }],
    });
    expect(editable.items[0]).toMatchObject({
      id: "abc-123",
      type: "COMPOSITION",
      namespace: "local",
    });
  });
});

describe("toWireFolder", () => {
  it("builds a root FOLDER with archetype_details", () => {
    const folder = emptyFolder("Root");
    addItem(folder, "abc-123");
    const wire = toWireFolder(folder);
    expect(wire._type).toBe("FOLDER");
    expect(wire.name).toEqual({ value: "Root" });
    expect(wire.archetype_node_id).toBe("openEHR-EHR-FOLDER.generic.v1");
    expect((wire.archetype_details as any).archetype_id.value).toBe(
      "openEHR-EHR-FOLDER.generic.v1",
    );
    expect(wire.items).toEqual([
      {
        _type: "OBJECT_REF",
        id: { _type: "HIER_OBJECT_ID", value: "abc-123" },
        namespace: "local",
        type: "COMPOSITION",
      },
    ]);
  });

  it("omits archetype_details on nested folders but still sets archetype_node_id", () => {
    const folder = emptyFolder("Root");
    addSubfolder(folder, "2026");
    const wire = toWireFolder(folder);
    const subWire = (wire.folders as Record<string, unknown>[])[0];
    expect(subWire.archetype_details).toBeUndefined();
    expect(subWire.archetype_node_id).toBe("openEHR-EHR-FOLDER.generic.v1");
    expect(subWire.name).toEqual({ value: "2026" });
  });

  it("drops items with a blank UID", () => {
    const folder = emptyFolder("Root");
    addItem(folder, "   ");
    addItem(folder, "real-uid");
    const wire = toWireFolder(folder);
    expect(wire.items).toHaveLength(1);
    expect((wire.items as Record<string, unknown>[])[0].id).toEqual({
      _type: "HIER_OBJECT_ID",
      value: "real-uid",
    });
  });

  it("falls back to a placeholder name for a blank folder name", () => {
    const folder = emptyFolder("   ");
    const wire = toWireFolder(folder);
    expect(wire.name).toEqual({ value: "(unnamed folder)" });
  });

  it("round-trips through fromWireFolder", () => {
    const original = {
      name: { value: "Patient Records" },
      items: [{ id: { value: "item-1" }, type: "COMPOSITION", namespace: "local" }],
      folders: [
        {
          name: { value: "2026" },
          items: [],
          folders: [{ name: { value: "August" }, items: [], folders: [] }],
        },
      ],
    };
    const editable = fromWireFolder(original);
    const wire = toWireFolder(editable);
    expect(wire.name).toEqual({ value: "Patient Records" });
    expect(wire.items).toEqual([
      {
        _type: "OBJECT_REF",
        id: { _type: "HIER_OBJECT_ID", value: "item-1" },
        namespace: "local",
        type: "COMPOSITION",
      },
    ]);
    const level1 = (wire.folders as Record<string, unknown>[])[0];
    expect(level1.name).toEqual({ value: "2026" });
    const level2 = (level1.folders as Record<string, unknown>[])[0];
    expect(level2.name).toEqual({ value: "August" });
  });
});

describe("addSubfolder / addItem / removeSubfolder / removeItem", () => {
  it("adds and removes a subfolder", () => {
    const folder = emptyFolder("Root");
    addSubfolder(folder, "Child");
    expect(folder.folders).toHaveLength(1);
    const childKey = folder.folders[0].key;
    removeSubfolder(folder, childKey);
    expect(folder.folders).toEqual([]);
  });

  it("adds and removes an item", () => {
    const folder = emptyFolder("Root");
    addItem(folder, "abc-123");
    expect(folder.items).toHaveLength(1);
    const itemKey = folder.items[0].key;
    removeItem(folder, itemKey);
    expect(folder.items).toEqual([]);
  });

  it("only removes the targeted subfolder/item, leaving siblings intact", () => {
    const folder = emptyFolder("Root");
    addSubfolder(folder, "A");
    addSubfolder(folder, "B");
    addItem(folder, "item-a");
    addItem(folder, "item-b");
    removeSubfolder(folder, folder.folders[0].key);
    removeItem(folder, folder.items[0].key);
    expect(folder.folders).toHaveLength(1);
    expect(folder.folders[0].name).toBe("B");
    expect(folder.items).toHaveLength(1);
    expect(folder.items[0].id).toBe("item-b");
  });
});

describe("getFolderAtPath", () => {
  it("returns the root for an empty path", () => {
    const root = emptyFolder("Root");
    expect(getFolderAtPath(root, [])).toBe(root);
  });

  it("resolves a nested path through folders[]", () => {
    const root = emptyFolder("Root");
    addSubfolder(root, "2026");
    addSubfolder(root.folders[0], "August");
    const resolved = getFolderAtPath(root, [0, 0]);
    expect(resolved.name).toBe("August");
    expect(resolved).toBe(root.folders[0].folders[0]);
  });

  it("distinguishes siblings by index", () => {
    const root = emptyFolder("Root");
    addSubfolder(root, "A");
    addSubfolder(root, "B");
    expect(getFolderAtPath(root, [0]).name).toBe("A");
    expect(getFolderAtPath(root, [1]).name).toBe("B");
  });

  it("throws with the offending path rather than returning undefined for a stale index", () => {
    const root = emptyFolder("Root");
    addSubfolder(root, "A");
    expect(() => getFolderAtPath(root, [5])).toThrow(/no folder at index 5/);
    expect(() => getFolderAtPath(root, [0, 3])).toThrow(/no folder at index 3/);
  });
});
