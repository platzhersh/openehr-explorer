import { describe, expect, it } from "vitest";

// Regression guard for OEH-107: expand/collapse carets are drawn by the shared
// <CaretIcon> (or <CollapsibleSection>), never by `▶ ▼ ▾` text glyphs, which
// render differently per OS/font and can't be sized or aligned consistently.
const sources = import.meta.glob("../**/*.vue", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

// Text-glyph carets, as literal characters or as "▶"-style JS escapes.
const GLYPH = /[▸▶▾▼▴◀]|\\u25(?:B6|BC|B8|BE|B2|C0)/i;

// Keys are relative to this file ("./X.vue", "../views/Y.vue"); show them as src/ paths.
const toSrcPath = (key: string): string =>
  key.startsWith("./") ? `src/components/${key.slice(2)}` : `src/${key.slice(3)}`;

describe("expand/collapse carets (OEH-107)", () => {
  it("scans the app's components", () => {
    expect(Object.keys(sources).length).toBeGreaterThan(20);
  });

  it("uses no text-glyph carets", () => {
    const offenders = Object.entries(sources)
      // CaretIcon's own docs comment names the glyphs it replaces.
      .filter(([key]) => !key.endsWith("/CaretIcon.vue"))
      .filter(([, source]) => GLYPH.test(source))
      .map(([key]) => toSrcPath(key));
    expect(offenders, "use <CaretIcon> / <CollapsibleSection> instead").toEqual([]);
  });
});
