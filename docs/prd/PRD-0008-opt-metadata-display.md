# PRD-0008: OPT Metadata Display Enhancement

**Version:** 1.0
**Date:** 2026-04-05
**Status:** Draft
**Owner:** openEHR Explorer (standalone — openEHR ecosystem)
**Repo slug:** `openehr-explorer`
**Depends on:** PRD-0001 (Desktop CDR Browser — MVP features)

---

## Executive Summary

Enhance the Template Browser's **OPT Tree tab** to parse and display rich metadata from the OPT XML `<description>` element. Currently, openEHR Explorer shows only the template tree structure in the OPT Tree tab, discarding valuable provenance, authorship, lifecycle, and documentation metadata embedded in every operational template.

This PRD adds a **metadata panel** to the OPT Tree view that surfaces:

- **Authorship & provenance** (original author, organization, contributors/reviewers)
- **Lifecycle state** (published, draft, deprecated)
- **Documentation** (purpose, use, copyright)
- **Technical metadata** (MD5 checksums, parent template references, semantic versioning)

This turns the Template Browser from a structural explorer into a **comprehensive template inspector** — critical for understanding template lineage, trustworthiness, and appropriate usage in clinical and development contexts.

**Scope:** Frontend-only enhancement. No new backend commands required; we already fetch OPT XML via the existing `get_template_opt` command.

---

## Problem Statement

### Current State

After PRD-0001, the Template Browser displays:

- **Template list** (left panel): template_id, concept, created_timestamp
- **OPT Tree tab**: Collapsible tree view of the Web Template structure (nodes, RM types, AQL paths)
- **OPT XML tab**: Raw syntax-highlighted XML with copy button
- **Web Template tab**: Raw JSON with copy button
- **FLAT Paths tab**: Extracted FLAT paths

The **OPT XML contains rich metadata**, but users must manually parse the raw XML to extract it. The `<description>` element is a standard openEHR structure containing:

```xml
<description>
  <original_author id="name">Hendrik Heiser</original_author>
  <original_author id="organisation">Cistec AG</original_author>
  <original_author id="email">hendrik.heiser@cistec.com</original_author>
  <original_author id="date">2025-03-19</original_author>
  <other_contributors>Jonas Bucher (Reviewer)</other_contributors>
  <lifecycle_state>published</lifecycle_state>
  <other_details id="licence">...</other_details>
  <other_details id="custodian_organisation">Cistec AG</other_details>
  <other_details id="original_namespace">com.cistec</other_details>
  <other_details id="MD5-CAM-1.0.1">0b1daab37a9a6d809d5dcc875960cddf</other_details>
  <other_details id="PARENT:MD5-CAM-1.0.1">005501C1FA493A483BF5F1121F2870EC</other_details>
  <other_details id="sem_ver">≥1.1.0</other_details>
  <!-- ... -->
</description>
```

### Pain Points

1. **Hidden provenance**: Users cannot quickly see who authored a template or which organization maintains it — critical for trust and support escalation.
2. **Lifecycle opacity**: No visual indication of whether a template is `published`, `draft`, `deprecated`, or `obsolete`.
3. **Lost documentation**: Purpose/use descriptions are buried in raw XML, making it hard to understand when to use a template.
4. **Technical lineage invisible**: MD5 checksums and parent template references are hidden, making it difficult to verify template integrity or understand inheritance.
5. **Poor discoverability**: Developers unfamiliar with openEHR specs don't know the `<description>` element exists.

### User Personas

(Inherited from PRD-0001; ranked by benefit from this PRD.)

1. **Clinical Informaticist** — Needs to verify template authorship and lifecycle state before approving for clinical use.
2. **openEHR Developer** — Wants to know who maintains a template when issues arise; needs to verify template versions match documentation.
3. **Integration Engineer** — Needs to validate template checksums and understand parent-child template relationships.
4. **openEHR Learner** — Benefits from seeing well-documented templates with clear purpose statements as learning examples.

---

## Goals & Non-Goals

### Goals

- Display human-readable metadata from OPT `<description>` in a structured, scannable format.
- Surface the most critical metadata prominently: lifecycle state, author, organization.
- Make technical metadata (checksums, parent refs) accessible but not overwhelming.
- Maintain performance: parsing XML client-side should add negligible latency.
- Provide a foundation for future enhancements (e.g., filtering templates by lifecycle state, showing template dependency graphs).

### Non-Goals

- **Editing OPT metadata** — Read-only display only. Template authoring happens in the Archetype Designer.
- **Validating MD5 checksums** — Out of scope for this PRD; defer to future quality-assurance features.
- **Displaying archetype-level metadata** — Focus on template-level `<description>` only.
- **Backend changes** — Use existing `get_template_opt` command; no new Rust code required.
- **Multi-template comparison** — Out of scope; defer to future PRD if needed.

---

## Feature Requirements

### Feature 1: OPT Description Metadata Parsing

**Priority:** P0 (Must Have)

#### 1.1 XML Parsing

Parse the OPT XML `<description>` element into a structured TypeScript interface:

```typescript
interface OptMetadata {
  originalAuthor: {
    name?: string;
    organisation?: string;
    email?: string;
    date?: string;
  };
  otherContributors?: string[]; // Array of contributor strings
  lifecycleState?: string; // "published" | "draft" | "deprecated" | etc.
  details?: string; // Free-text description/purpose
  otherDetails: Record<string, string>; // Key-value pairs for all <other_details id="...">
}
```

**Implementation notes:**

- Use browser-native `DOMParser` to parse OPT XML.
- Extract `<original_author id="...">` elements, grouping by `id` attribute.
- Extract `<other_contributors>` (may be multiple or single element with comma-separated values).
- Extract `<lifecycle_state>`.
- Extract `<details>` (free-text purpose/use).
- Extract all `<other_details id="...">` into a key-value map.
- Handle missing elements gracefully (not all templates have complete metadata).

#### 1.2 Error Handling

- If OPT XML is missing or malformed, show a fallback message: "Metadata unavailable".
- If `<description>` element is absent, show: "This template has no metadata".
- Invalid XML should not crash the app; log warning and display fallback.

---

### Feature 2: Metadata Display Panel

**Priority:** P0 (Must Have)

#### 2.1 UI Placement

Add a new **"Metadata"** section **above the tree view** in the OPT Tree tab. Layout:

```
┌─────────────────────────────────────────────────────────┐
│ OPT Tree tab (active)                                   │
├─────────────────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────────────────┐ │
│ │ METADATA (collapsible section, default: expanded)   │ │
│ │                                                     │ │
│ │ ● Lifecycle: [published] (badge, green)            │ │
│ │ ● Author: Hendrik Heiser (Cistec AG)               │ │
│ │ ● Contributors: Jonas Bucher (Reviewer)            │ │
│ │ ● Date: 2025-03-19                                 │ │
│ │ ● Custodian: Cistec AG                             │ │
│ │                                                     │ │
│ │ [▼ Show Technical Metadata] (expandable)           │ │
│ │   • MD5-CAM-1.0.1: 0b1daab37a9a6d809d5dcc875960cd │ │
│ │   • PARENT:MD5-CAM-1.0.1: 005501C1FA493A483BF5F11 │ │
│ │   • Semantic Version: ≥1.1.0                       │ │
│ │   • Namespace: com.cistec                          │ │
│ └─────────────────────────────────────────────────────┘ │
│                                                         │
│ [Tree view continues below...]                          │
└─────────────────────────────────────────────────────────┘
```

#### 2.2 Primary Metadata Fields (Always Visible)

Display prominently if available:

- **Lifecycle State**: Styled badge with color coding:
  - `published` → Green badge
  - `draft` → Yellow badge
  - `deprecated` / `obsolete` → Red badge
  - Other states → Gray badge
- **Original Author**: `{name} ({organisation})` or just name/org if one is missing. If email present, make name a `mailto:` link.
- **Other Contributors**: Comma-separated list or bulleted list if more than 3.
- **Author Date**: Display as-is (often ISO date or human-readable).
- **Custodian Organisation**: If different from author organisation.
- **Description/Purpose**: If `<details>` element exists, show in a muted text block (max 3 lines, "Show more..." expansion if longer).

#### 2.3 Technical Metadata (Collapsible)

Show in an expandable **"Technical Metadata"** disclosure:

- **MD5-CAM-1.0.1**: Template checksum (monospace font, copy button).
- **PARENT:MD5-CAM-1.0.1**: Parent template checksum (if present, indicates template inheritance).
- **Semantic Version**: If present (e.g., `≥1.1.0`).
- **Original Namespace**: If present (e.g., `com.cistec`).
- **Custodian Namespace**: If present and different from original.
- **Build UID**: If present.
- **Generated By**: If present (e.g., "Archetype Designer v1.24.16").
- **Other details**: Any remaining `<other_details>` key-value pairs not listed above.

Default state: **collapsed**. Advanced users can expand to see technical details.

#### 2.4 Fallback States

- **No metadata available**: Show muted message "This template has no metadata" (centered, gray text).
- **Partial metadata**: Display only the fields that exist; omit missing fields entirely (don't show "Author: N/A").

#### 2.5 Collapsibility

The entire metadata panel should be **collapsible** with a toggle button/icon:

- Default: **Expanded** (metadata visible).
- Collapsed state: Show only a single line: "📋 Metadata (Click to expand)".
- State persists in session (localStorage optional, not critical for v1).

---

### Feature 3: Component Structure

**Priority:** P1 (Nice to Have, but recommended for maintainability)

#### 3.1 New Component: `OptMetadata.vue`

Create a dedicated Vue component for metadata display:

**Location:** `src/components/OptMetadata.vue`

**Props:**
- `optXml: string` — The raw OPT XML string.

**Responsibilities:**
- Parse OPT XML using `DOMParser`.
- Extract metadata into `OptMetadata` TypeScript interface.
- Render metadata panel with collapsible sections.

**Emit:**
- (None; purely display component.)

#### 3.2 Integration into TemplateBrowser.vue

Import and use `<OptMetadata>` in the OPT Tree tab:

```vue
<div v-if="activeTab === 'tree'" class="tree-view">
  <OptMetadata v-if="templateStore.selectedOpt" :optXml="templateStore.selectedOpt" />
  <div v-if="wtTree" class="wt-tree">
    <WtTreeNode :node="wtTree" :depth="0" @copy="copyToClipboard" />
  </div>
</div>
```

---

## User Stories

### Story 1: Clinical Informaticist Reviews Template Provenance

**As a** clinical informaticist,
**I want to** see who authored a template and which organization maintains it,
**So that** I can assess trustworthiness and know who to contact for questions or issues.

**Acceptance Criteria:**
- ✅ Viewing a template's OPT Tree tab shows author name and organization.
- ✅ If author email is present, clicking the author name opens a mailto: link.
- ✅ Custodian organization is displayed if different from author.

---

### Story 2: Developer Checks Template Lifecycle State

**As an** openEHR developer,
**I want to** quickly see if a template is `published`, `draft`, or `deprecated`,
**So that** I know whether it's safe to use in production.

**Acceptance Criteria:**
- ✅ Lifecycle state is shown as a color-coded badge (green/yellow/red).
- ✅ Badge is prominently placed at the top of the metadata panel.
- ✅ Missing lifecycle state does not crash the app; field is omitted.

---

### Story 3: Integration Engineer Verifies Template Checksums

**As an** integration engineer,
**I want to** see the MD5-CAM checksum and parent template references,
**So that** I can verify template integrity and understand template inheritance.

**Acceptance Criteria:**
- ✅ MD5-CAM-1.0.1 checksum is displayed in monospace font.
- ✅ PARENT:MD5-CAM-1.0.1 is shown if present.
- ✅ Technical metadata section is collapsible (default: collapsed).
- ✅ Checksum values have a "Copy" button.

---

### Story 4: Learner Discovers Template Purpose

**As an** openEHR learner,
**I want to** read the template's purpose/description,
**So that** I understand when and how to use it.

**Acceptance Criteria:**
- ✅ If `<details>` element exists, it's displayed in the metadata panel.
- ✅ Long descriptions are truncated with "Show more..." expansion.
- ✅ Missing descriptions don't show an empty field.

---

## Technical Approach

### Metadata Parsing Strategy

**Frontend-only implementation:**

1. When a template is selected, `TemplateBrowser.vue` already fetches OPT XML via `templateStore.fetchOpt()`.
2. Pass `templateStore.selectedOpt` (string) to `<OptMetadata>` component.
3. `OptMetadata.vue` parses XML using `DOMParser`:
   ```typescript
   const parser = new DOMParser();
   const xmlDoc = parser.parseFromString(optXml, "text/xml");
   const description = xmlDoc.querySelector("description");
   ```
4. Extract metadata into structured object.
5. Render structured metadata using Vue template.

**No backend changes required.** All parsing happens client-side.

### Styling Approach

- Reuse existing design tokens from `src/assets/main.css`.
- Lifecycle badges use existing badge styles (similar to RM type badges).
- Metadata panel uses subtle background (e.g., `var(--color-surface)`) to distinguish from tree view.
- Monospace font for checksums, namespaces, UIDs (use `var(--font-mono)`).

### Performance Considerations

- XML parsing is lightweight (templates are typically < 500KB).
- Parsing happens only when OPT Tree tab is active and template is selected.
- No re-parsing on re-render; cache parsed metadata in component state.

---

## Success Metrics

**Quantitative:**

- **100% of templates with metadata** show at least one metadata field (author, lifecycle, or custodian).
- **Zero crashes** due to malformed OPT XML (error boundary handles gracefully).
- **< 50ms** parsing time for typical OPT (measured in dev tools performance profile).

**Qualitative:**

- Developers report **faster template evaluation** (reduced time to determine if a template is suitable).
- Clinical informaticists report **increased confidence** in template provenance.
- Positive feedback on metadata discoverability compared to raw XML inspection.

---

## Timeline & Prioritization

**Estimated effort:** 1–2 days (frontend-only, no backend changes).

**Prioritization:**
- **P0 (Must Have)**: Features 1 & 2 (metadata parsing and display panel).
- **P1 (Nice to Have)**: Feature 3 (dedicated `OptMetadata.vue` component for maintainability).

**Suggested milestone:** Include in next minor release (v0.3.0 or similar).

---

## Dependencies

- **PRD-0001** (Template Browser exists and fetches OPT XML).
- **No new backend work** — leverages existing `get_template_opt` command.
- **No new dependencies** — uses browser-native `DOMParser`.

---

## Open Questions & Risks

### Open Questions

1. **Should we display ALL `<other_details>` key-value pairs, or only known/common ones?**
   - **Proposed answer:** Display known keys (MD5, namespace, etc.) in structured format; show unknown keys in a generic "Other Details" list.

2. **Should lifecycle state badges be interactive (e.g., tooltip with definition)?**
   - **Proposed answer:** Yes, add tooltip explaining each state (defer to implementation phase).

3. **Should we parse `<languages>` and `<translations>` metadata?**
   - **Proposed answer:** Defer to future PRD; not critical for v1.

### Risks

- **Low risk:** Minimal new code; leverages existing data.
- **Edge case:** Some templates may have malformed or non-standard `<description>` elements. Mitigation: Robust error handling and fallback states.
- **Scope creep risk:** Stakeholders may request editing metadata (out of scope). Mitigation: Clearly communicate read-only nature in UI (no edit buttons).

---

## Alternatives Considered

### Alternative 1: Parse Metadata in Rust Backend

**Pros:** More robust XML parsing (use `quick-xml` crate); centralized logic.
**Cons:** Requires new backend command; increases data transfer (metadata sent separately); slower iteration.
**Decision:** Rejected. Frontend parsing is simpler and sufficient.

### Alternative 2: Show Metadata in a Separate Tab

**Pros:** Keeps OPT Tree tab clean.
**Cons:** Requires extra click to see metadata; reduces discoverability.
**Decision:** Rejected. Metadata is most useful alongside the tree view (e.g., seeing author while inspecting structure).

### Alternative 3: Only Parse Minimal Metadata (Author + Lifecycle)

**Pros:** Faster implementation.
**Cons:** Misses valuable technical metadata (checksums, parent refs).
**Decision:** Rejected. Full metadata parsing provides long-term value with minimal extra effort.

---

## Appendices

### Appendix A: Sample OPT Description XML

(From user-provided screenshot, `cistec.openehr.compound_document.v1`):

```xml
<description>
  <original_author id="name">Hendrik Heiser</original_author>
  <original_author id="organisation">Cistec AG</original_author>
  <original_author id="email">hendrik.heiser@cistec.com</original_author>
  <original_author id="date">2025-03-19</original_author>
  <other_contributors>Jonas Bucher (Reviewer)</other_contributors>
  <lifecycle_state>published</lifecycle_state>
  <other_details id="licence"/>
  <other_details id="custodian_organisation">Cistec AG</other_details>
  <other_details id="original_namespace">com.cistec</other_details>
  <other_details id="original_publisher">Cistec AG</other_details>
  <other_details id="custodian_namespace">com.cistec</other_details>
  <other_details id="MD5-CAM-1.0.1">0b1daab37a9a6d809d5dcc875960cddf</other_details>
  <other_details id="PARENT:MD5-CAM-1.0.1">005501C1FA493A483BF5F1121F2870EC</other_details>
  <other_details id="sem_ver">≥1.1.0</other_details>
  <other_details id="build_uid"/>
  <other_details id="Generated By">Archetype Designer v1.24.16, user=4027145</other_details>
</description>
```

### Appendix B: Lifecycle State Color Coding

| Lifecycle State | Badge Color | Hex Code |
|----------------|------------|----------|
| `published`    | Green      | `#28a745` |
| `draft`        | Yellow     | `#ffc107` |
| `deprecated`   | Orange     | `#fd7e14` |
| `obsolete`     | Red        | `#dc3545` |
| Other/Unknown  | Gray       | `#6c757d` |

---

## Revision History

| Version | Date       | Author              | Changes                          |
|---------|------------|---------------------|----------------------------------|
| 1.0     | 2026-04-05 | openEHR Explorer    | Initial draft                    |

---

**End of PRD-0008**
