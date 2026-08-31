# ADR-0026: One Storybook for the App and the Website

**Date:** 2026-08-30
**Status:** Accepted
**Deciders:** Development Team
**Related:** ADR-0021 (Storybook for Component Development), ADR-0024 (Astro + Vue for the Marketing/Docs Site), ADR-0025 (CodeBlock Copy-to-Clipboard Component)

## Context

ADR-0021 set up Storybook (`.storybook/`, root `npm run storybook`) for the app's own components in `src/components/`. ADR-0024 then added a second, entirely separate Vue component tree — `website/src/components/vue/` (`DownloadButton`, `ScreenshotGallery`, `DownloadsSparkline`, `DocsSidebar`, and later `CodeBlock` from ADR-0025) — inside its own npm project (`website/`, its own `package.json`/`node_modules`/lockfile), with no Storybook of its own.

Both component trees are plain Vue 3 SFCs (`website/`'s Vue islands have no Astro-specific imports — no `astro:content`, no `import.meta.env` — they're ordinary components that happen to be mounted by Astro at runtime), and both now pin the same Vue version (`3.5.31`). The question this ADR answers: can — and should — one Storybook instance cover both, rather than either leaving `website/`'s components with no isolated-development story at all, or standing up a second, independent Storybook inside `website/`.

## Decision

We will extend the **existing root Storybook** (not create a second one) to also render `website/src/components/vue/`'s components, by widening `.storybook/main.ts`'s `stories` glob and adding `website/public` as a `staticDirs` entry (so `ScreenshotGallery`'s real screenshot images resolve at the same relative paths the site itself uses). Story files are colocated next to each website component (`CodeBlock.stories.ts` next to `CodeBlock.vue`, etc.), matching the app's existing convention, and grouped under a `Website/` title prefix in the Storybook sidebar to keep the two trees visually distinct from `Components/`.

Storybook renders `website/`'s `.vue` files by transforming them directly off disk through the **app's own** Vite/Vue plugin (the same pipeline that already handles `src/components/`) — `website/`'s own Astro build is never invoked, and nothing in this pipeline depends on Astro at runtime.

### The one real coupling this introduces

Vite's TypeScript transform resolves the *nearest* `tsconfig.json` by walking up from whatever file it's transforming — so a story under `website/src/components/vue/` picks up `website/tsconfig.json`, which `extends: "astro/tsconfigs/strict"`. That `extends` chain has to resolve even though nothing in the Storybook build actually uses Astro's types, which means **`website/node_modules` (specifically, the `astro` package) has to be installed** for `npm run build-storybook` (run from the repo root) to succeed — confirmed by deliberately removing it and watching the build fail with `Tsconfig not found astro/tsconfigs/strict`, then reinstalling and watching it pass again.

`.github/workflows/ci.yml`'s `frontend` job now runs `npm ci --ignore-scripts` inside `website/` (mirroring the `--ignore-scripts` hardening from `.github/workflows/pages.yml`, per the SonarCloud finding on PR #169) immediately before `npm run build-storybook`, so CI installs both dependency trees rather than relying on `website/node_modules` happening to already be present.

### Design tokens: two naming schemes, deliberately left as-is here

`src/App.vue` defines the app's palette as `--color-bg`, `--color-primary`, etc.; `website/src/styles/tokens.css` (ADR-0024) defines the *same* palette as unprefixed `--bg`, `--primary`, etc. (inherited from the original static site's convention, predating ADR-0024). The two schemes don't collide — different variable names — so `.storybook/preview.css` now `@import`s both `website/src/styles/tokens.css` and `website/src/styles/base.css` alongside its existing `--color-*` block, and each component set renders correctly off its own token names in the same shared preview. Reconciling the two naming schemes into one is a real, worthwhile follow-up (this ADR's whole point is reducing exactly this kind of duplication) but is out of scope here — it would mean touching every website page/component's CSS, not just the Storybook config, and isn't required to get a working shared Storybook.

## Consequences

### Positive
- `website/`'s Vue islands get the same isolated-development/visual-review workflow (`npm run storybook`) the app's components already have, instead of only being checkable by running `npm run dev` inside `website/` and clicking through full pages.
- One Storybook instance, one `npm run storybook`/`build-storybook` command, one deployed Storybook (if this project publishes one) — a contributor doesn't need to know two projects both use Storybook and go looking for a second instance.
- `DocsSidebar`'s story demonstrates its search + active-section-tracking behavior (which reaches into sibling `.docs-content` DOM outside its own template) via a small fake content pane in the story's own `render` — this is the first place that behavior is exercised in isolation from the real docs page at all.
- CI now builds Storybook against `website/`'s components too, so a change that breaks one of those components' stories fails the same `frontend` check the app's own components already gate on.

### Negative
- `website/node_modules` must be installed before `npm run build-storybook` succeeds from the repo root — a non-obvious cross-project dependency for anyone running Storybook locally without having first run `npm ci` inside `website/` (documented in this ADR and inline in `ci.yml`, but still a rough edge).
- Two token-naming conventions for the same palette now visibly coexist in one `preview.css` rather than one being fixed — deferred, not resolved, by this ADR (see above).
- `.storybook/main.ts` and `preview.css` now reference `../website/...` relative paths, coupling the app's Storybook config to `website/`'s directory structure; renaming or relocating `website/` would need this config updated too.

### Neutral
- `website/`'s own dev/build/preview commands (`npm run dev`, `npm run build`) are completely unaffected — this ADR only adds a new consumer (Storybook) of `website/`'s source files, it doesn't change how `website/` builds itself.

## Alternatives Considered

### A second, independent Storybook inside `website/`
- **Pros:** no cross-project coupling; `website/` stays fully self-contained.
- **Cons:** two Storybook instances, two `npm run storybook` commands to remember, two configs to keep consistent (addons, a11y settings, theming) — the opposite of "shared," and the exact kind of duplication ADR-0024's whole premise was to reduce.
- **Verdict:** rejected — doesn't deliver what was actually asked for.

### Move `website/src/components/vue/` into `src/components/` (or a shared package), abandoning the separate npm project
- **Pros:** would eliminate the tsconfig-resolution coupling this ADR documents, since everything would live under one `tsconfig.json`.
- **Cons:** reopens ADR-0024's decision to keep `website/` a separate project from the Tauri app (different bundler, different deployment target, independently versioned) for a benefit (simpler Storybook wiring) that doesn't outweigh that separation. A shared `packages/ui` workspace remains a reasonable future step if genuine component sharing (not just a shared dev tool) materializes — not warranted for five components today.
- **Verdict:** rejected for now — ADR-0024's reasoning for the split still holds; revisit only if real code-sharing (not just Storybook) is needed.

### Leave `website/`'s components without any Storybook coverage
- **Pros:** zero new coupling.
- **Cons:** the gap this ADR closes — `website/`'s more complex components (`DocsSidebar`'s search/observer logic, `CodeBlock`'s clipboard handling) have no isolated way to develop or visually check states short of running the full site.
- **Verdict:** rejected — this was the actual request.

## References

- ADR-0021 — Storybook for Component Development
- ADR-0024 — Astro + Vue for the Marketing/Docs Site
- ADR-0025 — CodeBlock Copy-to-Clipboard as a Vue Component
- `.storybook/main.ts`, `.storybook/preview.css`
- `website/src/components/vue/*.stories.ts`
