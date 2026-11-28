import { describe, expect, it } from "vitest";
import { aqlPathForFlatPath, aqlPathsForFlatPaths, buildArchetypePathMap } from "./aqlPathIndex";

const BP = "openEHR-EHR-OBSERVATION.blood_pressure.v2";
const SYSTOLIC = "data[at0001]/events[at0006]/data[at0003]/items[at0004]/value";

// Better's shape: snake_case keys, `id` is the readable segment, `node_id` the archetype.
const template = {
  tree: {
    id: "vital_signs",
    name: "Vital signs",
    node_id: "openEHR-EHR-COMPOSITION.encounter.v1",
    aql_path: "",
    children: [
      {
        id: "blood_pressure",
        name: "Blood pressure",
        node_id: BP,
        aql_path: `/content[${BP}]`,
        children: [{ id: "systolic", name: "Systolic", aql_path: `/content[${BP}]/${SYSTOLIC}` }],
      },
    ],
  },
};

describe("buildArchetypePathMap", () => {
  it("keys archetypes by node id, with paths relative to the archetype", () => {
    const map = buildArchetypePathMap(template);
    expect(map.get(BP)?.map((e) => e.aqlPath)).toEqual([SYSTOLIC]);
  });

  it("relates the composition's paths to the composition", () => {
    const map = buildArchetypePathMap(template);
    expect(map.get("openEHR-EHR-COMPOSITION.encounter.v1")?.map((e) => e.aqlPath)).toEqual([
      `content[${BP}]`,
      `content[${BP}]/${SYSTOLIC}`,
    ]);
  });

  it("lists a path once when the archetype is used twice", () => {
    const bp = template.tree.children[0];
    const twice = { tree: { ...template.tree, children: [bp, bp] } };
    expect(buildArchetypePathMap(twice).get(BP)).toHaveLength(1);
  });
});

describe("aqlPathForFlatPath", () => {
  it("ignores occurrence indexes and attribute suffixes", () => {
    expect(aqlPathForFlatPath(template, "vital_signs/blood_pressure:1/systolic|magnitude")).toBe(
      `/content[${BP}]/${SYSTOLIC}`,
    );
  });

  it("returns null for keys the template does not describe", () => {
    expect(aqlPathForFlatPath(template, "ctx/language")).toBeNull();
    expect(aqlPathForFlatPath(template, "vital_signs/unknown")).toBeNull();
    expect(aqlPathsForFlatPaths(template, ["ctx/language"])).toEqual({});
  });
});
