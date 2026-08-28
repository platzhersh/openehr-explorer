# ADR-0023: Virtualized List Rendering for Large Data Views

**Date:** 2026-08-27
**Status:** Accepted
**Repo:** `openehr-explorer`
**Related:** ADR-0022 (harmonized XML viewer, the component fixed here), ADR-0021 (harmonized JSON viewer, also fixed here)

---

## Context

Three views became slow or fully unresponsive against real-world data volumes:

- **OPT XML tab** (`TemplateBrowser.vue`, via `XmlViewer.vue`): a real-world OPT XML document of ~12k lines rendered every line as DOM up front — one `<span>` per token, several tokens per line, so tens of thousands of nodes for a single document. The initial render blocked the main thread long enough to look frozen, and because nothing was ever removed from the DOM as the user scrolled, memory kept growing until the whole app (not just the XML tab) became unresponsive, forcing a restart. `RequestInspector.vue`'s XML tab shares the same component and the same failure mode for large response bodies.
- **Web Template JSON tab** (`TemplateBrowser.vue`, via `JsonViewer.vue`) and **Composition JSON/FLAT tabs** (`CompositionViewer.vue`, same component): the same problem, one row of DOM per visible (uncollapsed) line, for a large Web Template or composition. Less severe than the XML case in practice (`JsonViewer.vue` supports per-node collapse, so a user can manually shrink what's rendered), but the same unbounded-DOM root cause, and collapsing doesn't help the *first* render of a large document before anything's been collapsed.
- **AQL Runner → Stored Queries** (`AqlRunner.vue`): a CDR with hundreds/thousands of registered `STORED_QUERY` definitions rendered one `.saved-item` row per entry immediately on load, making the AQL Runner sluggish to open or switch servers on.

All three are the same underlying problem: a `v-for` over an unbounded server- or file-provided list, with no bound on the DOM node count.

## Decision

Add fixed-row-height list virtualization — render only the rows near the visible viewport (plus a small overscan buffer), backed by top/bottom spacer padding so the scrollbar still reflects the full list length — and apply it at all three sites.

### Implementation

- **`src/lib/virtualList.ts`** — pure range math (`computeVirtualRange`, `virtualScrollOffset`), unit-tested in isolation (`src/lib/virtualList.test.ts`), mirroring how `src/lib/xml.ts` keeps `XmlViewer.vue`'s tokenizer separately testable from the Vue component.
- **`src/composables/useVirtualList.ts`** — wires the pure math to a real scroll container: listens for `scroll` (rAF-throttled) and container resize (`ResizeObserver`), and exposes `visibleItems`, `topPadding`/`bottomPadding`, and `scrollToIndex()`. The scroll container ref is created by the *caller* and passed in (`useVirtualList(items, containerRef, options)`), not created and returned by the composable — this mirrors `useCodeMirror.ts`'s existing `container: Ref<HTMLElement | null>` parameter, and avoids a `vue-tsc` `noUnusedLocals` false-positive: a ref created inside a composable and re-exported under a new destructured name isn't reliably recognized by vue-tsc's template-ref usage analysis, whereas a ref declared directly in the consuming `<script setup>` and bound via `ref="..."` always is.
- **`XmlViewer.vue`**: `.xml-viewer`/`.xv-scroll` are now a flex column (`flex: 1; min-height: 0; overflow: auto`) that fills whatever height its caller's layout gives it, instead of growing to the full document height and relying on the page/panel to scroll — a virtualized window needs an actual viewport to window against. An earlier version of this component capped `.xv-scroll` at a fixed `max-height: 65vh`; that clipped a taller panel short and left dead space below the viewer in a shorter one; see the two call-site changes below for how each now supplies a real bounded height instead. Rows are given a fixed `20px` height (previously a relative `line-height: 1.6`) so the row math is exact; the constant is documented at both the CSS declaration and the `ROW_HEIGHT` JS constant since they must stay in sync. Rows also use `white-space: nowrap` rather than `pre`: free-text OPT content (e.g. a `<purpose>` description) can contain literal embedded newlines in the source XML, which `pre` renders as real line breaks, overflowing a row's fixed height and bleeding into the row below it — `nowrap` collapses embedded whitespace runs to a single space for display while `xmlLinesToText()` (copy-to-clipboard) still uses the untouched token text. The search-highlight "scroll to current match" behavior, previously done by `querySelector(".xv-match-current")?.scrollIntoView()`, now resolves the match's line index directly (via a reverse lookup built alongside the existing match-numbering pass) and calls the virtual list's own `scrollToIndex()` — the DOM node for an off-screen match may not exist yet, so scrolling can no longer depend on it already being rendered.
- **`TemplateBrowser.vue`**: while the OPT XML tab is active, `.panel-right` (normally just a big `overflow-y: auto` box scrolling all four tabs' content together) switches to a non-scrolling flex column (`.panel-right--xml`) so `.xml-view` can be handed the exact remaining height below the header as a real `flex: 1; min-height: 0` box — which `XmlViewer.vue` then fills. The other three tabs are untouched. This is a class toggle keyed off `activeTab`, not a permanent layout change, specifically to avoid touching the tree/JSON/FLAT tabs' scroll behavior in the same pass.
- **`RequestInspector.vue`**: `.xml-container` already capped itself at `max-height: 400px` (mirroring the sibling `.tree-container`/`.tree-scroll` pattern for the Tree body-view tab) — but it did so by scrolling *itself* (`overflow-y: auto`), which, now that `XmlViewer.vue` fills rather than caps its own height, would let the (potentially huge) document grow unbounded inside it before that outer scrollbar ever kicked in, defeating virtualization. Fixed by making `.xml-container` a flex column (`overflow: hidden`) so the bounded `max-height: 400px` box is what `XmlViewer.vue` fills, and its own inner `.xv-scroll` is what actually scrolls.
- **`AqlRunner.vue`**: the "Saved Queries (local)" and "Stored Queries (server)" lists each get their own virtualized, independently-scrolling region (`flex: 1; min-height: 0; overflow-y: auto` on `.saved-list`, replacing one shared `overflow-y: auto` on the whole `.saved-panel`) rather than one scrollbar for both sections combined — this also fixes a secondary UX issue where a long first section could scroll the second section's header out of view. `.saved-item` gets a fixed `34px` height (`box-sizing: border-box`) to match `QUERY_ROW_HEIGHT`.
- **`JsonViewer.vue`**: virtualizes over `visibleLines` — the *already-collapse-filtered* array the component computes today — so per-node collapse/expand keeps working exactly as before; the virtual list just windows over however many lines are currently visible. Gets the identical `.jv-scroll` flex-fill treatment and `white-space: nowrap` row fix `XmlViewer.vue` got (see above), and the same `querySelector(".jv-match-current")` → index-based `scrollToIndex()` change to the search "scroll to current match" behavior — except the reverse lookup here is keyed by `line.id` (JSON lines already carry a stable id for collapse tracking) rather than XmlViewer's positional line index. `TemplateBrowser.vue`'s Web Template JSON tab and `CompositionViewer.vue`'s JSON/FLAT tabs each get the same bounded-flex-ancestor treatment `TemplateBrowser.vue`'s OPT XML tab got (`.panel-right--bounded` / `.main-content--bounded`, toggled by `activeTab`). `JsonViewer.vue`'s other call sites (`RequestInspector.vue`, `CompositionForm.vue`, `EhrBrowser.vue`, and the AQL Runner's per-cell popover) are untouched: none of them previously gave `JsonViewer.vue` a self-imposed cap the way `RequestInspector.vue`'s `.xml-container` did, so they keep behaving exactly as before (content-hugging inside whatever ancestor already scrolled them) — virtualized under the hood as a free correctness improvement, but not part of this pass's call-site layout work since those values are typically small.

### Why fixed-row-height, not a general/measuring virtualizer

Every row in both target lists is a known, constant height by construction (a tokenized XML line never wraps — `white-space: nowrap` plus horizontal scroll — and a stored/saved query row is one line of text). A fixed-height virtualizer is a few dozen lines of arithmetic; a variable-height virtualizer needs to measure rendered rows (or accept estimation error) and is meaningfully more code for a case this codebase doesn't have yet. If a future list needs variable-height rows, `src/lib/virtualList.ts` is the place to extend, not a reason to add a dependency now.

### No new dependency

Consistent with ADR-0021/ADR-0022/ADR-0012's stated preference for small hand-rolled implementations over pulling in a library for a narrow, well-understood need — the same reasoning applies here: fixed-row-height windowing over a scroll container is a small, auditable amount of code.

## Consequences

### Positive

- All three views now render a bounded number of DOM nodes regardless of list/document size — opening a 12k-line OPT XML document, a large Web Template/composition JSON, or a stored-queries list in the thousands no longer blocks the main thread or grows memory as the user scrolls.
- `useVirtualList`/`virtualList.ts` are generic over the row content (`<T>`), so a future large list in this app can reuse them directly — as `JsonViewer.vue` already does, reusing the exact same composable `XmlViewer.vue` introduced.
- `RequestInspector.vue`'s XML tab and `JsonViewer.vue`'s other call sites (request/response payload previews, EHR detail JSON, AQL result cell popovers) get the same fix for free, since they share the two virtualized components.

### Negative

- `XmlViewer.vue` and `JsonViewer.vue` change from page-scrolling (the OPT XML/Web Template JSON tabs scrolled along with the rest of their panel) to a bounded, self-scrolling box wherever their call site now supplies one. This is a deliberate, necessary trade-off — a virtualized window needs a real viewport — but it is a visible behavior change from before this ADR.
- Because both components now fill rather than cap their own height, every call site that wants the fill-available-height behavior is responsible for handing it a genuinely bounded ancestor (a real flex height, or its own `max-height`/`overflow`) — there's no longer a component-level fallback that bounds it on its own. The four call sites that need this do it (`TemplateBrowser.vue`'s `.panel-right--bounded`, `CompositionViewer.vue`'s `.main-content--bounded`, `RequestInspector.vue`'s pre-existing `.xml-container` cap), but a future call site that drops either component into an unbounded container without doing the same would silently reintroduce the original freeze/memory-growth bug. Mitigated by the comments left at every call site and at each component's own scroll-container declaration; a component-owned `max-height` fallback was considered and rejected (see Alternatives) because any fixed value is exactly the "guessed viewport size" problem this pass replaced.
- All three virtualized rows need their height kept in sync between CSS and the JS constant that assumes it (`ROW_HEIGHT` in both `XmlViewer.vue`/`JsonViewer.vue`, `QUERY_ROW_HEIGHT` in `AqlRunner.vue`); a future style change to `.xv-line`/`.jv-line`/`.saved-item` that isn't mirrored in the matching constant will silently misalign virtualization (rows rendered at the wrong scroll offset). Documented at both declaration sites as the mitigation; a runtime-measured row height was considered (see Alternatives) and rejected for the added complexity.
- `white-space: nowrap` is a display-only normalization of embedded whitespace runs (including literal newlines) within a single row — for `XmlViewer.vue` this can affect a free-text OPT XML element (e.g. `<purpose>`) with intentional multiple-space formatting, rendering it collapsed to one space. Copy-to-clipboard is unaffected in both components (each serializes the untouched token text/parsed value, not the rendered DOM). `JsonViewer.vue`'s string tokens are `JSON.stringify()`'d already, so real newlines were already escaped there and this change is precautionary rather than fixing an observed bug the way it did for `XmlViewer.vue`.

### Neutral

- `AqlRunner.vue`'s "Saved Queries (local)" list is virtualized even though it's typically small (user-curated, local-file-backed) — done for consistency and because the composable makes it free, not because it was independently reported as slow.
- `JsonViewer.vue`'s other call sites (`RequestInspector.vue`'s request/response JSON, `CompositionForm.vue`'s payload previews, `EhrBrowser.vue`'s EHR detail JSON, `AqlRunner.vue`'s per-cell popover) get virtualization but not the fill-available-height layout change, since none of them were reported slow and their values are typically small — see the `JsonViewer.vue` implementation note above.

## Alternatives Considered

### Keep a component-owned `max-height` fallback on `.xv-scroll`/`.jv-scroll` (e.g. `65vh`) instead of requiring every call site to supply a bounded ancestor

- **Pros:** a call site that forgets to bound its container still gets *some* cap, rather than silently regressing to unbounded growth.
- **Cons:** this is exactly the bug being fixed — an early version of this change shipped with a `65vh` cap on `XmlViewer.vue`, and it clipped `TemplateBrowser.vue`'s OPT XML tab short of the panel's actual (taller) available height, leaving visible dead space below the viewer. Any single fixed value is wrong for some panel size; there's no vh guess that's simultaneously "big enough to fill a tall panel" and "small enough not to overflow a short one."
- **Verdict:** rejected for both components. Correctness here means each call site owning its real bound, the same way `RequestInspector.vue`'s `.tree-container`/`.tree-scroll` already did for the Tree body-view tab before this ADR.

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

- ADR-0022 — Harmonized XML Viewer Component (one of the two components this ADR fixes the performance of)
- ADR-0021 — Harmonized JSON Viewer Component (the other component this ADR fixes the performance of; also the precedent for splitting pure logic from the Vue component for testability)
- ADR-0012 — CodeMirror 6 for AQL Editor (precedent for `useCodeMirror.ts`'s caller-owned container ref pattern, reused here)
