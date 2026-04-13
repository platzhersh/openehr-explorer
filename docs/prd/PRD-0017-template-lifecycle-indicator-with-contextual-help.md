# PRD-0017: Template Lifecycle Indicator with Contextual Help

Version: 1.0
Date: 2026-04-13
Status: Implemented
Owner: openEHR Explorer
Repo slug: openehr-explorer
Depends on: PRD-0001 (Desktop CDR Browser — Template Browser feature), PRD-0008 (OPT Metadata Display)

## Executive Summary

Enhance the Template Metadata panel in the Template Browser by adding a contextual help indicator (ℹ️ icon) next to the Lifecycle field. Clicking the icon opens a popover that explains the current lifecycle value, lists all spec-defined values in the openEHR AOM 2 lifecycle state machine, and — when the stored value does not match any recognised value — surfaces a non-blocking warning hinting that the OPT was likely produced by an older or non-conformant tool.

The feature serves openEHR learners and developers who encounter lifecycle values for the first time, without cluttering the UI for users who already understand the concept.

## Problem Statement

**Current State:**
The Template Metadata panel displays the lifecycle value as a plain badge (e.g. `RELEASE_CANDIDATE`, `Initial`). There is no explanation of what the value means, what the full set of valid values is, or whether a given value is spec-compliant.

**Pain Points:**
- An openEHR learner sees `RELEASE_CANDIDATE` or `Initial` and has no idea what governance stage this represents or what other stages exist.
- An openEHR developer uploading a community OPT (e.g., from Ripple or CKM snapshots) may find the value is `Initial` or even an empty string — produced by Ocean Template Designer or older tooling — with no indication that this is a known tooling quirk rather than a template defect.
- There is no in-app documentation linking lifecycle states to their meaning in the openEHR governance model.
- Developers building on top of openEHR Explorer (or learning from it) miss an opportunity to understand the template publication workflow.

**User Personas most affected:**
- **openEHR Learner** — needs a concise explanation of lifecycle semantics without leaving the app.
- **openEHR Developer** — needs to quickly understand whether a non-standard value is a tooling artefact or a genuine data quality problem.
- **Clinical Informaticist** — may care about governance readiness; `release_candidate` vs `published` has real-world implications for clinical deployment.

## Goals & Success Metrics

### Goals
- Make lifecycle state self-documenting within the Template Browser, with zero additional navigation required.
- Surface a non-blocking warning for non-spec values without alarming users unnecessarily.
- Keep the implementation lightweight — no new views, no backend changes, pure frontend.

### Success Metrics
- A first-time openEHR learner can explain all lifecycle states after reading the popover, without visiting external documentation.
- The warning indicator correctly identifies `Initial`, empty strings, and other legacy/non-standard values produced by common tooling.
- No regression in the Template Browser render time (the lookup is a pure in-memory string comparison).

## Feature Requirements

### 1. Lifecycle Info Icon
**Priority: P0 (Must Have)**

Next to the `Lifecycle:` label row in the Template Metadata panel, render a small ℹ️ icon button (matching the existing icon style used elsewhere in the app). The icon is subtle — muted colour by default, transitioning to the app's cyan accent (`#64ffda`) on hover.

### 2. Lifecycle Popover
**Priority: P0 (Must Have)**

Clicking the ℹ️ icon opens a popover (non-blocking, dismissible by clicking elsewhere). The popover contains:

- **Header:** "Template Lifecycle States"
- **Body:** A brief sentence explaining the role of the lifecycle field — that it records the governance/publication stage of the template as set by its author, and that the value is a free-text string in the OPT XML with no CDR enforcement.
- A structured list of the seven spec-defined values from openEHR AOM 2 / ADL 2, each with a one-line description:

| Value | Meaning |
| --- | --- |
| `unmanaged` | Not under any governance process; typically a working draft not yet submitted. |
| `in_development` | Actively being designed or revised; not ready for production use. |
| `release_candidate` | Proposed for release; undergoing final review. |
| `published` | Officially released and suitable for production use. |
| `superseded` | Replaced by a newer version; kept for historical reference. |
| `deprecated` | Still valid but actively discouraged; migration advised. |
| `obsolete` | No longer valid for use. |

- **Footer note:** A one-line callout explaining that EHRBase does not enforce lifecycle — any OPT can be uploaded and used regardless of its lifecycle value.
- **Footer link:** "Learn more →" linking to `https://specifications.openehr.org/releases/AM/latest/AOM2.html` (opens in system browser via `@tauri-apps/plugin-opener`).

### 3. Non-Standard Value Warning
**Priority: P1 (Should Have)**

When the stored lifecycle value does not case-insensitively match any of the seven spec-defined values, render the lifecycle badge in amber (rather than the default neutral colour) and append a small ⚠️ icon inline with the badge.

Hovering over the ⚠️ icon shows a brief tooltip:

> "This value is not part of the openEHR AOM 2 lifecycle vocabulary. It was likely set by an older authoring tool (e.g. Ocean Template Designer uses 'Initial'). The template is still usable."

The warning must not block any action — the user can continue to browse and use the template normally.

**Known non-standard values to expect in the wild:**
- `Initial` (Ocean Template Designer default)
- `AuthorDraft` (some older CKM exports)
- Empty string (missing value)

### 4. Recognised-Value Styling
**Priority: P2 (Nice to Have)**

For recognised spec values, apply a subtle semantic colour to the badge to reinforce meaning at a glance:

| Values | Colour hint |
| --- | --- |
| `published` | Green |
| `release_candidate` | Cyan/blue |
| `in_development`, `unmanaged` | Neutral/grey |
| `deprecated`, `superseded`, `obsolete` | Amber/muted |

This is additive to the existing badge — colour is a hint, not the sole signal. Non-standard values always take amber regardless of this table.

## Technical Design

### Component Changes
All changes are confined to `OptMetadata.vue` (where the lifecycle badge currently renders) and a new `LifecycleBadge.vue` sub-component.

**Lifecycle classification logic** — a pure TypeScript module at `src/lib/template-lifecycle.ts`:

```typescript
export const SPEC_LIFECYCLE_VALUES = [
  'unmanaged',
  'in_development',
  'release_candidate',
  'published',
  'superseded',
  'deprecated',
  'obsolete',
] as const;

export type SpecLifecycleValue = typeof SPEC_LIFECYCLE_VALUES[number];

export function classifyLifecycle(value: string | undefined | null): 'spec' | 'non-standard' | 'empty' {
  if (!value || value.trim() === '') return 'empty';
  if ((SPEC_LIFECYCLE_VALUES as readonly string[]).includes(value.toLowerCase())) return 'spec';
  return 'non-standard';
}
```

Matching is case-insensitive (`.toLowerCase()`) to handle both `RELEASE_CANDIDATE` (as surfaced by EHRBase) and `release_candidate` (spec canonical form).

**Popover implementation** — use a lightweight custom popover consistent with the app's existing UI patterns (click-outside dismiss via a global `mousedown` listener). No new dependency required.

### No Backend Changes
The lifecycle value is already present in the template metadata returned by the existing Rust `template.rs` command and parsed in `OptMetadata.vue`. No Tauri command changes are needed.

## Implementation Milestones

### Milestone 1 — Core ✅
- [x] Add `src/lib/template-lifecycle.ts` with `SPEC_LIFECYCLE_VALUES` and `classifyLifecycle()`
- [x] Create `src/components/LifecycleBadge.vue` with info icon and popover
- [x] Integrate `LifecycleBadge` into the Template Metadata panel in `OptMetadata.vue`
- [x] Render lifecycle state table and footer note in popover body

### Milestone 2 — Warning indicator ✅
- [x] Render amber badge + ⚠️ tooltip for non-standard/empty values
- [ ] Add unit tests for `classifyLifecycle()` — deferred: the project does not yet have a JS/TS test runner. The module is pure and trivially testable once a runner (vitest) is introduced.

### Milestone 3 — Semantic badge colours ✅
- [x] Apply colour coding for recognised spec values per the table above

## Acceptance Criteria

- [x] Clicking the ℹ️ icon next to Lifecycle opens a popover listing all seven spec-defined lifecycle values with descriptions.
- [x] The popover closes on click-outside.
- [x] A template with `lifecycle_state = "Initial"` renders an amber badge with ⚠️ icon.
- [x] Hovering the ⚠️ icon shows the tooltip explaining the tooling artefact.
- [x] A template with `lifecycle_state = "RELEASE_CANDIDATE"` renders with no warning (case-insensitive match).
- [x] A template with an empty `lifecycle_state` renders an amber badge with ⚠️ icon.
- [x] `classifyLifecycle()` is a pure, testable function.
- [x] No visual regression in the Template Metadata panel layout.
- [x] No new runtime dependency introduced.

## Risks & Mitigations

**Risk:** The openEHR spec lifecycle vocabulary evolves and new values are added.
**Mitigation:** `SPEC_LIFECYCLE_VALUES` is a single constant in one file — trivial to update. The warning is intentionally non-alarmist so a briefly out-of-date list does no real harm.

**Risk:** Popover positioning is awkward at small window sizes.
**Mitigation:** Popover is anchored to the info icon and constrained to the viewport width via `max-width` and flips layout with `max-height + overflow-y` as needed.

## Alternatives Considered

- **Inline info box (always visible):** Rendering the lifecycle explanation as a persistent blue info box below the metadata panel, similar to the existing "About OPT Tree View" box. Rejected in favour of the popover pattern — the explanation is reference material, not a primary concern on every template load. The popover keeps the metadata panel clean for experienced users while remaining one click away for learners.
- **External link only:** Adding only a "Learn more →" link to the AOM 2 spec. Rejected as the primary approach — requires leaving the app and navigating a large spec document to find the relevant section. The in-app table is the primary surface; the link is supplementary.
- **Modal instead of popover:** A modal would demand deliberate dismissal and feels too heavy for reference information. Popover is the right pattern for non-blocking contextual help.

## Related
- PRD-0001: Desktop CDR Browser (Template Browser feature where this lives)
- PRD-0008: OPT Metadata Display (where the lifecycle badge was first introduced)
- openEHR AOM 2 Specification: https://specifications.openehr.org/releases/AM/latest/AOM2.html
- openEHR Archetype Designer (successor to Ocean Template Designer — uses spec-compliant values by default)
