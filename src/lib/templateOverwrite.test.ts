import { describe, expect, it } from "vitest";
import {
  duplicateMayReplace,
  duplicateWithoutOverwriteMessage,
  supportsTemplateOverwrite,
} from "./templateOverwrite";

describe("supportsTemplateOverwrite", () => {
  it("is only implemented for EHRbase", () => {
    expect(supportsTemplateOverwrite("ehrbase")).toBe(true);
    expect(supportsTemplateOverwrite("better_platform")).toBe(false);
    expect(supportsTemplateOverwrite("ferro_ehr")).toBe(false);
    expect(supportsTemplateOverwrite("generic")).toBe(false);
    expect(supportsTemplateOverwrite(undefined)).toBe(false);
  });
});

describe("duplicateWithoutOverwriteMessage", () => {
  it("explains FerroEHR's immutable templates", () => {
    const message = duplicateWithoutOverwriteMessage("ferro_ehr");
    expect(message).toContain("already exists");
    expect(message).toContain("immutable");
    expect(message).toContain("new template ID or version");
  });

  it("says nothing changed on Better Platform, which versions templates", () => {
    const message = duplicateWithoutOverwriteMessage("better_platform");
    expect(message).toContain("nothing changed");
    expect(message).toContain("new active version");
  });

  it("stays general for other server types", () => {
    for (const type of ["generic", undefined] as const) {
      expect(duplicateWithoutOverwriteMessage(type)).toContain(
        "Overwriting isn't supported for this server type",
      );
    }
  });
});

describe("duplicateMayReplace", () => {
  it("asks first except on FerroEHR (always refuses) and Better (versions templates)", () => {
    expect(duplicateMayReplace("ehrbase")).toBe(true);
    expect(duplicateMayReplace("better_platform")).toBe(false);
    expect(duplicateMayReplace("generic")).toBe(true);
    expect(duplicateMayReplace(undefined)).toBe(true);
    expect(duplicateMayReplace("ferro_ehr")).toBe(false);
  });
});
