# openEHR Explorer — Website

The public marketing/docs site (landing page, documentation, tool comparison, brand kit), built with [Astro](https://astro.build) + Vue 3. See [`docs/adr/ADR-0024-astro-vue-marketing-site.md`](../docs/adr/ADR-0024-astro-vue-marketing-site.md) for why this exists and how it relates to the desktop app in `../src/`.

This is a separate npm project from the root of the repo (its own `package.json`, `node_modules`, lockfile) — it is not part of the Tauri app build.

## Commands

Run from this directory (`website/`):

```bash
npm install       # install dependencies
npm run dev       # start the local dev server (http://localhost:4321)
npm run build     # type-check (astro check) + build to dist/
npm run preview   # preview the production build locally
npm run check     # type-check only
```

## Structure

```
website/
├── public/              static files served as-is (favicons, screenshots, robots.txt, …)
├── src/
│   ├── components/      shared Astro components (header, footer, SEO tags, …)
│   │   └── vue/          interactive Vue islands (gallery, sparkline, docs search, …)
│   ├── layouts/          Layout.astro — shared <head> boilerplate
│   ├── pages/            index.astro, docs.astro, compare.astro, brand-kit.astro
│   └── styles/           tokens.css (design tokens) + base.css (reset)
└── astro.config.mjs
```

Pages build to `docs.html`, `compare.html`, `brand-kit.html` (not Astro's default `/docs/`-style directory routing) so existing links and the sitemap keep working — see `build.format` in `astro.config.mjs`.

## Design tokens

`src/styles/tokens.css` is the single source of truth for the site's colour palette, mirroring the app's own CSS custom properties in `../src/App.vue` and the canonical table in `docs/prd/PRD-0007-openehr-explorer-product-website.md`. Change it there, not per-page.

## Deployment

`.github/workflows/pages.yml` builds this project on every change and (once the repository's Pages source is switched to "GitHub Actions" in Settings → Pages) publishes `dist/` via `actions/deploy-pages`.
