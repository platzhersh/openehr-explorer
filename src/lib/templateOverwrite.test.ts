import { describe, expect, it } from "vitest";
import { supportsTemplateOverwrite } from "./templateOverwrite";

describe("supportsTemplateOverwrite", () => {
  it("is only implemented for EHRbase", () => {
    expect(supportsTemplateOverwrite("ehrbase")).toBe(true);
    expect(supportsTemplateOverwrite("better_platform")).toBe(false);
    expect(supportsTemplateOverwrite("ferro_ehr")).toBe(false);
    expect(supportsTemplateOverwrite("generic")).toBe(false);
    expect(supportsTemplateOverwrite(undefined)).toBe(false);
  });
});
