import { describe, expect, it } from "vitest";
import { normalizeWebTemplate } from "./webtemplate";

describe("normalizeWebTemplate", () => {
  it("gives nameless structural nodes (Better's context/category) a readable name", () => {
    const out = normalizeWebTemplate({
      tree: {
        id: "root",
        name: "root",
        children: [
          { id: "context", rm_type: "EVENT_CONTEXT" },
          { id: "blood_pressure", name: "Blood pressure" },
          { id: "x", localized_name: "Localized" },
        ],
      },
    });
    const kids = (out.tree as { children: { name: string; inputs: unknown[] }[] }).children;
    expect(kids.map((k) => k.name)).toEqual(["Context", "Blood pressure", "Localized"]);
    expect(kids[0].inputs).toEqual([]);
  });

  it("converts Better's snake_case keys to the camelCase medblocks-ui reads", () => {
    const out = normalizeWebTemplate({
      template_id: "T",
      tree: {
        id: "root",
        name: "root",
        rm_type: "COMPOSITION",
        localized_names: { en_us: "Root" },
        children: [
          {
            id: "temp",
            name: "Temp",
            rm_type: "DV_QUANTITY",
            in_context: false,
            inputs: [{ suffix: "magnitude", validation: { range: { min_op: ">=", min: 0 } } }],
          },
        ],
      },
    });
    const tree = out.tree as Record<string, any>;
    expect(out.templateId).toBe("T");
    expect(tree.rmType).toBe("COMPOSITION");
    expect(tree.localizedNames).toEqual({ en_us: "Root" });
    expect(tree.children[0].rmType).toBe("DV_QUANTITY");
    expect(tree.children[0].inContext).toBe(false);
    expect(tree.children[0].inputs[0].validation.range.minOp).toBe(">=");
  });
});
