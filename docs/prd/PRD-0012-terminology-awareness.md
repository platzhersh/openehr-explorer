# PRD-0012: Terminology Awareness in Template Browser and Composition Viewer

**Date:** 2026-04-05
**Status:** Tier 1-3 implemented
**Owner:** openEHR Explorer
**Effort estimate:** S (Tier 1-2: ~1 day) / M (Tier 3: ~3 days)
**Priority:** P1 (Should Have)

---

## Problem Statement

openEHR archetypes are deeply linked to external clinical terminology systems — primarily SNOMED CT and LOINC, but also ICD-10/11, ATC, EDQM, UCUM, and national extensions like SwissMedic vaccine codes. This linkage surfaces in two distinct ways:

1. **Term bindings** in archetype/template definitions: a node like "oxygen saturation measurement" is annotated with `SNOMED-CT 252465000`. This is metadata on the *structure* — it tells an integration layer what the concept means.

2. **`DV_CODED_TEXT` values referencing external terminologies**: a composition element (e.g. "causative agent" in an allergy archetype, or "vaccine product" in an immunization) stores a code from SNOMED-CT, ICD-10, or similar. The code is just a string like `91302008` without context.

Currently, openEHR Explorer renders both of these silently: term bindings are not surfaced in the Template Browser tree, and raw codes like `91302008` appear as-is in the Composition Viewer. This makes the tool less useful for:

- **Developers** learning what a template node *means* in clinical terms
- **Debuggers** trying to understand what a stored composition actually says
- **Demo audiences** who should see `"Septicaemia (disorder)"` not `91302008`

---

## Goals

- Surface terminology bindings in the Web Template tree inspector so developers immediately understand what external concept each node maps to
- Visually distinguish `DV_CODED_TEXT` nodes that expect *local* (archetype-internal) values from those requiring *external* terminology lookup — a critical distinction for SDK developers building FLAT compositions
- Optionally resolve stored codes to human-readable preferred terms in the Composition Viewer, via a configurable FHIR terminology server
- Remain lightweight: no mandatory external dependencies, no authoring/search widget (out of scope)

---

## Non-Goals

- Terminology *search* / autocomplete widget for data entry (belongs in Open CIS, not in a browser/inspector tool)
- Full validation of stored codes against a value set
- Offline SNOMED CT distribution / Snowstorm Lite deployment (that is an operator concern; the app just points at a URL)
- Support for every possible terminology — SNOMED CT and LOINC cover 95%+ of what appears in community archetypes

---

## Solution: Three Tiers

### Tier 1 — Terminology Badges in Template Tree (P0 within this PRD)

**What:** In `TemplateBrowser.vue` / `CompositionTree.vue`, when rendering a Web Template node, detect and display:

- A `TERMINOLOGY` badge when a `DV_CODED_TEXT` node has no local `list` (meaning: external lookup required). The badge shows the expected system if determinable (e.g. `SNOMED-CT`, `LOINC`).
- A `LOCAL` badge (or no badge) when a `DV_CODED_TEXT` node has a pre-populated `list` array (archetype-internal values, renders fine as a select today).

**Detection logic** (Web Template JSON):

```typescript
function classifyCodedTextNode(node: WebTemplateNode): TerminologyType {
  const inputs = node.inputs ?? []
  const codeInput = inputs.find(i => i.suffix === 'code' || i.suffix === undefined)

  if (!codeInput) return 'unknown'
  if (codeInput.list && codeInput.list.length > 0) return 'local'

  // External: check for defaultValue on terminology suffix
  const termInput = inputs.find(i => i.suffix === 'terminology')
  if (termInput?.defaultValue) return termInput.defaultValue as TerminologyType

  return 'external-unknown'
}
```

**Also surface term bindings**: the Web Template JSON does not include `term_bindings` from the OPT, but the app already has OPT XML access (drag-and-drop upload, `template.rs`). Parse `<term_bindings>` from the cached OPT and display them in the node detail panel.

**Affected files:**
- `src/lib/webtemplate.ts` — add `classifyCodedTextNode()` helper
- `src/components/CompositionTree.vue` — render badge in node row
- `src/views/TemplateBrowser.vue` — render badge in template tree node row
- `src-tauri/src/commands/template.rs` — expose `term_bindings` from parsed OPT XML

---

### Tier 2 — Code Resolution in Composition Viewer (P1 within this PRD)

**What:** In `CompositionViewer.vue` (Pretty pane), when a `DV_CODED_TEXT` value is rendered and its `terminology_id` is a known external system (SNOMED-CT, LOINC, ICD-10, etc.), attempt to resolve the code to a preferred term via a configurable FHIR terminology server.

**Terminology server URL resolution** follows the two-level hierarchy defined in PRD-0011 (Global Settings):

```
effective_url = profile.terminology_url   // per-profile override (optional)
               ?? global_settings.terminology_url   // global default
               ?? None  // resolution disabled
```

**Rust backend** (`src-tauri/src/commands/terminology.rs`): All terminology server calls are proxied through Rust.

**Caching:** In-memory `HashMap<(system, code), String>` in `AppState`, populated on first lookup per session.

**Lazy resolution:** Codes are resolved on hover/expand, not on composition load.

**Affected files:**
- `src-tauri/src/commands/terminology.rs` — new module with `lookup_code` command and in-memory cache
- `src-tauri/src/lib.rs` — register command
- `src/lib/terminology.ts` — Vue-side Tauri invoke wrapper
- `src/components/CompositionTree.vue` — lazy resolution on hover/expand for external coded values
- `src/views/CompositionViewer.vue` — integration

---

### Tier 3 — Terminology Browser (implemented, revised scope)

**What shipped** differs from the originally-sketched "Web Template Terminology Contract Panel" (a
panel embedded in the Template Browser listing all external-coded nodes). Instead, this tier became
a standalone **Terminology Browser** view (`/terminology`, `src/views/TerminologyBrowser.vue`) — a
general-purpose inspector for the FHIR terminology server already configured via PRD-0011's
`terminology_url` hierarchy, independent of any one template. Rationale: term bindings describe a
single code per node, not a bound value set, so there was no clean way to list "all coded nodes and
their legal values" from data the app already parses — a standalone query tool made better use of
the `$lookup`/`$expand`/`$validate-code`/`$subsumes` operations the terminology server already
speaks, and generalizes beyond any one template.

**Four operations**, each its own tab, all user-triggered (errors are surfaced, not swallowed —
unlike the passive `lookup_code` resolution from Tier 2):

- **Describe a Code** (`CodeSystem/$lookup`) — preferred term, designations (synonyms), and
  properties (e.g. `parent`, `inactive`) for a single code. Richer than Tier 2's bare display string.
- **Expand a Value Set** (`ValueSet/$expand`) — lists a value set's member concepts, with an optional
  text filter and a result cap; each row can jump straight into "Describe a Code" for that concept.
- **Validate Membership** (`ValueSet/$validate-code` / `CodeSystem/$validate-code`) — checks whether
  a code is a legal member of a value set (or of the code system itself, if no value set is given).
- **Test Subsumption** (`CodeSystem/$subsumes`) — the hierarchy relationship between two codes in the
  same code system (`equivalent` / `subsumes` / `subsumed-by` / `not-subsumed`).

**Deep link from Template Browser:** each row in the existing "Bound Concepts" panel (Tier 1) now
carries a "Describe →" link that opens the Terminology Browser with that binding's system/code
pre-filled and immediately resolved — closing the loop from "what does this node bind to" to "give me
everything the terminology server knows about that code."

**Affected files:**
- `src-tauri/src/commands/terminology.rs` — `describe_code`, `expand_valueset`, `validate_code`,
  `test_subsumption` commands, alongside the existing `lookup_code`
- `src/lib/terminology.ts` — Vue-side wrappers for the four new commands
- `src/views/TerminologyBrowser.vue` — the new view
- `src/main.ts`, `src/components/AppSidebar.vue` — route + nav entry
- `src/views/TemplateBrowser.vue` — "Describe →" deep link on each bound concept

**Explicitly out of scope**, unchanged from the original non-goals: no terminology *search*/autocomplete
widget wired into composition authoring (`CompositionForm.vue`), no `ConceptMap/$translate` support, no
offline terminology distribution.

---

## Implementation Plan

### Phase 1: Tier 1 — Badges

- [x] Add `classifyCodedTextNode()` to `src/lib/webtemplate.ts`
- [x] Render terminology badge in `TemplateBrowser.vue` and `CompositionTree.vue` node rows
- [x] Parse `<term_bindings>` from OPT XML in `template.rs`
- [x] Display term bindings in Template Browser node detail panel

### Phase 2: Tier 2 — Code Resolution

- [x] Implement `terminology.rs` with `lookup_code` Tauri command + in-memory cache
- [x] Add `terminology.ts` Vue utility with invoke wrapper
- [x] Integrate into Composition Viewer Pretty pane — resolve codes on hover/expand

### Phase 3: Tier 3 — Terminology Browser

- [x] Add `describe_code`, `expand_valueset`, `validate_code`, `test_subsumption` commands to `terminology.rs`
- [x] Add matching wrappers + types to `src/lib/terminology.ts`
- [x] Build `TerminologyBrowser.vue` (Describe / Expand / Validate / Subsumes tabs) and register the `/terminology` route + sidebar entry
- [x] Empty state when no terminology server is configured (global or per-profile)
- [x] Deep link from Template Browser's Bound Concepts panel into the Describe tab

---

## Acceptance Criteria

### Tier 1

- [x] `DV_CODED_TEXT` nodes with no local list show an `EXTERNAL` badge in the template tree, coloured distinctly from `LOCAL` coded nodes
- [x] Where the expected terminology is determinable from the Web Template, the badge shows the system name (e.g. `SNOMED-CT`)
- [x] Nodes with OPT term bindings show bound concepts in the detail panel
- [x] No external API calls are made; feature works fully offline

### Tier 2

- [x] `ServerManager.vue` shows an optional "Terminology server URL" field
- [x] When a URL is configured and a composition is opened, external coded values in the Pretty pane display as `{preferred term} [{system} {code}]`
- [x] When the terminology server is unreachable, the viewer falls back gracefully to displaying the raw code with no error state
- [x] Repeated codes within a session hit the in-memory cache, not the network

### Tier 3

- [x] The Terminology Browser is reachable from the sidebar and shows a clear empty state (with a link to Settings/Server Manager) when no terminology server is configured
- [x] "Describe a Code" returns the preferred term, designations, and properties for a code
- [x] "Expand a Value Set" lists a value set's member concepts with an optional text filter
- [x] "Validate Membership" reports whether a code is valid, against either a value set or the code system itself
- [x] "Test Subsumption" reports the hierarchy relationship between two codes
- [x] Each operation surfaces terminology-server errors (unreachable, 404, unconfigured) directly, rather than degrading silently — these are user-triggered queries, not passive resolution
- [x] Bound Concepts rows in the Template Browser link directly into "Describe a Code" with the binding pre-filled

---

## Related

- **PRD-0011**: Global Settings — defines the terminology URL hierarchy this PRD depends on
- **PRD-0001**: openEHR Explorer Desktop CDR Browser — parent PRD
