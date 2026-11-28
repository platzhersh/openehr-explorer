import { describe, expect, it } from "vitest";
import { supportsCompositionValidation } from "./compositionValidation";

describe("supportsCompositionValidation", () => {
  it("is offered for Better Platform only", () => {
    expect(supportsCompositionValidation("better_platform")).toBe(true);
    for (const type of ["ehrbase", "ferro_ehr", "generic", undefined] as const) {
      expect(supportsCompositionValidation(type)).toBe(false);
    }
  });
});
