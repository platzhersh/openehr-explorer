# ADR-0028: WCAG AA Color Contrast Tokens and CI Gate

**Date:** 2026-09-30
**Status:** Accepted
**Deciders:** Development Team
**Related:** ADR-0021 (Storybook for Component Development), ADR-0026 (Shared Storybook)

## Context

The dark theme's secondary text tokens were too dim. `--color-text-muted` (`#5a6a8a`) measured 2.1–3.1:1 on the app's surfaces and was used in 100+ places (archetype paths, hints, line numbers, timestamps). `--color-text-secondary` failed on hover and tertiary surfaces. Several components also dimmed text with `opacity` on top of an already-muted color (tree badges at 0.6/0.4 gave ~1.6–2.5:1), and a few hard-coded colors (update banner button, version badge) failed on their own backgrounds. WCAG 2.x AA requires 4.5:1 for normal text and 3:1 for UI component boundaries.

`@storybook/addon-a11y` was installed but set to `test: "todo"`, so nothing ran in CI.

## Decision

1. **Tokens live in `src/styles/tokens.css`**, imported by `App.vue` and `.storybook/preview.css` (previously a hand-synced copy). The website mirrors the palette in `website/src/styles/tokens.css`.
2. **Text tokens must reach 4.5:1 on every surface token.** `--color-primary-dim` is a *fill* (white text on it ≥ 4.5:1, border ≥ 3:1 on `--color-bg`), not a text color.
3. **Don't dim text with `opacity`.** Pick a token (`--color-text-secondary` / `--color-text-muted`). Opacity is fine for disabled controls (WCAG-exempt) and non-text decoration.
4. **Two CI gates:**
   - `src/lib/contrast.test.ts` (runs in `npm test`): every text token × every surface, computed from `tokens.css`. Fast; catches palette regressions.
   - `npm run test:a11y` (`scripts/a11y-storybook.mjs`): axe-core's `color-contrast` rule over every story in the built Storybook, in real Chromium with computed colors. Catches opacity stacking, hard-coded colors, and per-component bugs. Exits non-zero on any violation.

## Consequences

- New components are covered automatically *if they have a story*. Views/states without stories are not scanned; add stories for new UI.
- axe can't evaluate text over images/gradients ("incomplete"); those need manual review.
- CI gains a Playwright Chromium install (~1 min).
- Only `color-contrast` is gated. Other axe rules (labels, roles) can be enabled via `A11Y_RULES=...` once their existing violations are triaged.
- The muted/secondary palette is lighter, so the visual hierarchy between `text`, `secondary` and `muted` is slightly flatter than before. This is the accepted cost.
