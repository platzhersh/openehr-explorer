# ADR-0023: Virtualized List Rendering for Large Data Views

**Date:** 2026-08-27
**Status:** Accepted
**Repo:** `openehr-explorer`
**Related:** ADR-0022 (harmonized XML viewer, the component fixed here), ADR-0021 (harmonized JSON viewer)

---

## Context

Two views became slow or fully unresponsive against real-world data volumes:

- **OPT XML tab** (`TemplateBrowser.vue`, via `XmlViewer.vue`): a real-world OPT XML document of ~12k lines rendered every line as DOM up front — one `<span>` per token, several tokens per line, so tens of thousands of nodes for a single document. The initial render blocked the main thread long enough to look frozen, and because nothing was ever removed from the DOM as the user scrolled, memory kept growing until the whole app (not just the XML tab) became unresponsive, forcing a restart. `RequestInspector.vue`'s XML tab shares the same component and the same failure mode for large response bodies.
- **AQL Runner → Stored Queries** (`AqlRunner.vue`): a CDR with hundreds/thousands of registered `STORED_QUERY` definitions rendered one `.saved-item` row per entry immediately on load, making the AQL Runner sluggish to open or switch servers on.

Both are the same underlying problem: a `v-for` over an unbounded server- or file-provided list, with no bound on the DOM node count.

## Decision

Add fixed-row-height list virtualization — render only the rows near the visible viewport (plus a small overscan buffer), backed by top/bottom spacer padding so the scrollbar still reflects the full list length — and apply it at both sites.

### Implementation

- **`src/lib/virtualList.ts`** — pure range math (`computeVirtualRange`, `virtualScrollOffset`), unit-tested in isolation (`src/lib/virtualList.test.ts`), mirroring how `src/lib/xml.ts` keeps `XmlViewer.vue`'s tokenizer separately testable from the Vue component.
- **`src/composables/useVirtualList.ts`** — wires the pure math to a real scroll container: listens for `scroll` (rAF-throttled) and container resize (`ResizeObserver`), and exposes `visibleItems`, `topPadding`/`bottomPadding`, and `scrollToIndex()`. The scroll container ref is created by the *caller* and passed in (`useVirtualList(items, containerRef, options)`), not created and returned by the composable — this mirrors `useCodeMirror.ts`'s existing `container: Ref<HTMLElement | null>` parameter, and avoids a `vue-tsc` `noUnusedLocals` false-positive: a ref created inside a composable and re-exported under a new destructured name isn't reliably recognized by vue-tsc's template-ref usage analysis, whereas a ref declared directly in the consuming `<script setup>` and bound via `ref="..."` always is.
- **`XmlViewer.vue`**: `.xv-scroll` is now a bounded, self-scrolling box (`max-height: 65vh; overflow: auto`) instead of growing to the full document height and relying on the page/panel to scroll — a virtualized window needs an actual viewport to window against. Rows are given a fixed `20px` height (previously a relative `line-height: 1.6`) so the row math is exact; the constant is documented at both the CSS declaration and the `ROW_HEIGHT` JS constant since they must stay in sync. The search-highlight "scroll to current match" behavior, previously done by `querySelector(".xv-match-current")?.scrollIntoView()`, now resolves the match's line index directly (via a reverse lookup built alongside the existing match-numbering pass) and calls the virtual list's own `scrollToIndex()` — the DOM node for an off-screen match may not exist yet, so scrolling can no longer depend on it already being rendered.
- **`AqlRunner.vue`**: the "Saved Queries (local)" and "Stored Queries (server)" lists each get their own virtualized, independently-scrolling region (`flex: 1; min-height: 0; overflow-y: auto` on `.saved-list`, replacing one shared `overflow-y: auto` on the whole `.saved-panel`) rather than one scrollbar for both sections combined — this also fixes a secondary UX issue where a long first section could scroll the second section's header out of view. `.saved-item` gets a fixed `34px` height (`box-sizing: border-box`) to match `QUERY_ROW_HEIGHT`.

### Why fixed-row-height, not a general/measuring virtualizer

Every row in both target lists is a known, constant height by construction (a tokenized XML line never wraps — `white-space: pre` plus horizontal scroll — and a stored/saved query row is one line of text). A fixed-height virtualizer is a few dozen lines of arithmetic; a variable-height virtualizer needs to measure rendered rows (or accept estimation error) and is meaningfully more code for a case this codebase doesn't have yet. If a future list needs variable-height rows, `src/lib/virtualList.ts` is the place to extend, not a reason to add a dependency now.

### No new dependency

Consistent with ADR-0021/ADR-0022/ADR-0012's stated preference for small hand-rolled implementations over pulling in a library for a narrow, well-understood need — the same reasoning applies here: fixed-row-height windowing over a scroll container is a small, auditable amount of code.

## Consequences

### Positive

- Both views now render a bounded number of DOM nodes regardless of list/document size — opening a 12k-line OPT XML document or a stored-queries list in the thousands no longer blocks the main thread or grows memory as the user scrolls.
- `useVirtualList`/`virtualList.ts` are generic over the row content (`<T>`), so a future large list in this app can reuse them directly.
- `RequestInspector.vue`'s XML tab gets the same fix for free, since it shares `XmlViewer.vue`.

### Negative

- `XmlViewer.vue` changes from page-scrolling (the OPT XML tab scrolled along with the rest of `TemplateBrowser.vue`'s right panel) to a bounded, self-scrolling box. This is a deliberate, necessary trade-off — a virtualized window needs a real viewport — but it is a visible behavior change from before this ADR.
- Both call sites now need their row height kept in sync between CSS and the JS constant that assumes it (`ROW_HEIGHT` / `QUERY_ROW_HEIGHT`); a future style change to `.xv-line` or `.saved-item` that isn't mirrored in the matching constant will silently misalign virtualization (rows rendered at the wrong scroll offset). Documented at both declaration sites as the mitigation; a runtime-measured row height was considered (see Alternatives) and rejected for the added complexity.

### Neutral

- `AqlRunner.vue`'s "Saved Queries (local)" list is virtualized even though it's typically small (user-curated, local-file-backed) — done for consistency and because the composable makes it free, not because it was independently reported as slow.

## Alternatives Considered

### Measure actual row height at runtime (e.g. via `ResizeObserver` on a rendered sample row) instead of a hardcoded constant

- **Pros:** removes the CSS/JS sync-drift risk noted above; would also support rows whose height varies with theme/font settings.
- **Cons:** neither target row's height varies by content (both are fixed single-line rows by construction), so this would add real complexity — measuring before the first paint, handling a font/zoom change mid-session — for a case that doesn't exist yet.
- **Verdict:** rejected for this pass. Worth revisiting if a future virtualized list has rows whose height isn't fully controlled by this app's own CSS.

### Add a filter/search box to the Stored Queries list instead of (or in addition to) virtualizing

- **Pros:** also a genuinely useful feature (`TemplateBrowser.vue`'s template list already has this), and reduces rendered rows without new infrastructure.
- **Cons:** doesn't fix the underlying problem — opening the view at all, before typing a single filter character, was the reported slowness — and a filter is an orthogonal usability feature, not a performance fix.
- **Verdict:** out of scope for this ADR; left as a natural follow-up feature request, independent of the perf fix here.

### Pull in a virtualization library (e.g. `vue-virtual-scroller`)

- **Verdict:** rejected for the same reason ADR-0021/ADR-0022 rejected external libraries for JSON/XML rendering — this app's two target lists are simple fixed-row-height cases, well within what a small hand-rolled implementation covers.

## References

- ADR-0022 — Harmonized XML Viewer Component (the component this ADR fixes the performance of)
- ADR-0021 — Harmonized JSON Viewer Component (precedent for splitting pure logic from the Vue component for testability)
- ADR-0012 — CodeMirror 6 for AQL Editor (precedent for `useCodeMirror.ts`'s caller-owned container ref pattern, reused here)
