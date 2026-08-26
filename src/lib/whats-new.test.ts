import { describe, expect, it } from "vitest";
import { WHATS_NEW, compareVersions, getEntriesSince } from "./whats-new";

describe("compareVersions", () => {
  it("orders by major, then minor, then patch", () => {
    expect(compareVersions("1.0.0", "2.0.0")).toBe(-1);
    expect(compareVersions("2.0.0", "1.0.0")).toBe(1);
    expect(compareVersions("1.2.0", "1.10.0")).toBe(-1);
    expect(compareVersions("1.2.5", "1.2.4")).toBe(1);
    expect(compareVersions("0.7.0", "0.7.0")).toBe(0);
  });

  it("treats malformed input as 0.0.0 rather than throwing", () => {
    expect(compareVersions("not-a-version", "0.0.1")).toBe(-1);
  });
});

describe("getEntriesSince", () => {
  it("returns nothing older than or equal to the last seen version", () => {
    const entries = getEntriesSince("999.0.0");
    expect(entries).toEqual([]);
  });

  it("returns every known entry when lastSeenVersion is null", () => {
    expect(getEntriesSince(null)).toHaveLength(WHATS_NEW.length);
  });

  it("returns entries in chronological (oldest-first) order", () => {
    const entries = getEntriesSince("0.0.0");
    for (let i = 1; i < entries.length; i++) {
      expect(compareVersions(entries[i - 1].version, entries[i].version)).toBeLessThanOrEqual(0);
    }
  });
});
