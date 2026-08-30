# ADR-0024: Astro + Vue for the Marketing/Docs Site

**Date:** 2026-08-30
**Status:** Proposed
**Deciders:** Development Team
**Related:** PRD-0007 (openEHR Explorer Product Website)

## Context

The public site (`docs/`, deployed by GitHub Pages from the `main` branch's `/docs` folder per PRD-0007) is four hand-written static HTML files: `index.html`, `docs.html`, `compare.html`, `brand-kit.html`. Each one repeats, verbatim, the same ~150 lines of header/footer markup and CSS custom properties (`:root` design tokens), plus its own inline `<script>` block for whatever interactivity that page needs (OS-aware download button, screenshot lightbox, a downloads sparkline, docs search + active-section tracking).

That duplication had already caused real drift: `compare.html`'s `--text-muted` token (`#7b8bb0`) didn't match the other three pages' value (`#5a6a8a`) from PRD-0007's own canonical token table — nobody noticed because there was nothing to keep the four copies in sync.

The app itself (`src/`) is Vue 3 + TypeScript + Vite. The site's tooling had no relationship to the app's: no components, no shared build, no type checking, hand-rolled vanilla JS for anything interactive. The ask that prompted this ADR was to make the site "more reusable" with the app — sharing components and tooling instead of two disconnected codebases that happen to use the same color palette.

## Decision

We will rebuild the site as an **Astro** project in `website/`, using Astro's Vue integration (`@astrojs/vue`) for interactive pieces, and keep it as a **separate npm project** (its own `package.json`) rather than folding it into the root workspace.

- **Astro, not a plain Vue SPA:** the site is content-first (mostly static marketing/docs copy) and needs to ship as fast, crawlable, zero-JS-by-default HTML for SEO (PRD-0007's Lighthouse/SEO goals). Astro's island architecture renders everything to static HTML at build time and only ships JavaScript for the specific components that need it (`client:load`), rather than hydrating a whole app shell like a Vue SPA / Nuxt would.
- **Vue for islands, not React/Svelte/vanilla:** the app is already Vue 3 — using the same framework for the site's interactive components (`DownloadButton.vue`, `ScreenshotGallery.vue`, `DownloadsSparkline.vue`, `DocsSidebar.vue`) means contributors don't context-switch between two component models, and a component that turns out to be useful in both places (e.g. a future shared icon set or a copy-to-clipboard button) can move between `website/src/components/vue/` and `src/components/` with minimal rewriting. The site pins the same Vue version (`3.5.31`) as the app for exactly this reason.
- **Separate `package.json`, not a root workspace:** the app is a Tauri desktop app built with plain Vite; the site is a static marketing/docs generator built with Astro. They don't share a bundler, a `tauri.conf.json`, or a deployment target, and folding them into one npm workspace would couple two independently-versioned, independently-deployed things for no real benefit at this stage. Nothing here rules out a shared workspace or an extracted `@openehr-explorer/ui` package later if genuine component sharing materializes.
- **`build.format: "file"`, not Astro's default directory routing:** keeps the historical URLs (`docs.html`, `compare.html`, `brand-kit.html`) working unchanged — external links, the sitemap, and search engine indexing all keep pointing at the same paths.
- **Deploy via GitHub Actions (`actions/upload-pages-artifact` + `actions/deploy-pages`), not committing `dist/` to `docs/`:** the repository's Pages source is currently "deploy from a branch" (`/docs` on `main`), which is why static HTML lived directly in `docs/` in the first place. Building on every push and publishing the build artifact directly (rather than committing generated output back into `docs/`) avoids checking in build artifacts and matches GitHub's current recommended approach. **This requires a one-time manual change in the repo settings — Settings → Pages → Source → "GitHub Actions"** — before `.github/workflows/pages.yml` takes over deployment; until that switch happens, this workflow builds and validates the site (and can be pointed at a preview/staging Pages environment) without affecting the live site still served from `docs/`.

### What changed structurally

- **Shared design tokens** (`website/src/styles/tokens.css`): the single `:root` token block from PRD-0007's canonical table, imported once. `compare.html`'s drifted `--text-muted` is fixed to the canonical value as part of this migration — a direct example of the problem this ADR is meant to prevent from recurring.
- **Shared layout primitives** (`website/src/components/`): `SiteHeader.astro`, `SimpleFooter.astro`, `LandingFooter.astro`, `SEO.astro`, `Logo.astro`, `GithubIcon.astro`, `FeatureCard.astro`, `Callout.astro` replace what were four independent copies of header/footer/meta-tag markup.
- **Vue islands** replace the vanilla-JS IIFEs that used to sit at the bottom of `index.html` and `docs.html`: OS-aware download button, screenshot lightbox/gallery, downloads-history sparkline, and the docs sidebar's search + `IntersectionObserver` active-section tracking. Behavior is unchanged; each island is a direct port of the corresponding IIFE onto Vue's reactivity.
- **`docs/` is left untouched by this change.** It remains the live, deployed site until the Pages source setting above is switched and this workflow is verified end-to-end.

## Consequences

### Positive
- One source of truth for design tokens and layout chrome — the exact class of bug that motivated this (a page silently drifting from the brand spec) is now caught by simply looking at one file instead of four.
- New pages/sections reuse `Layout.astro`, `SiteHeader.astro`, etc. instead of copy-pasting a `<style>` block and header markup.
- Interactive behavior is testable, typed (`astro check` runs 0 errors/warnings against the whole site), and readable as Vue components instead of jQuery-style DOM IIFEs.
- `npm run build` (`astro check && astro build`) fails CI on a type error or broken page, which plain static HTML had no equivalent of.

### Negative
- A second `node_modules`/toolchain to keep pinned and updated alongside the app's.
- Contributors touching the site now need basic Astro familiarity (`.astro` file syntax) in addition to Vue.
- The migration is a large diff to review carefully for visual regressions, even though it was verified page-by-page against screenshots of the previous static site during development.

### Neutral
- `website/` and `src/` remain independently versioned and built; no shared workspace tooling was introduced. Revisiting that (a real monorepo workspace, or an extracted shared component package) is a separate future decision, not part of this one.

## Migration Plan

1. **This PR (draft):** land `website/` alongside the still-live `docs/`, with a `pages.yml` workflow that builds (and can deploy to a Pages environment once the source setting is switched) but does not yet replace the live site.
2. **Verify:** confirm the built site matches `docs/` visually and functionally (screenshots, Lighthouse, broken-link check).
3. **Cut over:** switch Settings → Pages → Source to "GitHub Actions", merge, confirm the live site is served from the Actions deployment.
4. **Retire `docs/`:** once the cutover is confirmed stable, delete the old static HTML/assets from `docs/` (keeping `docs/adr/` and `docs/prd/`, which are unrelated documentation, not site source).
5. **Follow-ups considered out of scope for this PR:** an `@astrojs/sitemap`-generated sitemap instead of the static `sitemap.xml` copy; converting `compare.astro`'s large comparison table into a data-driven component instead of hand-written markup; extracting genuinely shared components (e.g. an icon set) into a package consumable by both `website/` and `src/`.

## Alternatives Considered

### Next.js / Nuxt
- **Pros:** Full-featured React/Vue meta-frameworks with strong ecosystems.
- **Cons:** Both assume a JS-app-first model (client-side routing, hydration by default) that's the wrong shape for a mostly-static marketing/docs site; heavier build output and dependency surface for no benefit here. Nuxt would at least match the app's Vue choice, but still carries SPA-oriented defaults (route middleware, server rendering modes) this site doesn't need.
- **Verdict:** Rejected — more framework than a four-page static site needs.

### Eleventy / plain Vite + vanilla templating
- **Pros:** Lighter than Astro; Eleventy in particular is a well-established static site generator.
- **Cons:** No first-class way to embed Vue components for the interactive pieces (gallery, sparkline, docs search) without hand-wiring hydration; would keep the vanilla-JS-IIFE pattern this ADR is trying to move away from, or require a second framework's templating language on top.
- **Verdict:** Rejected — Astro gets the same "ship static HTML by default" benefit while giving first-class Vue component support for islands.

### Keep hand-written static HTML, just deduplicate with a shell script / includes
- **Pros:** No new dependency at all; smallest possible change.
- **Cons:** Doesn't solve the actual problem (no component model, no type checking, vanilla JS for all interactivity) and shell-script HTML includes are a worse developer experience than a real templating/component system for very little savings.
- **Verdict:** Rejected — treats the symptom (duplicated header markup) without addressing the underlying lack of tooling parity with the app.

## References

- PRD-0007: openEHR Explorer — Product Website (GitHub Pages)
- [Astro documentation](https://docs.astro.build)
- [`@astrojs/vue`](https://docs.astro.build/en/guides/integrations-guide/vue/)
- `website/README.md` — local dev instructions
- `.github/workflows/pages.yml`
