# ADR-0022: Harmonized XML Viewer Component

**Date:** 2026-08-27
**Status:** Accepted
**Repo:** `openehr-explorer`
**Related:** [OEH-35](https://linear.app/platzh1rsch/issue/OEH-35/harmonize-xml-display-opt-xml-response-body-xml-dedupe-highlightxml), ADR-0021 (harmonized JSON viewer, the direct precedent for this ADR), ADR-0012 (CodeMirror 6 for AQL Editor)

---

## Context

ADR-0021 harmonized JSON display behind a reusable `JsonViewer.vue`, but explicitly left XML out of scope:

> `highlightXml()` (XML highlighting in `TemplateBrowser.vue`/`RequestInspector.vue`) ... [is] out of scope — none of these render raw JSON text.

XML has the same duplication problem JSON had before ADR-0021: a hand-rolled `formatXml()` + `highlightXml()` pair was duplicated near-verbatim in two places — `TemplateBrowser.vue`'s **OPT XML** tab and `RequestInspector.vue`'s **Response Body → XML** tab (auto-selected for `application/xml`/`text/xml` responses) — and the two copies had already drifted:

- `TemplateBrowser.vue` matched tag/attribute names with `/(&lt;\/?)([\w-:]+)([\s\S]*?)(&gt;)/g` — allowing `:` for namespaced elements (e.g. `xs:string`, common in OPT XML Schema output).
- `RequestInspector.vue` used `/(&lt;\/?)([\w-]+)([\s\S]*?)(&gt;)/g` — no `:` support, so a namespaced XML response inspected via the Request Inspector didn't highlight its tag names correctly even though the same markup rendered fine in the OPT XML tab.

Both were built the same way JSON's old highlighter was: escape the whole document to HTML entities, regex-match `&lt;...&gt;` patterns to wrap tag/attribute spans in `<span>`s, then render the result via `v-html`. Neither view had line numbers, consistent with the gap ADR-0021 closed for JSON.

## Decision

We will build one reusable component, **`src/components/XmlViewer.vue`**, backed by a pure, unit-tested tokenizer in **`src/lib/xml.ts`**, and adopt it at both existing XML display sites, retiring the duplicated `formatXml()`/`highlightXml()` implementations.

### Component design

- **Rendering approach:** `src/lib/xml.ts` tokenizes the raw XML text directly — not a stringify-then-regex-over-HTML-escaped-text approach. `parseXmlLines()` first breaks the document into lines at each direct `><` tag boundary (same strategy the old `formatXml()` used, so indentation/line-breaking behavior is unchanged), then tokenizes each line's content with a single regex pass into typed spans (`decl`, `comment`, `bracket`, `tag`, `attr-name`, `attr-value`, `punct`, `text`). Nesting depth is derived from each line's own tag shape (a lone opening tag indents deeper; a lone closing tag dedents; a self-closing tag or an inline open+text+close pair leaves depth unchanged) rather than re-matching a whole-line regex, which also incidentally fixes a latent bug in the original indent-detection regex for single-character tag names.
- **No `v-html`:** `XmlViewer.vue` renders each token via ordinary Vue text interpolation (`{{ }}`), the same as `JsonViewer.vue`. This removes the escape/re-render round-trip entirely — there is no HTML string to build, so there's nothing to double-escape and no `v-html` XSS-classed surface for this content the way the old implementation had (mitigated there by manual escaping, but still worth removing).
- **Fixing the namespaced-tag regression:** both the tag-name and attribute-name character classes in `src/lib/xml.ts` are `[\w:.-]+`, matching the more permissive of the two prior implementations. Since there is now exactly one implementation, this can't drift again.
- **Props/emits:** mirrors `JsonViewer.vue`'s shape for consistency at call sites — `xml: string`, `searchTerm?: string`, `currentMatchIndex?: number`, `showLineNumbers?: boolean` (default `true`), `showCopyButton?: boolean` (default `true`), and a `total-matches` emit. Search matching, current-match highlighting, and scroll-into-view reuse the same per-token span-matching approach as `JsonViewer.vue`, so `TemplateBrowser.vue`'s OPT XML tab keeps identical `SearchOverlay` next/previous/count behavior to before, just delegated to the component instead of hand-rolled per view.
- **Copy-to-clipboard:** one icon button docked top-right (same affordance/placement as `JsonViewer.vue`), copying the reconstructed pretty-printed text (`xmlLinesToText()`).
- **Line numbers:** a non-selectable gutter column, one row per tokenized line — consistent with `JsonViewer.vue` and this ADR's stated goal of feature parity between the two "raw code" viewers.
- **Theming:** reuses `--font-mono` and the app's border/surface/text `--color-*` variables for chrome (copy button, gutter); token colors keep the same hex values the old highlighter used (`#6495ed` tag, `#ffd93d` attr-name, `#6bff8e` attr-value, `#ff6b6b` declaration) since those were already shared between the two duplicated copies and this ADR isn't a restyle — only `JsonViewer.vue`'s string/number/boolean colors have a similar non-token precedent, so this isn't a new pattern.
- **No new dependency:** consistent with ADR-0021 and ADR-0012 — a hand-rolled tokenizer over the small, well-understood subset of XML this app needs to display (openEHR OPT XML and JSON/XML API responses) is a smaller and more auditable surface than pulling in a full XML parser or syntax-highlighting library, and this project has no other XML-parsing need that would justify one.
- **Collapsible sections:** not implemented in this pass. OEH-35 marked this a lower-priority nice-to-have relative to JSON's (OPT XML is read more often top-to-bottom than drilled into), and `XmlViewer.vue`'s flat per-line token model doesn't currently track parent/child element relationships the way `JsonViewer.vue`'s recursive value walk does — adding real collapse would mean layering a depth/subtree-boundary structure on top of the tokenizer, which is left as a future iteration if usage shows it's wanted.

### Migration scope

`XmlViewer` replaces the flat XML renderers at:

- `TemplateBrowser.vue` — OPT XML tab (keeps its `SearchOverlay` integration, now backed by `XmlViewer`'s `total-matches` emit and `currentMatchIndex` prop instead of the removed `highlightSearchInContent()`/`scrollToMatch()` helpers)
- `RequestInspector.vue` — Response Body → XML tab (drops the separate "Copy All" toolbar button in favor of `XmlViewer`'s built-in copy button, matching how the Raw tab's `JsonViewer` is already integrated there)

### Testing

Unlike JSON (which is parsed by the browser's own `JSON.parse`), there's no built-in XML parser this app already trusts for this purpose, so the tokenizer in `src/lib/xml.ts` is the actual place correctness bugs like the namespaced-tag regression can hide. It's split out from the Vue component specifically so it can be unit-tested directly (`src/lib/xml.test.ts`) — nesting/indentation, namespaced tag and attribute names, attribute tokenization, declarations, comments, self-closing tags, and the plain-text round-trip used for copy-to-clipboard.

## Consequences

### Positive

- One tokenizer/component to maintain for XML display instead of two divergent hand-rolled copies; the namespaced-tag regression (OEH-35's original trigger) can't reappear because there's only one regex to keep correct, and it's now covered by unit tests.
- Removes `v-html` from both XML display sites.
- Both XML views gain line numbers for the first time, closing the same JSON-vs-XML feature gap ADR-0021 closed for JSON generally.
- `RequestInspector.vue`'s XML tab gains a consistent copy-button affordance (matching `JsonViewer.vue`'s Raw tab) instead of a separate toolbar button.

### Negative

- The line-boundary/indentation strategy inherited from the original `formatXml()` (break at direct `><` adjacency) is a heuristic, not a real XML parse — pathological input (e.g. a `>` character inside unescaped attribute content, which isn't valid XML) can still misformat, same limitation the old code had. A real DOM-based parse (`DOMParser`) was considered and rejected for this pass — see Alternatives.
- No collapsible sections yet, unlike `JsonViewer.vue` — deferred per OEH-35's stated priority.

### Neutral

- Token color values are carried over unchanged from the old highlighter rather than being redefined as new `--color-*` tokens; this ADR doesn't attempt a visual restyle.

## Alternatives Considered

### Parse with `DOMParser` and walk the resulting DOM tree (mirroring `JsonViewer.vue`'s approach to parsed JSON)

- **Pros:** a real parse instead of a line-adjacency heuristic; would make collapsible sections straightforward to add later (real parent/child structure already available, the way `JsonViewer.vue` gets it from the parsed JSON value).
- **Cons:** `DOMParser`-produced trees lose whitespace-only text nodes and some formatting fidelity by default; reconstructing a faithful "pretty-printed, indented, byte-for-byte-plausible" rendering from a parsed DOM (rather than from the original text) is meaningfully more code than tokenizing the text directly, and OPT XML / API response bodies are the exact kind of content where a user might want to see the raw source's actual line breaks and attribute ordering, not a re-serialized reconstruction. A `DOMParser` parse failure (e.g. a truncated response body being inspected mid-debugging) would also need a fallback path back to plain text, adding another branch.
- **Verdict:** rejected for this pass — the line-tokenizer approach fixes the actual bug (namespaced tags) and adds the actual missing feature (line numbers) with less code and no risk of reformatting the source away from what the server actually sent. Worth reconsidering if collapsible XML sections become a real ask.

### Keep two copies, just deduplicate into a shared utility function without a component

- **Pros:** smallest possible change.
- **Cons:** same rationale ADR-0021 already rejected this for JSON — doesn't add line numbers consistently, and each call site would still need to wire up its own `<pre v-html>` rendering and search-highlighting by hand.
- **Verdict:** rejected — same reasoning as ADR-0021.

## References

- [OEH-35](https://linear.app/platzh1rsch/issue/OEH-35/harmonize-xml-display-opt-xml-response-body-xml-dedupe-highlightxml) — tracking issue
- ADR-0021 — Harmonized JSON Viewer Component (direct precedent for this ADR's structure and rationale)
- ADR-0012 — CodeMirror 6 for AQL Editor (precedent for avoiding heavyweight editor/viewer dependencies in this Tauri app)
