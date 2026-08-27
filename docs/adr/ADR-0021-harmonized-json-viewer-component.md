# ADR-0021: Harmonized JSON Viewer Component

**Date:** 2026-08-27
**Status:** Accepted
**Repo:** `openehr-explorer`
**Related:** [OEH-33](https://linear.app/platzh1rsch/issue/OEH-33/harmonize-json-display-across-the-app-with-a-reusable-jsonviewer), ADR-0012 (CodeMirror 6 for AQL Editor)

---

## Context

The app renders raw JSON to the user in at least nine places: `CompositionViewer.vue` (JSON tab, FLAT tab), `EhrBrowser.vue` (EHR Detail JSON tab), `TemplateBrowser.vue` (Web Template JSON panel), `AqlRunner.vue` (result-cell complex-value detail), `CompositionForm.vue` (Preview JSON panel, Request/Response debug panel), and `RequestInspector.vue` (Request/Response Raw tabs). Each of these grew its own implementation independently, and none combines all of syntax highlighting, copy-to-clipboard, line numbers, and collapsible sections:

| Location | Highlight | Copy | Line #s | Collapse |
|---|:---:|:---:|:---:|:---:|
| `CompositionViewer.vue` — JSON / FLAT tabs | ✅ own regex highlighter | ✅ | ❌ | ❌ |
| `EhrBrowser.vue` — EHR Detail JSON tab | ❌ plain `<pre>` | ✅ | ❌ | ❌ |
| `TemplateBrowser.vue` — Web Template JSON panel | ✅ duplicate regex highlighter | ✅ | ❌ | ❌ |
| `AqlRunner.vue` — result-cell detail | ❌ | ❌ | ❌ | ✅ native `<details>` |
| `CompositionForm.vue` — debug panel | ❌ | ❌ | ❌ | ❌ |
| `CompositionForm.vue` — Preview JSON panel | ❌ | ✅ | ❌ | ❌ |
| `RequestInspector.vue` — Raw tabs | ❌ | partial (Response only) | ❌ | ❌ |
| `RequestInspector.vue` — Tree tabs (`JsonTreeNode.vue`) | N/A interactive tree | partial (per-field) | — | ✅ |

Two consequences of this drift:

1. **Duplicated, diverging highlighter.** `CompositionViewer.vue` and `TemplateBrowser.vue` each carry their own near-verbatim copy of a hand-rolled `highlightJson()` regex highlighter (keys/strings/numbers/booleans/null wrapped in `<span>`s), with different hard-coded hex colors (`#79c0ff` vs `#6495ed`) instead of the app's shared `--color-*` CSS custom properties defined in `src/App.vue`. A third, parallel `highlightXml()` exists for XML (`TemplateBrowser.vue`, `RequestInspector.vue`) — out of scope here, but built on the same pattern.
2. **The best existing building block is stranded.** `src/components/JsonTreeNode.vue` (used only inside `RequestInspector.vue`'s Tree tabs) is a genuinely well-built recursive, collapsible, searchable, openEHR-aware tree renderer with `_type`/`archetype_node_id`/`DV_QUANTITY` awareness. Every other JSON display in the app reimplements a flatter, less capable view instead of reusing it, because it wasn't built as a general-purpose component.

No JSON-viewer or generic syntax-highlighting dependency (e.g. `highlight.js`, `prismjs`, `shiki`, `vue-json-pretty`) is currently installed. The only editor/highlighting dependency in `package.json` is CodeMirror 6 (`codemirror`, `@codemirror/*`), used solely by `AqlEditor.vue` for the AQL query editor per ADR-0012. That ADR is directly relevant precedent: an earlier attempt to use Monaco Editor for the same Tauri app caused the AQL Runner page to freeze on load, because Monaco's synchronous init (~4MB bundle, web worker setup) blocked the WebView's main thread. Any solution here must avoid repeating that mistake — this is a much smaller, more frequently-rendered UI surface (JSON panes appear on nearly every page) than the single AQL editor instance, so bundle weight and init cost matter even more.

## Decision

We will build one reusable component, **`src/components/JsonViewer.vue`**, and adopt it at all flat/raw JSON display sites, retiring the duplicated `highlightJson()` implementations.

### Component design

- **Rendering approach:** hand-rolled, not a third-party JSON-viewer library. `JsonViewer` recursively walks the parsed value (not a string re-highlighted with regex) and renders each JSON token as a `<span>` with a type-based class (`.jv-key`, `.jv-string`, `.jv-number`, `.jv-boolean`, `.jv-null`, `.jv-punctuation`), replacing the current stringify-then-regex-highlight approach. Walking the parsed value directly (rather than re-parsing pretty-printed text with regex) is what makes accurate per-node collapse boundaries possible, and sidesteps the correctness edge cases regex highlighting has (e.g. matching `"key"` inside a string value).
- **Why hand-rolled over a dependency:** the four required capabilities (highlight, copy, line numbers, collapse) are individually simple to implement directly against the app's own theme variables, and a hand-rolled ~200-300 line SFC avoids adding bundle weight for a JSON-specific concern the app already partially solved twice. This mirrors the ADR-0012 lesson (avoid heavyweight editor/viewer dependencies in the Tauri WebView) at a much smaller scale — a JSON tree walker has none of Monaco's async/web-worker/bundle-size problems, so there's nothing here that is Tauri-load-bearing enough to justify a dependency the way CodeMirror was for the AQL editor's autocomplete and language tooling.
- **Props:** `value: unknown` (accepts an already-parsed object/array/primitive, so callers pass data directly instead of pre-stringifying — this also fixes `EhrBrowser.vue`, which today has no highlighting at all because it renders a plain string), `searchTerm?: string` (reuses the existing search-and-highlight behavior already present in `CompositionViewer.vue`/`TemplateBrowser.vue` and `JsonTreeNode.vue`'s tree search), `defaultCollapsedDepth?: number` (default `Infinity`, i.e. fully expanded — matches current behavior everywhere else; `RequestInspector.vue`'s Tree tab currently auto-expands to depth 3, so that call site alone will pass `defaultCollapsedDepth={3}`), `lineNumbers?: boolean` (default `true`).
- **Copy-to-clipboard:** one icon button docked at the top-right of the component (not per-field), copying `JSON.stringify(value, null, 2)` via `navigator.clipboard`, replacing the various "Copy JSON"/"Copy All" text buttons scattered per view so the affordance is always in the same place relative to the content it copies.
- **Collapsible sections:** each object/array node gets a click target on its opening brace/bracket (▸/▾ disclosure triangle, consistent with `JsonTreeNode.vue`'s existing interaction) that toggles a collapsed placeholder (`{…}` / `[…]`, with an item count).
- **Line numbers:** rendered as a non-selectable gutter column, computed from the flattened, pretty-printed line count — consistent with a normal code viewer and with the `AqlEditor.vue` CodeMirror instance's own gutter, so the two "code-like" surfaces in the app feel consistent.
- **Theming:** token colors and layout use the app's existing `--color-*` custom properties and `--font-mono` from `src/App.vue`'s `:root` block — not new hard-coded hex values — resolving the color drift between the two existing `highlightJson()` copies.

### Migration scope

`JsonViewer` replaces the flat/raw JSON renderers at:
- `CompositionViewer.vue` (JSON tab, FLAT tab)
- `EhrBrowser.vue` (EHR Detail JSON tab)
- `TemplateBrowser.vue` (Web Template JSON panel)
- `AqlRunner.vue` (result-cell complex-value detail, replacing the native `<details>`/plain `<pre>`)
- `CompositionForm.vue` (Preview JSON panel, Request/Response debug panel)
- `RequestInspector.vue` (Request Body / Response Body Raw tabs)

`highlightXml()` (XML highlighting in `TemplateBrowser.vue`/`RequestInspector.vue`) and the FLAT-path/table views are out of scope — they aren't raw JSON.

### `JsonTreeNode.vue` / Tree tabs: kept as-is, not merged

`RequestInspector.vue`'s Tree tabs keep using `JsonTreeNode.vue` rather than being migrated to `JsonViewer`. The two components serve genuinely different jobs: `JsonTreeNode` is an *interactive inspection* tree purpose-built for debugging openEHR wire payloads (per-field copy on `uid`/`object_id`, `_type` pills, `DV_QUANTITY` magnitude+unit badges), while `JsonViewer` is a *read/copy/scan* code-style display for "here's the JSON, look at or grab all of it." Folding openEHR-specific field awareness into the general-purpose `JsonViewer` would couple a component used app-wide to Request Inspector-specific concerns. `JsonViewer`'s collapse/highlight/line-number mechanics may be extracted into shared composables later if `JsonTreeNode` is refactored to reuse them, but that refactor is not part of this ADR's scope.

## Consequences

### Positive
- One component to maintain for JSON display instead of at least three divergent hand-rolled implementations; new views get highlight+copy+lines+collapse for free by using `JsonViewer`.
- Fixes real current gaps: `EhrBrowser.vue`'s EHR Detail JSON tab gains highlighting for the first time; `AqlRunner.vue`'s cell detail and `CompositionForm.vue`'s debug/preview panels gain a copy button for the first time; every flat JSON view gains line numbers and collapse for the first time.
- Removes the color-scheme drift between `CompositionViewer.vue` and `TemplateBrowser.vue` by centralizing on the app's `--color-*` theme variables.
- No new runtime dependency, consistent with this project's general preference (per ADR-0012 and ADR-0007) for a small, auditable, pinned dependency surface.

### Negative
- Upfront cost of writing and testing a non-trivial recursive rendering component (token spans, collapse state, line-number sync, search highlighting) rather than pulling in an off-the-shelf viewer.
- Six call sites need migration and manual visual regression checking (dark theme, long/deeply-nested payloads, search-highlight interaction).
- A hand-rolled highlighter needs to keep handling JSON edge cases correctly (escaped characters in strings, `-0`/exponent number formats, deeply nested structures) that a mature library would already cover; this is a maintenance surface the team is taking on deliberately at a scale (single JSON pane) where the risk is judged low.

### Neutral
- `JsonTreeNode.vue` remains a separate, RequestInspector-specific component; this ADR does not unify it with `JsonViewer`, only documents that the split is intentional.

## Alternatives Considered

### Adopt a third-party JSON viewer library (e.g. `vue-json-pretty`, `vue-json-viewer`)
- **Pros:** less code to write and maintain; batteries-included collapse/search/theming.
- **Cons:** another pinned dependency to track (per ADR-0007's exact-pinning policy) for functionality that is not large; most such libraries ship their own CSS/theme system that would need overriding to match `--color-*`; less control over the exact copy-button placement and line-number gutter behavior the app wants.
- **Verdict:** rejected — the feature set needed is narrow enough that a small in-house component is lower total cost than integrating and re-theming a library, and keeps the dependency surface minimal as this project prefers.

### Extend `JsonTreeNode.vue` into the general-purpose component
- **Pros:** reuses the most capable existing code; single component for both interactive inspection and flat display.
- **Cons:** `JsonTreeNode` is deeply openEHR-aware (RM `_type`, `archetype_node_id`, `DV_QUANTITY`) by design for the Request Inspector's debugging use case; generalizing it risks either leaking that awareness into unrelated views (e.g. an AQL result cell that isn't openEHR wire-format shaped) or a large refactor to strip and re-layer that behavior.
- **Verdict:** rejected for this pass; left as a possible future refactor once `JsonViewer` exists and any shared primitives (collapse state, token coloring) become obvious.

### Keep the regex-based `highlightJson()` approach, just deduplicate it into a shared utility function
- **Pros:** smallest possible change; no new component architecture.
- **Cons:** doesn't add copy-button/line-numbers/collapse consistently — those still need be implemented per view; regex-over-pretty-printed-string highlighting has correctness edge cases (matching inside string values) and can't cleanly support per-node collapse (no structural boundaries once stringified).
- **Verdict:** rejected — solves only the duplication complaint, not the missing-capabilities complaint that is the actual ask.

## References

- [OEH-33](https://linear.app/platzh1rsch/issue/OEH-33/harmonize-json-display-across-the-app-with-a-reusable-jsonviewer) — tracking issue with full call-site checklist
- ADR-0012 — CodeMirror 6 for AQL Editor (precedent for avoiding heavyweight editor/viewer dependencies in this Tauri app)
- ADR-0007 — Pinned dependency versions
