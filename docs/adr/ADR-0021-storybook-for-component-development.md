# ADR-0021: Storybook for Component Development

**Date:** 2026-08-27
**Status:** Accepted
**Deciders:** Development Team
**Related:** OEH-34 (Linear)

## Context

The project had no isolated environment for developing or visually inspecting Vue components. Every component (`src/components/`) could only be exercised by running the full Tauri app and navigating to a view that happens to use it — slow for iterating on a single component, and there was no lightweight way to see a component's states (e.g. `disabled`, `checked`, with/without a label) side by side, or to document its props for other contributors.

The request was simply "do we have Storybook set up?" — we didn't, so this ADR records the decision to add it and the scope chosen.

## Decision

We will use **Storybook 10** with the `@storybook/vue3-vite` framework preset as the component-development/documentation tool for `src/components/`.

Stories are colocated with the component they document (`Foo.vue` + `Foo.stories.ts`), matching the existing convention of colocating a component's Vue file, not a separate `src/stories/` tree.

### Scope kept minimal on purpose

Storybook's `init` scaffolds a larger default setup than we adopted. We deliberately trimmed it:

- **Kept:** `storybook`, `@storybook/vue3-vite` (framework), `@storybook/addon-docs` (auto-generated docs/controls from `defineProps`), `@storybook/addon-a11y` (accessibility checks in the story UI).
- **Dropped `@storybook/addon-onboarding`:** it only drives the first-run interactive tutorial overlay; not useful once the project is set up, and it ships tutorial assets we don't need in the repo.
- **Dropped `@chromatic-com/storybook`:** wires into Chromatic, a third-party hosted visual-regression service. Nothing in this project currently uses Chromatic, and adding a SaaS integration wasn't part of the ask — can be revisited if visual regression testing becomes a real need.
- **Dropped `@storybook/addon-vitest` (+ `@vitest/browser-playwright`, `@vitest/coverage-v8`):** this addon turns every story into a real-browser Vitest test via Playwright. It's a legitimate testing strategy, but it's a separate decision from "can we develop/inspect components in isolation" — it would change `vitest.config.ts` into a multi-project config, add a browser dependency to the fast `npm run test` path, and needs its own CI wiring. Left for a follow-up ADR if we decide to test stories, not just view them.
- **Dropped the scaffolded `src/stories/` example** (Button/Header/Page demo components from the Storybook template) — not part of this app, would just be noise. Replaced with one real story, `src/components/ToggleSwitch.stories.ts`, covering the `Off` / `On` / `Disabled` / `WithoutLabel` states, as a working example for future stories to copy.

### Theming

`src/App.vue` defines the app's CSS custom properties (`--color-bg`, `--color-text`, etc.) in a global, unscoped `<style>` block that's only loaded when the full app mounts. Storybook renders components standalone, so those tokens would otherwise be missing. `.storybook/preview.css` carries a copy of the `:root` token block plus the base `body` rule, imported from `.storybook/preview.ts`, so components render with real theme colors instead of unstyled defaults. This is a duplicated subset that needs to stay in sync manually if the palette in `App.vue` changes — acceptable given how rarely the palette changes, and simpler than extracting the tokens into a shared file solely for Storybook's benefit.

### Dependency pinning

All Storybook packages are pinned to exact versions (`10.5.10`), per ADR-0007 / the pinned-dependency convention in `CLAUDE.md` — `storybook init` writes `^`-prefixed ranges by default (bypassing the repo's `save-exact=true` `.npmrc`, which only applies to `npm install <pkg>`), so these were pinned by hand after scaffolding.

### CI

`npm run build-storybook` was added as a step in the `frontend` CI job (`.github/workflows/ci.yml`) so a story or config that breaks the Storybook build fails CI, the same way `npm run build` already guards the app build.

## Consequences

### Positive
- Components can be developed and visually reviewed in isolation (`npm run storybook`), without running the full Tauri app or navigating to a specific view.
- `@storybook/addon-docs` auto-generates a props/controls table from each component's `defineProps`, giving lightweight documentation for free.
- `@storybook/addon-a11y` surfaces accessibility issues per-component as stories are written.
- CI catches a broken Storybook build before merge.

### Negative
- Another devDependency tree to keep pinned/updated alongside the app's own dependencies.
- The `.storybook/preview.css` token subset can drift from `src/App.vue`'s palette if the app's theme changes and this file isn't updated alongside it.
- Coverage is opt-in: stories only exist for components someone chooses to write one for (currently just `ToggleSwitch`). No mechanism enforces every new component gets a story.

### Neutral
- Story-as-test (`@storybook/addon-vitest`) was evaluated and deliberately deferred rather than rejected — see "Scope kept minimal on purpose" above.

## Alternatives Considered

### Full `storybook init` scaffold (all addons, `addon-vitest` browser testing)
- **Pros:** Zero extra setup work; story-driven browser testing "for free".
- **Cons:** Pulls in a third-party SaaS integration (Chromatic) unasked, a tutorial addon with no lasting value, and restructures `vitest.config.ts` into a multi-project config with a Playwright browser dependency on the default `npm run test` path — a much bigger change than "add Storybook".
- **Verdict:** Rejected for this ADR; addon-vitest can be reconsidered separately if we want stories to double as tests.

### Histoire (Vite-native alternative to Storybook)
- **Pros:** Lighter weight, built specifically for Vite, faster startup.
- **Cons:** Smaller ecosystem/addon surface, less familiar to contributors coming from other projects, project has since seen slower maintenance activity than Storybook.
- **Verdict:** Rejected in favor of Storybook's larger ecosystem and addon-docs/addon-a11y support.

### No component workbench (status quo)
- **Pros:** No new dependency.
- **Cons:** Components can only be inspected in the context of a full app view; no isolated state exploration or generated docs.
- **Verdict:** Rejected — this was the gap prompting the request.

## References

- [Storybook documentation](https://storybook.js.org/docs)
- ADR-0007: Pinned Dependency Versions
- `src/components/ToggleSwitch.stories.ts` — reference story
- `.storybook/main.ts`, `.storybook/preview.ts`, `.storybook/preview.css`
