# PRD-0010: Composition Viewer In-Panel Search

**Version:** 1.0
**Date:** 2026-04-05
**Status:** Draft
**Owner:** openEHR Explorer (standalone — openEHR ecosystem)
**Repo slug:** `openehr-explorer`
**Depends on:** PRD-0009 (Template Browser In-Panel Search)

---

## Executive Summary

Extend the **in-panel search functionality** from PRD-0009 (Template Browser) to the **Composition Viewer** across its three view tabs (Pretty, JSON, FLAT). Users can press **Ctrl+F / Cmd+F** to activate context-aware search specific to each tab:

- **Pretty View**: Filter composition tree nodes (similar to OPT Tree filtering with ancestor preservation)
- **JSON View**: Highlight all matches with Enter/Shift+Enter navigation
- **FLAT View**: Highlight all matches with Enter/Shift+Enter navigation

This provides a **consistent search experience** across the entire application and enables developers to quickly locate specific data points, paths, or values within composition instances.

**Scope:** Frontend-only enhancement. Reuses `SearchOverlay` component and search patterns from PRD-0009.

---

## Problem Statement

### Current State

After PRD-0009, the Template Browser has comprehensive in-panel search, but the **Composition Viewer lacks this capability**. Users must:

- Manually scroll through large composition trees to find specific nodes
- Use browser Ctrl+F (which doesn't understand tree hierarchies)
- Copy JSON/FLAT to external editors to search

### Pain Points

1. **Large compositions are difficult to navigate**: Compositions with 100+ nodes require tedious manual scrolling/expanding
2. **No quick way to find specific values**: Searching for a patient name, diagnosis code, or medication requires visual scanning
3. **Browser search doesn't filter trees**: Native Ctrl+F highlights but doesn't collapse irrelevant branches
4. **Inconsistent UX**: Template Browser has search; Composition Viewer doesn't

---

## Goals & Non-Goals

### Goals

- Provide **consistent search UX** matching PRD-0009 implementation
- **Pretty View**: Tree filtering with ancestor preservation (like OPT Tree)
- **JSON View**: Highlight matches with navigation (like Web Template)
- **FLAT View**: Highlight matches with navigation (like FLAT Paths highlighting)
- Reuse existing `SearchOverlay` component from PRD-0009
- Keyboard activation: Ctrl+F / Cmd+F, Esc dismissal

### Non-Goals

- Searching across multiple compositions (single composition scope only)
- Regex or advanced search operators (defer to future PRD)
- Search state persistence across composition changes
- Backend search or filtering

---

## Feature Requirements

### Feature 1: Keyboard-Activated Search Overlay

**Priority:** P0 (Must Have)

**Implementation:** Identical to PRD-0009 Feature 1.

- Trigger: Ctrl+F / Cmd+F activates search
- SearchOverlay component appears at top of active tab
- Esc key dismisses search
- Auto-focus search input on activation
- Tab-specific search state (switching tabs clears search)

**Reuse from PRD-0009:**
- `SearchOverlay.vue` component (no changes needed)
- Keyboard event handling pattern
- Search overlay UI/UX

---

### Feature 2: Pretty View — Tree Filtering

**Priority:** P0 (Must Have)

#### 2.1 Behavior

**Identical to PRD-0009 Feature 5 (OPT Tree filtering):**

- Filter tree nodes to show only matching nodes **and their ancestors**
- Matching logic: Case-insensitive substring match against:
  - Node **name** (e.g., "Blood Pressure", "systolic")
  - Node **path** (e.g., "/content[openEHR-EHR-OBSERVATION...]")
  - Node **value** (e.g., "120", "mmHg", "Dr. Smith")
- Display:
  - Matching nodes: **Highlighted** (yellow background on matched text)
  - Ancestor nodes: **Visible but dimmed** (opacity 0.6, italic)
  - Non-matching nodes with no matching descendants: **Hidden**
- Auto-expand all ancestor nodes of matches
- Match counter: "X nodes match"

#### 2.2 Empty State

- If no matches: "No nodes match 'query'" centered message
- Original tree reappears when search cleared

#### 2.3 Implementation Notes

**Reuse from PRD-0009:**
- Tree filtering algorithm (recursive ancestor preservation)
- `FilteredNode` interface pattern
- Highlighting with `<mark>` tags
- CSS styles for `.tree-search-match`, `.is-match`, `.is-ancestor`

**Adapt for Composition Viewer:**
- Filter `CompositionTree` component nodes
- Match against composition data paths and values
- Preserve existing path highlighting integration

---

### Feature 3: JSON View — Highlight Matches

**Priority:** P0 (Must Have)

**Implementation:** Identical to PRD-0009 Feature 3 (Web Template JSON highlighting).

- Highlight all matching text in syntax-highlighted JSON
- Yellow background for matches, orange for current match
- Enter = next match, Shift+Enter = previous match
- Auto-scroll current match into view
- Match counter: "X of Y matches"

**Reuse from PRD-0009:**
- Two-pass highlighting (syntax first, then search)
- Match navigation logic (currentMatchIndex tracking)
- `scrollToMatch()` function
- CSS styles for `.search-match` and `.current-match`

**Note:** JSON syntax highlighting already exists in CompositionViewer.vue but needs `:deep()` selectors (see PRD-0009 implementation).

---

### Feature 4: FLAT View — Highlight Matches

**Priority:** P0 (Must Have)

**Implementation:** Identical to PRD-0009 Feature 3, applied to FLAT JSON.

- Same behavior as JSON View (highlight + navigation)
- Works with FLAT format key-value pairs
- Match counter: "X of Y matches"

**Reuse from PRD-0009:**
- All JSON highlighting logic
- Match navigation
- CSS styles

---

## User Stories

### Story 1: Developer Finds Patient Name in Composition

**As an** openEHR developer,
**I want to** search for "Max Mustermann" in a composition,
**So that** I can quickly verify the composer/subject data without scrolling.

**Acceptance Criteria:**
- ✅ Pressing Ctrl+F in Pretty view activates search
- ✅ Typing "Max" filters tree to show matching nodes (e.g., composer/name)
- ✅ Ancestor nodes remain visible for context
- ✅ Matched text is highlighted in yellow

---

### Story 2: Informaticist Searches JSON for Specific Code

**As a** clinical informaticist,
**I want to** search JSON view for "433" (SNOMED code),
**So that** I can verify the correct terminology code was used.

**Acceptance Criteria:**
- ✅ Pressing Cmd+F in JSON view activates search
- ✅ Typing "433" highlights all occurrences
- ✅ Pressing Enter cycles through matches
- ✅ Current match is highlighted in orange and scrolls into view

---

### Story 3: Developer Searches FLAT for Path

**As an** openEHR developer,
**I want to** search FLAT view for "blood_pressure" path,
**So that** I can find the exact FLAT path syntax for my SDK integration.

**Acceptance Criteria:**
- ✅ Pressing Ctrl+F in FLAT view activates search
- ✅ Typing "blood_pressure" highlights all matching keys/values
- ✅ Match counter shows "5 of 12"
- ✅ Navigation works with Enter/Shift+Enter

---

## Technical Approach

### Reuse from PRD-0009

**Components:**
- `SearchOverlay.vue` (no changes)
- Keyboard event handling pattern
- Tree filtering algorithm
- JSON/XML highlighting with search
- Match navigation logic

**CSS:**
- All search-related styles from PRD-0009
- `.search-match`, `.current-match`, `.tree-search-match`
- `.is-match`, `.is-ancestor` for tree nodes

### New Implementation

**File:** `src/views/CompositionViewer.vue`

**Add:**
1. Import `SearchOverlay` component
2. Add search state refs: `showPanelSearch`, `panelSearchQuery`, `currentMatchIndex`
3. Keyboard event handlers (onMounted/onUnmounted)
4. Tree filtering logic (adapt from PRD-0009 `filterTreeNode`)
5. JSON/FLAT highlighting (reuse `highlightSearchInContent` pattern)
6. Match navigation (`goToNextMatch`, `goToPreviousMatch`)
7. Integrate SearchOverlay into each tab's template

**Adapt CompositionTree component:**
- Add optional `searchQuery` prop
- Add filtered node rendering (similar to `WtTreeNodeFiltered`)
- Highlight matched text in node names/values

---

## Success Metrics

**Quantitative:**
- Search activation < 200ms
- Tree filtering < 100ms for 200+ node compositions
- JSON/FLAT highlighting < 100ms for 500KB data

**Qualitative:**
- Developers report faster composition inspection
- Consistent UX with Template Browser search
- Positive feedback on tree filtering with ancestor context

---

## Timeline & Prioritization

**Estimated effort:** 1–2 days (leverages PRD-0009 implementation)

**Prioritization:**
- **P0**: Features 1, 2, 3, 4 (all tab search functionality)

**Suggested milestone:** Include in same release as PRD-0009 (v0.3.0 or similar)

---

## Dependencies

- **PRD-0009** (Template Browser In-Panel Search) — Must be implemented first
- `SearchOverlay.vue` component
- Existing `CompositionTree` component
- No backend changes required

---

## Open Questions & Risks

### Open Questions

1. **Should search in Pretty view match against data type badges (DV_TEXT, etc.)?**
   - **Proposed answer:** Yes, include RM types in search matching.

2. **Should we highlight path column matches in Pretty view separately?**
   - **Proposed answer:** Yes, use same `.tree-search-match` styling.

3. **Should FLAT view filter keys (like FLAT Paths tab) in addition to highlighting?**
   - **Proposed answer:** No, keep FLAT view as pure highlighting for consistency with JSON view. Users can use Pretty view for filtering.

### Risks

- **Low risk:** Reuses proven PRD-0009 implementation
- **CompositionTree complexity:** May need refactoring to support filtering efficiently
  - **Mitigation:** Use computed properties for filtered tree, similar to OPT Tree
- **Performance with large compositions:** 500+ node compositions may cause lag
  - **Mitigation:** Test with large samples; add debouncing if needed

---

## Alternatives Considered

### Alternative 1: Only Implement JSON/FLAT Search (Skip Pretty View)

**Pros:** Simpler implementation; fewer changes to CompositionTree.
**Cons:** Loses the most valuable search use case (tree filtering); inconsistent with PRD-0009.
**Decision:** Rejected. Pretty view tree filtering is the primary value proposition.

---

### Alternative 2: Unified Search Across All Three Tabs

**Pros:** Single search shows results in all tabs simultaneously.
**Cons:** Confusing UX (which tab shows matches?); complex state management.
**Decision:** Rejected. Tab-specific search is clearer and matches PRD-0009.

---

## Appendices

### Appendix A: Code Reuse from PRD-0009

**Reusable Components:**
- `SearchOverlay.vue` (unchanged)

**Reusable Functions/Patterns:**
```typescript
// From PRD-0009 TemplateBrowser.vue
function filterTreeNode(node, query): FilteredNode | null
function highlightSearchInContent(html, searchQuery): string
function escapeRegex(str): string
function goToNextMatch()
function goToPreviousMatch()
function scrollToMatch()
```

**Reusable CSS:**
```css
/* From PRD-0009 */
:deep(.search-match) { /* yellow highlighting */ }
:deep(.search-match.current-match) { /* orange highlighting */ }
:deep(.tree-search-match) { /* tree node highlighting */ }
:deep(.wt-node-header.is-match) { /* matching node background */ }
:deep(.wt-node-header.is-ancestor) { /* ancestor node dimming */ }
```

---

### Appendix B: Composition Tree Filtering Example

**Before search (all nodes visible):**
```
▼ Nutritional state (COMPOSITION)
  ▼ category (DV_CODED_TEXT): event
    ▸ defining_code (CODE_PHRASE): 433
  ▼ composer (PARTY_IDENTIFIED)
    - name: "Max Mustermann"
  ▼ Physical examination findings (OBSERVATION)
    ▼ Event Series (EVENT)
      ...
```

**After searching "Max" (filtered):**
```
▼ Nutritional state (COMPOSITION) [ancestor, dimmed]
  ▼ composer (PARTY_IDENTIFIED) [ancestor, dimmed]
    - name: "Max Mustermann" [match, highlighted]
```

Only the matching node and its ancestors are shown; other branches are hidden.

---

## Revision History

| Version | Date       | Author              | Changes                          |
|---------|------------|---------------------|----------------------------------|
| 1.0     | 2026-04-05 | openEHR Explorer    | Initial draft                    |

---

**End of PRD-0010**
