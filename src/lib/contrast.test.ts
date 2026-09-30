import { describe, expect, it } from "vitest";
import root from "../styles/tokens.css?inline";
import medblocks from "../styles/medblocks-overrides.css?inline";

// Guards the design tokens in src/styles/tokens.css against WCAG 2.x AA (ADR-0028).
// Computed-style checks over real components run separately via
// `npm run test:a11y` (axe-core against the built Storybook).

const token = (name: string): string => {
  const m = root.match(new RegExp(`--color-${name}:\\s*(#[0-9a-fA-F]{6})`));
  if (!m) throw new Error(`token --color-${name} not found in tokens.css`);
  return m[1]!;
};

const luminance = (hex: string): number => {
  const [r, g, b] = [1, 3, 5]
    .map((i) => parseInt(hex.slice(i, i + 2), 16) / 255)
    .map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r! + 0.7152 * g! + 0.0722 * b!;
};
const contrast = (a: string, b: string): number => {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi! + 0.05) / (lo! + 0.05);
};

// Every surface text is actually rendered on.
const surfaces = ["bg", "bg-secondary", "bg-tertiary", "surface", "surface-hover"];
// Every token used as text color.
const textTokens = [
  "text",
  "text-secondary",
  "text-muted",
  "primary",
  "error",
  "warning",
  "success",
];

describe("color token contrast (WCAG AA, 4.5:1)", () => {
  for (const fg of textTokens) {
    for (const bg of surfaces) {
      it(`--color-${fg} on --color-${bg}`, () => {
        expect(contrast(token(fg), token(bg))).toBeGreaterThanOrEqual(4.5);
      });
    }
  }
});

describe("primary-dim as a filled background", () => {
  // .btn-primary, .badge-primary, update banner: white text on --color-primary-dim.
  it("white text on --color-primary-dim", () => {
    expect(contrast("#ffffff", token("primary-dim"))).toBeGreaterThanOrEqual(4.5);
  });
  // Borders/focus rings are UI components: WCAG 1.4.11 asks for 3:1.
  it("--color-primary-dim border on --color-bg", () => {
    expect(contrast(token("primary-dim"), token("bg"))).toBeGreaterThanOrEqual(3);
  });
});

describe("medblocks-ui form inputs (Shoelace overrides)", () => {
  // The Shoelace components only load from the CDN in the app/Storybook
  // (ADR-0008), so axe can't see them offline — check the token pair here.
  const slColor = (name: string): string => {
    const m = medblocks.match(new RegExp(`--sl-color-${name}:\\s*(#[0-9a-fA-F]{6})`));
    if (!m) throw new Error(`--sl-color-${name} not found in medblocks-overrides.css`);
    return m[1]!;
  };
  const ref = (prop: string): string => {
    const m = medblocks.match(new RegExp(`--sl-${prop}:\\s*var\\(--sl-color-([a-z0-9-]+)\\)`));
    if (!m) throw new Error(`--sl-${prop} not found in medblocks-overrides.css`);
    return slColor(m[1]!);
  };
  for (const bg of ["input-background-color", "input-background-color-hover"]) {
    it(`placeholder on --sl-${bg}`, () => {
      expect(contrast(ref("input-placeholder-color"), ref(bg))).toBeGreaterThanOrEqual(4.5);
    });
  }
  it("input text on input background", () => {
    expect(contrast(ref("input-color"), ref("input-background-color"))).toBeGreaterThanOrEqual(4.5);
  });
});
