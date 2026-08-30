# ADR-0025: CodeBlock Copy-to-Clipboard as a Vue Component

**Date:** 2026-08-30
**Status:** Accepted
**Deciders:** Development Team
**Related:** ADR-0024 (Astro + Vue for the Marketing/Docs Site), PR #168 (`docs/assets/code-copy.{css,js}`, the static-site original this migrates)

## Context

While the Astro + Vue rebuild of the marketing/docs site (ADR-0024) was in review, `main` gained a copy-to-clipboard affordance for the install command snippets on the still-live static site: `docs/assets/code-copy.css` + `docs/assets/code-copy.js`, wired into nine `<div class="code-block">` blocks across `docs/index.html` and `docs/docs.html` (PR #168). Each opted-in block carried two hand-written, parallel representations of the same snippet:

```html
<div class="code-block" data-copy="curl -fsSL ...\nsudo apt update &amp;&amp; sudo apt install open-ehr-explorer">
  <span class="code-line">curl -fsSL ...</span>
  <span class="code-line">sudo apt update &amp;&amp; sudo apt install open-ehr-explorer</span>
</div>
```

The `data-copy` attribute (what the button actually copies, with literal `\n` line breaks and HTML-escaped quotes) has to be kept byte-for-byte in sync with the visible `<span class="code-line">`/`<span class="code-comment">` markup by hand — the exact kind of duplication ADR-0024 already flagged as the site's core problem (it's how `compare.html`'s `--text-muted` token had silently drifted). Porting this feature into the Astro rebuild by copying the vanilla-JS/CSS files verbatim (as `public/assets/code-copy.{css,js}` + a `<script>` tag, the way the rest of `docs/assets/` was carried over) would import that exact duplication risk into the new site on day one.

## Decision

We will port this as a Vue island, **`website/src/components/vue/CodeBlock.vue`**, instead of copying the static JS/CSS files.

- **Single source per snippet:** the component takes one `segments: ({ comment: string } | { cmd: string })[]` prop. It renders the visible `<span class="code-line">`/`<span class="code-comment">` markup **and** derives the exact clipboard text from that same array (grouping rule: join with `\n`, insert an extra blank line before every comment after the first — the same grouping the original hand-written `data-copy` strings used). There is one array to get right per snippet instead of a rendered-markup/data-copy pair that can drift apart.
- **Self-contained styling:** `CodeBlock.vue`'s `<style scoped>` includes the *entire* `.code-block` appearance (background, border, font, spacing), not just the copy-button-specific rules from `code-copy.css`. Astro's own scoped CSS (`.code-block` rules written directly in `index.astro`/`docs.astro`) attaches a `data-astro-cid-*` attribute selector that only matches elements the `.astro` file's own template renders directly — it would not reach elements rendered by a child Vue component's template. A component that needs to look right has to carry its own complete styling rather than depending on the host page's scoped rules.
- **Same clipboard mechanics as the original:** `navigator.clipboard.writeText()` with a `document.execCommand('copy')` fallback for browsers/contexts without the async Clipboard API, and the same 1.8s "copied" checkmark flash — ported behavior, not a redesign.
- **Scope unchanged from PR #168:** wired into the same nine blocks (3 in `index.astro`'s Quick Start, 6 in `docs.astro`'s Installation/Troubleshooting) and no others — the SELECT query example, search-syntax examples, and the `docker-compose up -d` troubleshooting block stay plain, uncopyable `<div class="code-block">`s, matching upstream's own scope decision.

## Consequences

### Positive
- A snippet's visible text and its clipboard text cannot drift apart — they're the same data, rendered two ways, instead of two hand-maintained representations.
- No new escaping/encoding to get right by hand: no more literal `\n` and HTML-escaped `&quot;`/`&amp;` inside an HTML attribute value; `segments` is a plain TypeScript array literal.
- Consistent with ADR-0024's stated direction (Vue islands for the site's interactive pieces, matching the app's own Vue 3 usage) rather than accreting more vanilla-JS/CSS asset pairs alongside them.

### Negative
- `CodeBlock.vue`'s CSS necessarily repeats a few rules (`.code-block` background/border/font/padding) that also exist, slightly differently, in `index.astro`/`docs.astro`'s own scoped styles for the site's other (non-copy) code blocks — an unavoidable consequence of Astro/Vue's per-component style scoping, not the same drift problem this ADR is solving (there is no second `data-copy`-shaped mirror of the same content to fall out of sync with here, only the same handful of CSS property values appearing twice).

### Neutral
- `docs/assets/code-copy.{css,js}` remain in place, unmodified, in the old static `docs/` site per ADR-0024's migration plan — they're retired together with the rest of `docs/`'s static HTML at cutover, not touched separately.

## Alternatives Considered

### Copy `docs/assets/code-copy.{css,js}` into `website/public/` verbatim, wire with a plain `<script>` tag
- **Pros:** smallest possible diff; zero behavioral risk since it's the exact already-shipped code.
- **Cons:** re-imports the hand-synced `data-copy`/visible-markup duplication this ADR exists to avoid, on the very site whose founding rationale (ADR-0024) is "stop hand-syncing duplicated content across pages." Also the one clearly vanilla-JS-DOM-query piece of interactivity left inconsistent with every other interactive piece of the site, which is otherwise Vue islands.
- **Verdict:** rejected — trades a real, demonstrated duplication risk for a marginally smaller diff.

### A generic "shell snippet" data structure shared with a future MDX/content-collection docs layer
- **Pros:** could let non-technical contributors add copyable snippets via frontmatter/content data instead of JSX-like `segments` arrays in `.astro` files.
- **Cons:** no such content layer exists yet, and speculatively designing one now is out of scope for porting a single already-shipped feature.
- **Verdict:** rejected as premature; revisit if/when the docs page grows a content-collection layer.

## References

- ADR-0024 — Astro + Vue for the Marketing/Docs Site
- PR #168 — `doc(website): add copy-to-clipboard and clearer line breaks to install snippets` (the static-site original)
- `website/src/components/vue/CodeBlock.vue`
- `docs/assets/code-copy.css`, `docs/assets/code-copy.js` — the static-site original, retired at `docs/` cutover per ADR-0024
