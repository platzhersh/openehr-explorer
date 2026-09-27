import { describe, expect, it } from "vitest";
import { terminologySystemAnalyticsKey } from "./terminology";

describe("terminologySystemAnalyticsKey", () => {
  it("maps known shortcuts case-insensitively", () => {
    expect(terminologySystemAnalyticsKey("SNOMED-CT")).toBe("snomed-ct");
    expect(terminologySystemAnalyticsKey(" loinc ")).toBe("loinc");
  });

  it("maps canonical URIs of known systems to their shortcut", () => {
    expect(terminologySystemAnalyticsKey("http://snomed.info/sct")).toBe("snomed-ct");
    expect(terminologySystemAnalyticsKey("http://hl7.org/fhir/sid/icd-10")).toBe("icd-10");
  });

  it("never leaks custom system identifiers", () => {
    expect(terminologySystemAnalyticsKey("http://example.org/private-codes")).toBe("other");
    expect(terminologySystemAnalyticsKey("")).toBe("other");
  });
});
