import { describe, expect, it } from "vitest";
import { computeVirtualRange, virtualScrollOffset } from "./virtualList";

describe("computeVirtualRange", () => {
  it("returns an empty range for an empty list", () => {
    expect(
      computeVirtualRange({ scrollTop: 0, viewportHeight: 200, rowHeight: 20, itemCount: 0 }),
    ).toEqual({ startIndex: 0, endIndex: 0, topPadding: 0, bottomPadding: 0 });
  });

  it("renders from the top with overscan clamped at zero", () => {
    const range = computeVirtualRange({
      scrollTop: 0,
      viewportHeight: 100,
      rowHeight: 20,
      itemCount: 1000,
      overscan: 5,
    });
    // 100/20 = 5 visible rows + 5 overscan on each side, clamped to 0 at the top.
    expect(range.startIndex).toBe(0);
    expect(range.endIndex).toBe(15);
    expect(range.topPadding).toBe(0);
    expect(range.bottomPadding).toBe((1000 - 15) * 20);
  });

  it("windows around the current scroll position", () => {
    const range = computeVirtualRange({
      scrollTop: 500,
      viewportHeight: 100,
      rowHeight: 20,
      itemCount: 1000,
      overscan: 5,
    });
    // scrollTop 500 / rowHeight 20 = row 25, minus 5 overscan = row 20.
    expect(range.startIndex).toBe(20);
    expect(range.endIndex).toBe(35);
    expect(range.topPadding).toBe(400);
    expect(range.bottomPadding).toBe((1000 - 35) * 20);
  });

  it("clamps the end of the window at the last item", () => {
    const range = computeVirtualRange({
      scrollTop: 900,
      viewportHeight: 100,
      rowHeight: 20,
      itemCount: 50,
      overscan: 5,
    });
    expect(range.endIndex).toBe(50);
    expect(range.bottomPadding).toBe(0);
  });

  it("never renders a negative or out-of-range start", () => {
    const range = computeVirtualRange({
      scrollTop: -50,
      viewportHeight: 100,
      rowHeight: 20,
      itemCount: 3,
    });
    expect(range.startIndex).toBe(0);
    expect(range.endIndex).toBe(3);
  });
});

describe("virtualScrollOffset", () => {
  it("returns the row's own top offset when not centering", () => {
    expect(virtualScrollOffset(10, 20, 200, false)).toBe(200);
  });

  it("centers the row within the viewport", () => {
    // row 10 top = 200, minus half the viewport (100) plus half a row (10) = 110.
    expect(virtualScrollOffset(10, 20, 200, true)).toBe(110);
  });

  it("never returns a negative offset", () => {
    expect(virtualScrollOffset(0, 20, 400, true)).toBe(0);
  });
});
