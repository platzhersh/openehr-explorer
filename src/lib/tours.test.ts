import { describe, expect, it } from "vitest";
import { TOURS, getTourById, getTourForRoute } from "./tours";

describe("tours", () => {
  it("every tour has a unique id", () => {
    const ids = TOURS.map((t) => t.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("every tour has at least one step and one route name", () => {
    for (const tour of TOURS) {
      expect(tour.steps.length).toBeGreaterThan(0);
      expect(tour.routeNames.length).toBeGreaterThan(0);
    }
  });

  it("every step targets a data-tour attribute selector", () => {
    for (const tour of TOURS) {
      for (const step of tour.steps) {
        expect(step.target).toMatch(/^\[data-tour="[a-z0-9-]+"\]$/);
      }
    }
  });

  it("no route name is claimed by more than one tour", () => {
    const seen = new Map<string, string>();
    for (const tour of TOURS) {
      for (const routeName of tour.routeNames) {
        expect(seen.has(routeName)).toBe(false);
        seen.set(routeName, tour.id);
      }
    }
  });

  it("getTourById finds a known tour and returns undefined for an unknown one", () => {
    expect(getTourById("ehrs")?.label).toBe("EHR Browser");
    expect(getTourById("nope")).toBeUndefined();
  });

  it("getTourForRoute resolves aliased routes to the same tour", () => {
    expect(getTourForRoute("ehrs")?.id).toBe("ehrs");
    expect(getTourForRoute("ehr-detail")?.id).toBe("ehrs");
  });

  it("getTourForRoute returns undefined for routes with no tour", () => {
    expect(getTourForRoute("settings")).toBeUndefined();
    expect(getTourForRoute(undefined)).toBeUndefined();
    expect(getTourForRoute(null)).toBeUndefined();
  });
});
