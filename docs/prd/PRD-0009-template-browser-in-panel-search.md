# PRD-0009: Template Browser In-Panel Search

**Version:** 1.0
**Date:** 2026-04-05
**Status:** Draft
**Owner:** openEHR Explorer (standalone — openEHR ecosystem)
**Repo slug:** `openehr-explorer`
**Depends on:** PRD-0001 (Desktop CDR Browser — MVP features), PRD-0008 (OPT Metadata Display)

---

## Executive Summary

Add **keyboard-activated in-panel search** to all four Template Browser view tabs (OPT Tree, OPT XML, Web Template, FLAT Paths). Users can press **Ctrl+F** (Windows/Linux) or **Cmd+F** (macOS) to activate a context-aware search overlay specific to each tab:

- **FLAT Paths**: Live-filter displayed paths (fuzzy/substring matching)
- **Web Template JSON**: Highlight all search matches in the syntax-highlighted JSON
- **OPT XML**: Highlight all search matches in the syntax-highlighted XML
- **OPT Tree**: Filter displayed tree nodes (keeping parent nodes of matches visible for context)

This feature transforms the Template Browser from a **passive viewer** into a **powerful exploration tool**, enabling developers to quickly locate specific archetype nodes, FLAT paths, terminology codes, or metadata attributes across large templates without manual scrolling or browser Ctrl+F (which doesn't understand tree hierarchies).

**Scope:** Frontend-only enhancement. No backend changes required.

---

## Problem Statement

### Current State

After PRD-0001 and PRD-0008, the Template Browser displays four rich views of template data:

1. **OPT Tree**: Collapsible tree of template nodes with metadata panel and info banner
2. **OPT XML**: Syntax-highlighted raw XML with copy button
3. **Web Template**: Syntax-highlighted JSON with copy button
4. **FLAT Paths**: Scrollable list of FLAT paths (often 50–500+ items)

### Pain Points

1. **FLAT Paths overwhelm**: Templates with 200+ FLAT paths require tedious scrolling to find a specific path (e.g., "blood_pressure/systolic").
2. **Browser Ctrl+F doesn't understand tree structure**: Using native browser search on the OPT Tree highlights matches but doesn't collapse irrelevant branches — users must manually scan the entire tree.
3. **JSON/XML search is brittle**: Browser Ctrl+F highlights text in the raw HTML, but doesn't interact with syntax highlighting, making it hard to see context (e.g., whether a match is a key, value, or attribute).
4. **No keyboard-first workflow**: Developers expect Ctrl+F/Cmd+F to work in content-heavy panels, but currently it triggers the browser's global search (which searches the entire app UI, including sidebars).
5. **Lost context in FLAT paths**: Filtering paths client-side would be trivial, but currently there's no UI for it — users copy-paste the entire list into an external editor to search.

### User Personas

(Inherited from PRD-0001 and PRD-0008; ranked by benefit from this PRD.)

1. **openEHR Developer** — Primary beneficiary. Frequently searches for specific archetype nodes, FLAT paths, or terminology codes when debugging form mappings.
2. **Integration Engineer** — Needs to quickly locate specific paths in large templates to map to external data schemas.
3. **Clinical Informaticist** — Searches for specific clinical concepts (e.g., "diagnosis", "medication") to verify template coverage.
4. **openEHR Learner** — Uses search to explore template structure by keyword (e.g., "time", "coded_text").

---

## Goals & Non-Goals

### Goals

- Activate search with **Ctrl+F (Windows/Linux) or Cmd+F (macOS)** when focus is within a Template Browser panel.
- Provide **tab-specific search behavior**:
  - **FLAT Paths**: Real-time filtering (show only matching paths).
  - **OPT XML & Web Template**: Highlight all matches in syntax-highlighted content with scroll-to-match navigation.
  - **OPT Tree**: Filter tree nodes (collapse non-matching branches, keep parent nodes visible for context).
- Display a **search input overlay** at the top of each panel (non-modal, dismissible with Esc).
- Show **match count** and **current match index** (e.g., "3 of 12 matches").
- Support **case-insensitive** search by default (optional case-sensitive toggle is a stretch goal).
- **Navigate between matches** with Enter (next) and Shift+Enter (previous) in JSON/XML views.

### Non-Goals

- **Regex search** — Out of scope for v1; defer to future PRD if needed.
- **Multi-term search** (e.g., "AND/OR" operators) — Out of scope.
- **Search across multiple templates** — Scope is single-template, single-panel search only.
- **Persistent search state across tabs** — Switching tabs clears the search query (each tab is independent).
- **Backend search** — All search is client-side; no new API calls.
- **Fuzzy matching** (beyond simple substring) — Nice-to-have for future iteration.

---

## Feature Requirements

### Feature 1: Keyboard-Activated Search Overlay

**Priority:** P0 (Must Have)

#### 1.1 Activation

- **Trigger**: Pressing **Ctrl+F** (Windows/Linux) or **Cmd+F** (macOS) while focus is within the Template Browser right panel.
- **Behavior**:
  - Show a search input overlay at the **top of the active tab content area** (below the tab bar, above the metadata/info sections).
  - Prevent the browser's default Ctrl+F behavior for the Template Browser panel (use `event.preventDefault()`).
  - Auto-focus the search input field.

#### 1.2 Search Input Overlay UI

**Layout:**

```
┌─────────────────────────────────────────────────────────┐
│ OPT Tree | OPT XML | Web Template | FLAT Paths (active) │
├─────────────────────────────────────────────────────────┤
│ ┌───────────────────────────────────────────────────┐   │
│ │ 🔍 [Search input field...        ] [3 of 12] [×]  │   │
│ └───────────────────────────────────────────────────┘   │
│                                                         │
│ [Panel content with search applied...]                 │
└─────────────────────────────────────────────────────────┘
```

**Components:**
- **Search icon** (🔍 or magnifying glass SVG)
- **Input field**: Placeholder text varies by tab:
  - FLAT Paths: "Filter paths..."
  - Web Template: "Search JSON..."
  - OPT XML: "Search XML..."
  - OPT Tree: "Search tree..."
- **Match counter**: "X of Y" (e.g., "3 of 12") — only shown when matches exist
- **Close button** (×): Dismisses the search overlay and clears the search

**Styling:**
- Background: Subtle overlay (e.g., `rgba(100, 149, 237, 0.08)` similar to info banner)
- Border: Bottom border to separate from content
- Compact height (36–40px)
- Fixed position at top of tab content (does not scroll away)

#### 1.3 Dismissal

- **Esc key**: Clears search and hides overlay
- **Close button (×)**: Clears search and hides overlay
- **Switching tabs**: Hides search overlay (does not persist query across tabs)

#### 1.4 State Management

- Each tab maintains its own search state (independent queries)
- Search state resets when:
  - Template is changed (new template selected)
  - Search overlay is dismissed
  - Tab is switched

---

### Feature 2: FLAT Paths — Live Filtering

**Priority:** P0 (Must Have)

#### 2.1 Behavior

- **As user types**: Filter the displayed FLAT paths list in real-time
- **Matching logic**: Case-insensitive substring match (e.g., "blood" matches "minimal/blood_pressure/systolic")
- **Display**: Show only matching paths; hide non-matching paths entirely
- **Match counter**: "X of Y paths" (e.g., "3 of 50 paths")

#### 2.2 Empty State

- If no matches: Show centered message "No paths match 'query'"
- Original list reappears when search is cleared

#### 2.3 Performance

- Filtering should be instant for up to 1000 paths
- Use `computed` reactive property in Vue (no debouncing needed for simple substring match)

---

### Feature 3: Web Template JSON — Highlight Matches

**Priority:** P0 (Must Have)

#### 3.1 Behavior

- **As user types**: Highlight all matching text in the syntax-highlighted JSON
- **Matching logic**: Case-insensitive substring match across the entire JSON string (keys, values, etc.)
- **Highlighting**: Wrap matches in `<mark>` tags with distinct styling (e.g., yellow background)
- **Current match**: Highlight with different color (e.g., orange background) and scroll into view
- **Match counter**: "X of Y matches" (e.g., "3 of 12")

#### 3.2 Navigation

- **Enter**: Jump to next match (cycles back to first after last match)
- **Shift+Enter**: Jump to previous match (cycles to last after first match)
- **Auto-scroll**: Scroll the current match into view (centered if possible)

#### 3.3 Implementation Notes

- Apply highlighting **after** syntax highlighting (don't break existing HTML structure)
- Use a two-pass approach:
  1. Syntax highlight the JSON (existing `highlightJson` function)
  2. Wrap search matches in `<mark>` tags (case-insensitive, preserve syntax highlighting)
- Handle escaped characters gracefully (e.g., searching for `"value"` should match both the key `"value":` and string value `"some value"`)

---

### Feature 4: OPT XML — Highlight Matches

**Priority:** P0 (Must Have)

#### 4.1 Behavior

Identical to Feature 3 (Web Template JSON), but applied to the OPT XML view:

- Highlight all matches in syntax-highlighted XML
- Navigate with Enter/Shift+Enter
- Auto-scroll to current match
- Match counter

#### 4.2 Implementation Notes

- Apply highlighting after existing `highlightXml` function
- Same two-pass approach as JSON highlighting

---

### Feature 5: OPT Tree — Filter Nodes

**Priority:** P0 (Must Have)

#### 5.1 Behavior

- **As user types**: Filter tree nodes to show only matching nodes **and their ancestors**
- **Matching logic**: Case-insensitive substring match against:
  - Node **name** (e.g., "Blood Pressure")
  - Node **ID** (e.g., "blood_pressure")
  - Node **RM type** (e.g., "OBSERVATION")
  - Node **AQL path** (e.g., "/content[openEHR-EHR-OBSERVATION.blood_pressure.v1]")
- **Display**:
  - Matching nodes: **Highlighted** (e.g., bold or yellow background on matched text)
  - Ancestor nodes: **Visible but dimmed** (to provide context)
  - Non-matching nodes with no matching descendants: **Hidden**
- **Auto-expand**: Expand all ancestor nodes of matches (override collapsed state)
- **Match counter**: "X nodes match" (e.g., "5 nodes match")

#### 5.2 Tree Filtering Algorithm

**Pseudocode:**
```
function filterTree(node, query):
  if query is empty:
    return node (show all)

  matchesQuery = node.name.includes(query) OR
                 node.id.includes(query) OR
                 node.rmType.includes(query) OR
                 node.aqlPath.includes(query)

  filteredChildren = node.children.map(child => filterTree(child, query)).filter(notNull)

  if matchesQuery OR filteredChildren.length > 0:
    return {
      ...node,
      children: filteredChildren,
      isMatch: matchesQuery,
      isAncestor: !matchesQuery && filteredChildren.length > 0,
      forceExpanded: true
    }
  else:
    return null (hide this node)
```

#### 5.3 Visual Treatment

- **Matching nodes**: Highlight matched substring (e.g., yellow background)
- **Ancestor nodes**: Dimmed text color (e.g., 50% opacity) with italic font
- **Expand/collapse icons**: Disabled during search (all matching branches auto-expanded)

#### 5.4 Empty State

- If no matches: Show centered message "No nodes match 'query'"
- Original tree reappears when search is cleared

---

## User Stories

### Story 1: Developer Finds FLAT Path Quickly

**As a** openEHR developer,
**I want to** filter FLAT paths by typing "systolic",
**So that** I can quickly locate `minimal/blood_pressure/systolic/magnitude` without scrolling through 200+ paths.

**Acceptance Criteria:**
- ✅ Pressing Ctrl+F activates search overlay
- ✅ Typing "systolic" filters the list to show only matching paths
- ✅ Match count shows "2 of 50 paths"
- ✅ Clearing search restores full list

---

### Story 2: Informaticist Searches XML for Terminology Code

**As a** clinical informaticist,
**I want to** search the OPT XML for "SNOMED" to verify terminology bindings,
**So that** I can ensure the template uses standardized codes.

**Acceptance Criteria:**
- ✅ Pressing Cmd+F (macOS) activates search overlay
- ✅ Typing "SNOMED" highlights all 5 occurrences in yellow
- ✅ Current match is highlighted in orange
- ✅ Pressing Enter cycles through matches
- ✅ Each match auto-scrolls into view

---

### Story 3: Developer Filters Tree to Find Specific Node

**As an** openEHR developer,
**I want to** search the OPT Tree for "medication" to find all medication-related nodes,
**So that** I can see the structure without manually expanding the entire tree.

**Acceptance Criteria:**
- ✅ Pressing Ctrl+F activates search overlay
- ✅ Typing "medication" filters the tree to show 3 matching nodes
- ✅ Parent nodes remain visible (dimmed) for context
- ✅ Matching text is highlighted in yellow
- ✅ Match count shows "3 nodes match"
- ✅ Clearing search restores full tree

---

### Story 4: Learner Explores Template by Keyword

**As an** openEHR learner,
**I want to** search the Web Template JSON for "DV_CODED_TEXT",
**So that** I can learn which fields use coded values.

**Acceptance Criteria:**
- ✅ Pressing Ctrl+F activates search overlay
- ✅ Typing "DV_CODED_TEXT" highlights all 8 matches
- ✅ Pressing Enter jumps to each match sequentially
- ✅ Syntax highlighting is preserved (key names remain blue, values remain green)

---

## Technical Approach

### Keyboard Event Handling

**Capture Ctrl+F / Cmd+F at the component level:**

```typescript
// In TemplateBrowser.vue
onMounted(() => {
  window.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown);
});

function handleKeydown(e: KeyboardEvent) {
  // Only capture if focus is within the template browser panel
  if (!isTemplateDetailView.value) return;

  if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
    e.preventDefault();
    showSearchOverlay.value = true;
    nextTick(() => searchInputRef.value?.focus());
  }

  if (e.key === 'Escape' && showSearchOverlay.value) {
    clearSearch();
  }
}
```

### FLAT Paths Filtering

**Use computed property for reactive filtering:**

```typescript
const searchQuery = ref('');

const filteredFlatPaths = computed(() => {
  if (!searchQuery.value) return flatPaths.value;

  const query = searchQuery.value.toLowerCase();
  return flatPaths.value.filter(path => path.toLowerCase().includes(query));
});
```

### JSON/XML Highlighting with Search

**Two-pass highlighting approach:**

```typescript
function highlightJsonWithSearch(json: string, searchQuery: string): string {
  // First pass: Apply syntax highlighting
  let highlighted = highlightJson(json);

  // Second pass: Wrap search matches in <mark> tags
  if (searchQuery) {
    const regex = new RegExp(`(${escapeRegex(searchQuery)})`, 'gi');
    highlighted = highlighted.replace(regex, '<mark class="search-match">$1</mark>');
  }

  return highlighted;
}
```

**Note:** Must handle HTML entities carefully (e.g., `&lt;` should match when searching for `<`).

### Tree Filtering

**Recursive filtering with ancestor tracking:**

```typescript
interface FilteredNode extends WtNode {
  isMatch: boolean;
  isAncestor: boolean;
  forceExpanded: boolean;
}

function filterTree(node: WtNode, query: string): FilteredNode | null {
  if (!query) return { ...node, isMatch: false, isAncestor: false, forceExpanded: false };

  const lowerQuery = query.toLowerCase();
  const matchesQuery =
    node.name.toLowerCase().includes(lowerQuery) ||
    node.id.toLowerCase().includes(lowerQuery) ||
    node.rmType.toLowerCase().includes(lowerQuery) ||
    node.aqlPath.toLowerCase().includes(lowerQuery);

  const filteredChildren = node.children
    .map(child => filterTree(child, query))
    .filter(Boolean) as FilteredNode[];

  if (matchesQuery || filteredChildren.length > 0) {
    return {
      ...node,
      children: filteredChildren,
      isMatch: matchesQuery,
      isAncestor: !matchesQuery && filteredChildren.length > 0,
      forceExpanded: true,
    };
  }

  return null;
}
```

### Match Navigation (JSON/XML)

**Track current match index and scroll into view:**

```typescript
const currentMatchIndex = ref(0);
const totalMatches = ref(0);

function goToNextMatch() {
  if (totalMatches.value === 0) return;

  currentMatchIndex.value = (currentMatchIndex.value + 1) % totalMatches.value;
  scrollToMatch(currentMatchIndex.value);
}

function scrollToMatch(index: number) {
  const matches = document.querySelectorAll('.search-match');
  if (matches[index]) {
    matches[index].scrollIntoView({ behavior: 'smooth', block: 'center' });

    // Highlight current match differently
    matches.forEach((el, i) => {
      el.classList.toggle('current-match', i === index);
    });
  }
}
```

---

## Success Metrics

**Quantitative:**

- **Search activation**: < 200ms from keypress to search overlay visible
- **FLAT Paths filtering**: < 50ms to filter 1000 paths
- **JSON/XML highlighting**: < 100ms to highlight matches in 500KB JSON/XML
- **Tree filtering**: < 100ms to filter tree with 200+ nodes
- **Zero regressions**: Existing syntax highlighting, collapsing, and copy functionality remain unchanged

**Qualitative:**

- Developers report **faster template exploration** compared to manual scrolling or browser Ctrl+F
- Positive feedback on **keyboard-first workflow** (Ctrl+F activation feels native)
- Users successfully **locate specific nodes/paths** in large templates without frustration

---

## Timeline & Prioritization

**Estimated effort:** 2–3 days (frontend-only, no backend changes).

**Prioritization:**
- **P0 (Must Have)**: Features 1, 2, 3, 4, 5 (core search functionality for all tabs)
- **P1 (Nice to Have)**:
  - Case-sensitive toggle
  - Search history (recent searches dropdown)
  - Regex support

**Suggested milestone:** Include in next minor release (v0.3.0 or similar).

---

## Dependencies

- **PRD-0001** (Template Browser exists with all four tabs)
- **PRD-0008** (OPT Tree includes metadata panel and info banner — search must not interfere)
- **No new backend work** — All search is client-side
- **No new dependencies** — Uses vanilla JS string manipulation and DOM APIs

---

## Open Questions & Risks

### Open Questions

1. **Should search state persist when switching templates?**
   - **Proposed answer**: No. Switching templates clears search (different content context).

2. **Should Enter/Shift+Enter navigation work in FLAT Paths and Tree views?**
   - **Proposed answer**: Not necessary for v1 (filtering is sufficient). Defer to future iteration.

3. **Should we support searching within collapsed nodes in OPT Tree?**
   - **Proposed answer**: Yes — search auto-expands matching branches.

4. **Should we highlight partial matches in Tree view (e.g., "blood" in "blood_pressure")?**
   - **Proposed answer**: Yes — use `<mark>` tags similar to JSON/XML highlighting.

### Risks

- **Performance risk**: Large templates (500+ nodes, 1MB JSON) may cause lag during highlighting.
  - **Mitigation**: Test with large templates; add debouncing if needed (300ms delay).

- **Regex edge cases**: Escaping user input for regex matching is error-prone.
  - **Mitigation**: Use simple `String.prototype.includes()` for v1 (no regex); defer regex support to future PRD.

- **Accessibility**: Screen reader users may not discover the search feature.
  - **Mitigation**: Add aria-label to search input; announce match count via aria-live region.

- **Browser Ctrl+F conflict**: Some users may expect native browser search to work.
  - **Mitigation**: Add tooltip/help text explaining the in-panel search behavior.

---

## Alternatives Considered

### Alternative 1: Use Browser's Native Ctrl+F

**Pros:** No implementation effort; users already familiar with browser search.
**Cons:**
- Doesn't understand tree hierarchies (collapses non-matching branches)
- Doesn't filter FLAT paths (just highlights)
- Breaks syntax highlighting in JSON/XML views
- Poor UX for dense content

**Decision:** Rejected. In-panel search provides superior UX for structured data.

---

### Alternative 2: Global Search Across All Tabs

**Pros:** Single search input searches all four tabs simultaneously.
**Cons:**
- Complex UI (how to show results from 4 different formats?)
- Confusing match counts ("12 matches" — where?)
- Performance implications (search 4 views simultaneously)

**Decision:** Rejected. Tab-specific search is simpler and more intuitive.

---

### Alternative 3: Add Search to Template List (Left Panel)

**Pros:** Users could search across all templates (not just current template).
**Cons:** Out of scope for this PRD (different use case).
**Decision:** Deferred to future PRD. This PRD focuses on in-panel search only.

---

## Appendices

### Appendix A: Keyboard Shortcuts Summary

| Shortcut          | Action                                  | Context                  |
|-------------------|-----------------------------------------|--------------------------|
| Ctrl+F / Cmd+F    | Activate search overlay                 | Template Browser panel   |
| Esc               | Dismiss search overlay / clear search   | Search overlay active    |
| Enter             | Next match (JSON/XML only)              | Search overlay active    |
| Shift+Enter       | Previous match (JSON/XML only)          | Search overlay active    |

---

### Appendix B: Match Highlighting Styles

**Search Match (non-current):**
```css
.search-match {
  background: rgba(255, 255, 0, 0.3); /* Yellow */
  border-radius: 2px;
  padding: 0 2px;
}
```

**Current Match:**
```css
.search-match.current-match {
  background: rgba(255, 165, 0, 0.5); /* Orange */
  outline: 1px solid rgba(255, 165, 0, 0.8);
}
```

**Tree Node Match:**
```css
.wt-node-header.is-match .wt-name {
  background: rgba(255, 255, 0, 0.2);
  font-weight: 700;
}

.wt-node-header.is-ancestor {
  opacity: 0.5;
  font-style: italic;
}
```

---

### Appendix C: Accessibility Considerations

**ARIA Labels:**
```html
<input
  type="text"
  aria-label="Search within template"
  aria-describedby="match-counter"
/>

<div id="match-counter" role="status" aria-live="polite">
  3 of 12 matches
</div>
```

**Keyboard Navigation:**
- Ensure search input is reachable via Tab key
- Ensure search can be dismissed with Esc (keyboard-only users)
- Announce match count changes to screen readers via aria-live

---

## Revision History

| Version | Date       | Author              | Changes                          |
|---------|------------|---------------------|----------------------------------|
| 1.0     | 2026-04-05 | openEHR Explorer    | Initial draft                    |

---

**End of PRD-0009**
