import { describe, expect, it } from "vitest";
import { extractFlatPaths, flatPathStyle, normalizeWebTemplate } from "./webtemplate";

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

describe("extractFlatPaths", () => {
  // Cardinalities behind the sample composition in Better's API cookbook.
  const template = {
    tree: {
      id: "vital_signs",
      max: 1,
      children: [
        {
          id: "blood_pressure",
          max: -1,
          children: [
            { id: "any_event", max: -1, children: [{ id: "systolic", max: 1 }] },
            { id: "location", max: 1, children: [{ id: "location_of_measurement", max: 1 }] },
            { id: "comment", max: 3 },
          ],
        },
      ],
    },
  };

  it("indexes every repeatable node in Better's style", () => {
    expect(extractFlatPaths(template, flatPathStyle("better_platform"))).toEqual([
      "vital_signs/blood_pressure:0/any_event:0/systolic",
      "vital_signs/blood_pressure:0/comment:0",
      "vital_signs/blood_pressure:0/location/location_of_measurement",
    ]);
  });

  it("leaves paths unindexed for other server types", () => {
    for (const type of ["ehrbase", "ferro_ehr", "generic", undefined]) {
      expect(extractFlatPaths(template, flatPathStyle(type))).toEqual([
        "vital_signs/blood_pressure/any_event/systolic",
        "vital_signs/blood_pressure/comment",
        "vital_signs/blood_pressure/location/location_of_measurement",
      ]);
    }
  });
});
