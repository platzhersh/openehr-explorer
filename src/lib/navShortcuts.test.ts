import { describe, expect, it } from "vitest";
import {
  navItems,
  pathForShortcutKey,
  shortcutKeyForIndex,
  shortcutRangeLabel,
} from "./navShortcuts";

describe("navShortcuts", () => {
  it("maps every tab to its position-based key", () => {
    navItems.forEach((item, i) => {
      expect(pathForShortcutKey(shortcutKeyForIndex(i))).toBe(item.path);
    });
  });

  it("maps comma to settings", () => {
    expect(pathForShortcutKey(",")).toBe("/settings");
  });

  it("ignores keys beyond the tab list and non-shortcut keys", () => {
    expect(pathForShortcutKey(String(navItems.length + 1))).toBeNull();
    expect(pathForShortcutKey("0")).toBeNull();
    expect(pathForShortcutKey("a")).toBeNull();
  });

  it("describes the range from the tab count", () => {
    expect(shortcutRangeLabel()).toBe(`1–${navItems.length}`);
  });
});
