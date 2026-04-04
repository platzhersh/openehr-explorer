# PRD-0006: AQL Editor — Monaco Integration & Autocomplete

**Version:** 1.0
**Date:** 2026-04-04
**Status:** Draft
**Owner:** openEHR Explorer (standalone — openEHR ecosystem)
**Repo slug:** `openehr-explorer`

---

## Executive Summary

Replace the current plain `<textarea>` in the AQL Runner with Monaco Editor — the engine powering VS Code — and layer on three tiers of AQL-aware autocomplete: static keyword completion, fixed openEHR RM path completion, and template-aware `aqlPath` completion driven by the Web Templates already cached by the Template Browser.

The result is an AQL editing experience that feels like a purpose-built query IDE rather than a raw text box, reducing the friction of writing correct AQL from scratch and eliminating the need to cross-reference the Web Template JSON manually.

---

## Problem Statement

**Current State:**

The AQL Runner uses a plain `<textarea>` with no syntax awareness. The query text is rendered in monospace font but receives no highlighting, no completion, and no structural feedback. Running the wrong query — due to a typo in a keyword, a stale alias, or a misremembered `aqlPath` — produces a cryptic EHRBase error that requires the developer to manually inspect the Web Template to find the correct path.

**Pain Points:**

- Developers must remember or look up AQL keyword syntax (`CONTAINS`, `ORDER BY`, `MATCHES`) without any in-editor prompting.
- `aqlPath` values like `/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude` must be copied by hand from the Template Browser tree or Web Template JSON.
- No indication of structural problems until the query is executed and an error is returned by EHRBase.
- The gap between the AQL Runner and the Template Browser forces constant context-switching.
- New openEHR developers have no in-app guidance on what paths are available on a given RM type.

**User Personas Affected:**

1. **openEHR Developer** — Writes ad-hoc AQL during development; benefits most from `aqlPath` autocomplete.
2. **openEHR Learner** — Doesn't know AQL syntax; keyword completion is essential for discoverability.
3. **Integration Engineer** — Needs to construct precise AQL for FHIR↔openEHR pipeline testing; benefits from all three completion tiers.

---

## Goals & Non-Goals

### Goals

- Replace `<textarea>` with Monaco Editor in `AqlRunner.vue` with zero regression in existing functionality (execute, save, load, Ctrl+Enter shortcut).
- Implement AQL syntax highlighting for keywords, RM type names, string literals, and path expressions.
- Implement three tiers of autocomplete (detailed in Feature Requirements).
- Introduce an explicit **Context Template** selector (Option A) so template-aware path completion knows which Web Template's `aqlPath` index to use.
- Document Option B (query inference) as a future upgrade path without implementing it.

### Non-Goals

- AQL semantic validation (i.e., flagging incorrect paths before execution) — out of scope for this PRD. Consider a future PRD.
- Full AQL grammar parser in TypeScript — keyword and path-based completion does not require a full parser.
- Autocomplete for the EHR Browser or Template Browser views — Monaco is scoped to the AQL Runner only.
- Replacing CodeMirror 6 as an option — Monaco is chosen for this PRD (see Alternatives Considered).

---

## Feature Requirements

### 1. Monaco Editor Integration

**Priority:** P0 (Must Have)

**User Stories:**
- As a developer, I want the AQL editor to highlight keywords so I can read queries at a glance.
- As a developer, I want the editor to behave like VS Code (keyboard shortcuts, bracket matching, multi-cursor) so I feel immediately productive.

**Functional Requirements:**

- Replace `<textarea>` in `AqlRunner.vue` with a Monaco Editor instance.
- Register AQL as a custom Monaco language (`aql`) with the following token rules:

| Token Category | Examples | Color role |
|---|---|---|
| Keywords | `SELECT`, `FROM`, `WHERE`, `CONTAINS`, `ORDER BY`, `LIMIT`, `OFFSET`, `AND`, `OR`, `NOT`, `AS`, `LIKE`, `MATCHES`, `EXISTS`, `TOP` | Keyword (blue) |
| RM Type Names | `EHR`, `COMPOSITION`, `OBSERVATION`, `EVALUATION`, `INSTRUCTION`, `ACTION`, `CLUSTER`, `ELEMENT`, `SECTION` | Type name (teal) |
| Aggregate functions | `COUNT`, `MAX`, `MIN`, `SUM`, `AVG` | Function (yellow) |
| String literals | `'...'`, `"..."` | String (orange) |
| Path expressions | `/data[at0001]/...`, `e/ehr_id/value` | Path (grey or default) |
| Comments | `-- ...` | Comment (muted) |

- Preserve existing Ctrl+Enter (and Cmd+Enter on macOS) shortcut to execute the query.
- Match the existing dark theme (`--color-bg`, `--color-surface`, `--color-border` CSS variables) — use Monaco's `defineTheme` to apply the Explorer's color palette.
- Editor height: resizable via a drag handle between the editor section and the results section (replaces fixed height). Minimum 120px, default 40% of the pane height.
- Expose the Monaco editor instance via a Vue `ref` so other components can call `editor.getValue()` and `editor.setValue()` programmatically (needed for "load saved query" and "clear" actions).

**Technical Notes:**
- Install `monaco-editor` via npm. Use the Vite plugin `vite-plugin-monaco-editor` or `@monaco-editor/loader` to avoid the Web Worker bundling complexity.
- In Tauri's WebView, Monaco Web Workers are sandboxed. Use `monaco-editor/esm/vs/editor/editor.worker` directly and disable worker-based features that are unavailable (e.g., TypeScript language server). AQL is a custom language so no language server worker is required.
- Wrap Monaco instantiation in a `onMounted` / `onBeforeUnmount` lifecycle pair in a `useMonacoEditor` composable (`src/composables/useMonacoEditor.ts`).

**Acceptance Criteria:**
- [ ] Monaco Editor renders in place of the `<textarea>` with no layout regressions.
- [ ] AQL keywords are highlighted in queries loaded from saved queries and typed live.
- [ ] Ctrl/Cmd+Enter executes the query.
- [ ] Dark theme matches the rest of the Explorer UI (no white flash, no default blue theme).
- [ ] Editor is resizable between 120px and full pane height.
- [ ] Existing save/load/clear actions continue to work.

---

### 2. Layer 1 — Keyword Autocomplete

**Priority:** P0 (Must Have)

**User Stories:**
- As a developer, I want keyword suggestions when I start typing in the AQL editor so I don't need to remember exact syntax.
- As a learner, I want to discover available AQL clauses from the editor itself.

**Functional Requirements:**

- Register a Monaco `CompletionItemProvider` for the `aql` language.
- Offer the following static keyword completions at any position where a keyword is valid:

```
SELECT    FROM      WHERE     CONTAINS   ORDER BY
LIMIT     OFFSET    AND       OR         NOT
AS        LIKE      MATCHES   EXISTS     TOP
DISTINCT  GROUP BY  HAVING    UNION
COUNT(*)  MAX()     MIN()     SUM()      AVG()
```

- Each completion item shows:
  - **Label:** the keyword (e.g., `SELECT`)
  - **Kind:** `Keyword`
  - **Documentation:** a one-line description of what the clause does (e.g., `SELECT — specifies which fields to return`)
  - **Insert text:** the keyword followed by a space (simple keywords) or a snippet for clauses with required structure (e.g., `ORDER BY ${1:path} ${2|ASC,DESC|}`)

- Completions are case-insensitive triggered: typing `sel`, `SEL`, or `Sel` all surface `SELECT`.

**Technical Notes:**
- This is pure static data — no server calls, no template loading required.
- Implement as a constant array of `monaco.languages.CompletionItem` objects in `src/lib/aql/keywords.ts`.

**Acceptance Criteria:**
- [ ] Typing `se` offers `SELECT` in the completion popup.
- [ ] Typing `con` offers `CONTAINS` and `COUNT(*)`.
- [ ] Each completion item has a documentation tooltip.
- [ ] Completions do not fire inside string literals (i.e., not inside `'...'`).

---

### 3. Layer 2 — openEHR RM Path Autocomplete

**Priority:** P1 (Should Have)

**User Stories:**
- As a developer, I want path suggestions when I type `e/` or `c/` so I don't have to look up EHR and COMPOSITION field names.

**Functional Requirements:**

- When the cursor is preceded by an alias token followed by `/`, offer RM path completions for the known RM type bound to that alias.
- RM paths are **static** — fixed by the openEHR Reference Model 1.1.0 specification, not fetched from the server.

**EHR paths (triggered by `e/`):**

| Path | rmType | Description |
|---|---|---|
| `e/ehr_id/value` | `String` | UUID of the EHR |
| `e/time_created/value` | `DV_DATE_TIME` | Creation timestamp |
| `e/system_id/value` | `String` | Owning CDR system identifier |
| `e/ehr_status/subject/external_ref/id/value` | `String` | Patient identifier |
| `e/ehr_status/subject/external_ref/namespace` | `String` | Patient ID namespace |
| `e/ehr_status/is_queryable` | `Boolean` | Whether EHR appears in AQL |
| `e/ehr_status/is_modifiable` | `Boolean` | Whether EHR accepts writes |
| `e/ehr_status/uid/value` | `String` | UID of the EHR_STATUS object |

**COMPOSITION paths (triggered by `c/` or whichever alias is bound to COMPOSITION):**

| Path | rmType | Description |
|---|---|---|
| `c/uid/value` | `String` | Composition UID |
| `c/name/value` | `String` | Composition name (template name) |
| `c/archetype_details/template_id/value` | `String` | Template ID |
| `c/archetype_details/archetype_id/value` | `String` | Root archetype ID |
| `c/archetype_details/rm_version` | `String` | RM version string |
| `c/context/start_time/value` | `DV_DATE_TIME` | Composition start time |
| `c/context/end_time/value` | `DV_DATE_TIME` | Composition end time |
| `c/context/location` | `String` | Clinical location |
| `c/context/setting/value` | `String` | Setting (e.g., "primary medical care") |
| `c/language/code_string` | `String` | ISO 639-1 language code |
| `c/territory/code_string` | `String` | ISO 3166-1 territory code |
| `c/composer/name` | `String` | Author name |

- Each path completion shows:
  - **Label:** the full path (e.g., `e/ehr_status/subject/external_ref/id/value`)
  - **Kind:** `Field`
  - **Detail:** the rmType (e.g., `String`, `DV_DATE_TIME`)
  - **Documentation:** description string as above

- Alias detection: parse the query text above the cursor for patterns like `FROM EHR e`, `FROM EHR e[...]`, `CONTAINS COMPOSITION c`, etc., using a lightweight regex — no full parser required.

**Technical Notes:**
- Implement alias extraction in `src/lib/aql/aliasParser.ts`.
- RM path tables are static constants in `src/lib/aql/rmPaths.ts`.

**Acceptance Criteria:**
- [ ] Typing `e/` after `FROM EHR e` triggers the EHR path completion list.
- [ ] Typing `c/` after `CONTAINS COMPOSITION c` triggers the COMPOSITION path list.
- [ ] Each completion shows the rmType as detail text.
- [ ] The alias parser handles both lowercase and uppercase RM type keywords.
- [ ] Path completions do not appear when cursor is inside a string literal.

---

### 4. Layer 3 — Template-Aware `aqlPath` Autocomplete

**Priority:** P1 (Should Have)

**User Stories:**
- As a developer, I want to select a template and then receive path completions from that template's web template tree when I type an alias bound to an OBSERVATION or other archetype-constrained RM type.
- As a developer, I want each path suggestion to show the human-readable clinical label (e.g., "Systolic") alongside the raw `aqlPath`.

**Functional Requirements:**

#### 4a. Context Template Selector (Option A — Implemented)

- Add a **Context Template** dropdown immediately above the Monaco editor in the AQL Runner UI.
- The dropdown is populated from `templateStore.templates` (the same list shown in the Template Browser).
- Default state: `— No template context —` (no template selected; Layer 3 completions are inactive).
- Selecting a template triggers `templateStore.fetchWebTemplate(serverId, templateId)` if not already cached.
- The selected template ID is stored in local component state (`contextTemplateId: Ref<string | null>`); it is not persisted across sessions in v1 (add persistence as a follow-up).
- A clear button (x) resets the selection to the default state.
- The dropdown is visually distinct from the query controls (smaller, labelled "Context Template") so it does not look like a query execution option.

#### 4b. Path Index Extraction

- When a template is selected and its Web Template JSON is available, build an `AqlPathIndex` by recursively walking the Web Template tree.
- Each index entry:

```typescript
interface AqlPathEntry {
  aqlPath: string        // e.g. /data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude
  label: string          // e.g. "Systolic"
  rmType: string         // e.g. "DV_QUANTITY"
  localizedName?: string // optional, from web template node
}
```

- The index is built by `extractAqlPathIndex(webTemplate: WebTemplate): AqlPathEntry[]` in `src/lib/aql/aqlPathIndex.ts`.
- This function reuses the Web Template tree traversal already used by `extractFlatPaths` in `src/lib/webtemplate.ts` — they differ only in what they extract from each node.

#### 4c. Completion Provider Extension

- Extend the Monaco `CompletionItemProvider` from Layer 2: when cursor follows an alias `/` and the alias is bound to a CONTAINS type with an archetype filter (e.g., `obs[openEHR-EHR-OBSERVATION.blood_pressure.v2]`), offer the `AqlPathIndex` entries as completions.
- If the alias is bound to COMPOSITION (or EHR) without an archetype filter, fall back to the Layer 2 static RM paths for that type.
- Each `AqlPathEntry` completion shows:
  - **Label:** the human-readable `label` (e.g., "Systolic")
  - **InsertText:** the full `aqlPath` (e.g., `/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude`)
  - **Detail:** the `rmType` (e.g., `DV_QUANTITY`)
  - **Documentation:** the full `aqlPath` displayed verbatim so the developer can see what will be inserted

**Acceptance Criteria:**
- [ ] The Context Template dropdown appears above the editor and lists all templates from the connected server.
- [ ] Selecting a template fetches and caches its Web Template if not already loaded.
- [ ] Typing `obs/` (where `obs` is bound to a specific archetype) offers the corresponding `AqlPathEntry` items.
- [ ] Each completion shows the human label as the primary text and the raw `aqlPath` in the documentation tooltip.
- [ ] Clearing the template context disables Layer 3 completions without affecting Layers 1 and 2.
- [ ] The path index is rebuilt when the template selection changes.

---

### 5. Option B — Query-Inferred Template Context (Future, Not Implemented)

**Priority:** P3 (Future Consideration — documented for roadmap awareness)

**Description:**

Option B eliminates the explicit Context Template dropdown by inferring the template from the query text itself. Two inference strategies:

**Strategy B1 — Template ID filter:** Parse the WHERE clause for a pattern matching:
```sql
WHERE c/archetype_details/template_id/value = 'IDCR - Vital Signs Encounter.v1'
```
Extract the string literal value, look it up in `templateStore`, and load the corresponding path index automatically.

**Strategy B2 — Archetype ID filter:** Parse CONTAINS clauses for patterns matching:
```sql
CONTAINS OBSERVATION obs[openEHR-EHR-OBSERVATION.blood_pressure.v2]
```
Extract the archetype ID, find all loaded Web Templates that include a node with that archetype ID, and use the first match (or prompt the user to disambiguate if multiple templates match).

**Why Not Implemented Now:**
- Both strategies require a regex-based or partial-grammar query parser that is non-trivial to get right for the full range of AQL clause orderings and whitespace variations.
- Strategy B2 can match multiple templates, requiring disambiguation UI.
- The Context Template dropdown (Option A) is explicit, predictable, and sufficient for the current user base.
- Option B is the natural follow-on when the alias parser (Layer 2) is mature enough to handle these patterns reliably.

**Migration Path:**
- Option B does not require changes to `AqlPathIndex` or the completion provider — only the trigger mechanism changes (from dropdown selection to query-text parsing).
- Option A and Option B can coexist: Option A as an override, Option B as the default inference.

---

## Technical Design

### New Files

```
src/
└── lib/
    └── aql/
        ├── language.ts          # Monaco language registration (tokenizer rules)
        ├── theme.ts             # Monaco theme definition (Explorer dark palette)
        ├── keywords.ts          # Layer 1: static keyword completion items
        ├── rmPaths.ts           # Layer 2: static EHR + COMPOSITION path tables
        ├── aliasParser.ts       # Alias-to-RM type extraction from query text
        ├── aqlPathIndex.ts      # Layer 3: Web Template -> AqlPathEntry[] extractor
        └── completionProvider.ts # Unified Monaco CompletionItemProvider (Layers 1-3)

src/
└── composables/
    └── useMonacoEditor.ts       # Monaco lifecycle (mount, dispose, getValue, setValue)
```

### Modified Files

```
src/views/AqlRunner.vue          # Replace <textarea> with Monaco container div
                                 # Add Context Template dropdown UI
src/components/AqlEditor.vue     # Replaced with Monaco-based implementation
src/stores/template.ts           # No changes required (fetchWebTemplate already exists)
```

### Dependency

```json
"monaco-editor": "0.52.2"
```

Monaco bundles are large (~4MB uncompressed). Use Vite's tree-shaking and manual worker configuration to keep the bundle close to 1MB for the AQL language only.

---

## Acceptance Criteria Summary

| # | Criterion | Priority |
|---|---|---|
| AC-01 | Monaco renders in AqlRunner.vue with AQL syntax highlighting | P0 |
| AC-02 | Dark theme matches Explorer UI palette; no white flash | P0 |
| AC-03 | Ctrl/Cmd+Enter executes query; all existing actions work | P0 |
| AC-04 | Editor is resizable between 120px and full pane height | P0 |
| AC-05 | Keyword completions fire on partial keyword input | P0 |
| AC-06 | Completions do not appear inside string literals | P0 |
| AC-07 | `e/` triggers EHR RM path completions | P1 |
| AC-08 | `c/` triggers COMPOSITION RM path completions | P1 |
| AC-09 | Context Template dropdown appears; populates from templateStore | P1 |
| AC-10 | Selecting a template fetches/caches its Web Template | P1 |
| AC-11 | `obs/` (archetype-bound alias) triggers template aqlPath completions | P1 |
| AC-12 | Layer 3 completions show human label as primary, aqlPath in docs | P1 |
| AC-13 | Clearing template context disables Layer 3; Layers 1-2 unaffected | P1 |
| AC-14 | aliasParser unit tests pass for >= 10 representative query shapes | P1 |
| AC-15 | Monaco bundle delta is < 1.5MB gzipped | P2 |
| AC-16 | Feature works on macOS, Windows, and Linux Tauri builds | P1 |

---

## Change Log

| Version | Date | Author | Changes |
|---|---|---|---|
| 1.0 | 2026-04-04 | openEHR Explorer Team | Initial draft |
