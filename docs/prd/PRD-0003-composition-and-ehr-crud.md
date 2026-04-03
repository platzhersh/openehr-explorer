# PRD-0003: openEHR Explorer — Composition & EHR CRUD

**Version:** 1.1
**Date:** 2026-04-03
**Status:** Draft
**Owner:** openEHR Explorer (standalone — openEHR ecosystem)
**Repo slug:** `openehr-explorer`
**Depends on:** PRD-0001 (Desktop CDR Browser — MVP features)

---

## Executive Summary

Extend openEHR Explorer beyond read-only browsing to support full **Create, Update, and Delete** operations on both **EHRs** (patient records) and **Compositions** (clinical documents). The centrepiece is a **Web Template-driven form renderer** that generates a usable data-entry UI from any uploaded OPT — so developers can submit real compositions to their CDR without writing a single line of FLAT JSON by hand.

This turns openEHR Explorer from a passive inspection tool into an active development companion: test compositions can be created and destroyed in seconds, EHR lifecycle can be exercised without Postman, and FLAT path mappings become immediately verifiable through the form-to-CDR feedback loop.

**Form rendering** is implemented using **medblocks-ui** Web Components (see ADR-0002) rather than a custom-built renderer. This keeps scope manageable and leverages a battle-tested openEHR-native library.

**Better Platform** is supported in Beta in this release. Full, verified Better compatibility — including FLAT format dialect detection and API difference handling — is deferred to PRD-0003. See the [Better Platform Beta](#better-platform-beta) section for details.

---

## Problem Statement

### Current State (after PRD-0001 MVP)

PRD-0001 delivers a solid read-only explorer: connect to a server, browse EHRs, view compositions with template-aware labels, inspect templates, run AQL queries.

What it cannot do:

- Create a new EHR (patient record) from within the app
- Submit a composition to the CDR — the core write operation in any openEHR workflow
- Edit or delete existing compositions
- Manage the EHR lifecycle (mark as non-queryable, update subject identity, delete)

### Pain Points

- **Creating test data requires Postman / curl.** Crafting a valid FLAT payload manually, cross-referencing the Web Template for correct paths and types, is error-prone and slow. A single mistyped key silently fails or produces cryptic EHRBase errors.
- **No composition write path means no full round-trip.** Developers cannot verify that their template models data correctly until they build their own application layer.
- **EHR management is opaque.** Creating an EHR with a meaningful subject identity (MRN, NHS number) requires knowing the exact `POST /ehr` body schema.
- **Deleting test data is tedious.** No quick way to clean up test EHRs and compositions accumulated during development sessions.

### User Personas

(Inherited from PRD-0001; ranked by benefit from this PRD.)

1. **openEHR Developer** — Primary beneficiary. Can now complete a full template-to-CDR workflow inside the Explorer without external tooling.
2. **openEHR Learner** — Can submit their first composition interactively, seeing exactly which FLAT paths were sent and what the CDR returned.
3. **Clinical Informaticist** — Can validate that a template captures the intended data by filling in the form and verifying the stored composition.
4. **Integration Engineer** — Can generate known-good composition payloads for use in pipeline tests.

---

## Goals & Non-Goals

### Goals

- A developer can create a new EHR and submit a composition against a loaded template in under 2 minutes, with zero JSON editing.
- The form renderer handles at minimum: `DV_QUANTITY`, `DV_TEXT`, `DV_DATE_TIME`, `DV_CODED_TEXT`, `DV_BOOLEAN`, `DV_COUNT`.
- The FLAT payload sent to the CDR is shown in full before and after submission.
- EHR create, update (subject / status), and delete are available from the EHR Browser.
- Composition update (new version) and delete are available from the Composition Viewer.

### Non-Goals

- **Full clinical-grade form UX** — This is a developer tool. Forms are functional, not polished for clinical use.
- **Authoring new OPT/archetype templates** — Out of scope; use the Archetype Designer.
- **Offline / disconnected mode** — All CRUD operations require an active CDR connection.
- **Multi-record batch import** — Out of scope for this PRD.
- **Full verified Better Platform CRUD** — Deferred to PRD-0003.

---

## Feature Requirements

### Feature 1: Web Template Form Renderer (Composition Create)

**Priority:** P0 (Must Have)

#### 1.1 Entry Points

From the **Template Browser** (PRD-0001, Feature 4): each template row gains a **"New Composition"** button.

From the **EHR Detail panel**: **"Add Composition"** dropdown listing all templates available on the server.

#### 1.2 EHR Selection / Creation

A mandatory **EHR selector** rendered above the form:

- Dropdown showing existing EHRs (EHR ID + subject label if present)
- **"+ Create new EHR"** inline option — opens the EHR Create flow (Feature 3) in a sheet/modal; on success, auto-selects the new EHR
- Selected EHR ID shown persistently in the form header

#### 1.3 Form Rendering via medblocks-ui

The form is rendered using **medblocks-ui** `<mb-form>` Web Components (see ADR-0002). The app fetches the Web Template JSON from the CDR and passes it to medblocks-ui, which handles field generation, type-appropriate controls, value set dropdowns, and FLAT output.

Required **context fields** (rendered above the medblocks-ui form as plain Vue inputs, merged into the FLAT payload before submission):

| Field               | Control           | Default       |
| ------------------- | ----------------- | ------------- |
| `ctx/language`      | Select            | `"en"`        |
| `ctx/territory`     | Select (ISO 3166) | Server locale |
| `ctx/composer_name` | Text              | —             |
| `ctx/time`          | Date-time picker  | Now           |

Styling applied via CSS custom properties on the medblocks-ui host elements, maintained in `src/styles/medblocks-overrides.css`.

#### 1.4 FLAT Preview Panel

A resizable side panel (default collapsed) showing the **live FLAT JSON** assembled from ctx fields + medblocks-ui `mb-submit` output. Updates on every form change. **"Copy FLAT"** button copies to clipboard.

#### 1.5 Submission

1. Merge ctx fields with the medblocks-ui FLAT output.
2. `POST /rest/openehr/v1/ehr/{ehr_id}/composition` with `Content-Type: application/openehr.wt.flat.schema+json`.
3. On success: success banner with Composition UID + **"View Composition"** link.
4. On error: full CDR error body displayed in a scrollable, syntax-highlighted error panel.

**"Reset Form"** clears all fields without leaving the page.

#### 1.6 Draft Persistence

Form state (template ID + EHR ID + FLAT snapshot) is auto-saved to local app storage. On reopening: **"Resume previous draft?"** / **"Start fresh"** prompt.

---

### Feature 2: Composition Update & Delete

**Priority:** P0 (Must Have)

#### 2.1 Composition Update (New Version)

From the Composition Viewer, an **"Edit"** button opens the medblocks-ui form pre-populated with the existing composition's data.

**Pre-population:** Fetch the composition with `Accept: application/openehr.wt.flat.schema+json` and pass the returned FLAT object to medblocks-ui's `value` property. If the server does not support FLAT retrieval, show a dismissible warning: _"Could not load composition in FLAT format — starting with an empty form."_

**Template mismatch:** If the composition's `template_id` is not in the local cache, the app fetches the Web Template on demand via `GET /rest/openehr/v1/definition/template/adl1.4/{template_id}` (with a loading state). If the fetch fails, show an error with a link to the Template Browser to upload the missing template.

Submission calls `PUT /rest/openehr/v1/ehr/{ehr_id}/composition/{composition_uid}`. New version UID shown in success banner.

#### 2.2 Composition Delete

From the Composition Viewer action menu, behind a confirmation dialog:

> "This will create a deleted version of the composition. The EHR audit trail is preserved. Continue?"

Calls `DELETE /rest/openehr/v1/ehr/{ehr_id}/composition/{composition_uid}`. On success, navigates to EHR Detail with a toast notification.

---

### Feature 3: EHR Create

**Priority:** P0 (Must Have)

**Entry points:** EHR Browser toolbar **"+ New EHR"** button, and inline from the Composition Form EHR selector.

**Form fields** (hand-coded Vue — this is standard REST, not a clinical template):

| Field               | Type        | Notes                              |
| ------------------- | ----------- | ---------------------------------- |
| Subject Namespace   | Text        | e.g. `uk.nhs.nhs_number`, `ch.ahv` |
| Subject External ID | Text        | The actual identifier value        |
| Is Queryable        | Toggle      | Default: on                        |
| Is Modifiable       | Toggle      | Default: on                        |
| EHR ID (optional)   | Text (UUID) | Leave blank to let server generate |

`POST /rest/openehr/v1/ehr`. On success: new EHR at top of list, EHR ID displayed with one-click copy.

---

### Feature 4: EHR Update & Delete

**Priority:** P1 (Should Have)

#### 4.1 EHR Status Update

Inline form in EHR Detail panel to update `is_queryable`, `is_modifiable`, and subject external ref. `PUT /rest/openehr/v1/ehr/{ehr_id}/ehr_status` with `If-Match` header set to current version UID.

#### 4.2 EHR Delete

From EHR Detail action menu. Requires typing the EHR ID to confirm:

> ⚠️ "Deleting an EHR is permanent on most CDR implementations. This cannot be undone. Type the EHR ID to confirm."

`DELETE /rest/openehr/v1/ehr/{ehr_id}`. If the server does not support this operation, the full HTTP response is surfaced.

---

## Better Platform Beta

Better Platform is listed as a **supported server type** in the Server Profile form, and all read operations function against it. For CRUD operations introduced in this PRD, Better Platform support is explicitly labelled **"Beta"**.

**UI treatment:**

- A `⚠ Beta` badge appears next to "Better Platform" in the Server Profile server type selector.
- A dismissible info banner is shown at the top of every CRUD form when the active server is a Better Platform profile: _"CRUD operations on Better Platform are in Beta. Some operations may behave differently. Please report issues via GitHub Issues."_ The banner links to the PRD-0003 tracking issue.

**Known divergences (to be fully resolved in PRD-0003):**

- Better uses a STRUCTURED format variant rather than EHRBase's openEHR FLAT; the medblocks-ui FLAT output may require transformation before submission.
- Better's EHR management endpoints differ from the openEHR REST spec in some edge cases.
- Better returns different error response shapes, which the generic error panel may not parse cleanly.

Full Better Platform support — including automated server-type detection (ADR-0003), format adaptation, and verified end-to-end CRUD — is tracked in **PRD-0003**.

---

## UX & Interaction Design Principles

**Transparency first.** Every CRUD operation exposes a collapsible "Request / Response" panel with the exact HTTP method, URL, headers, and body.

**Non-destructive defaults.** Delete actions always require a confirmation step. EHR Delete requires typing the EHR ID.

**Failure is informative.** CDR error responses displayed in full, syntax-highlighted — never just an HTTP status code.

**FLAT is always visible.** The FLAT preview panel and the post-submission Request/Response panel make the mapping explicit at all times.

---

## Technical Notes

### medblocks-ui Integration

See **ADR-0002** for full rationale and integration approach. Summary:

```
npm install medblocks-ui
```

In `main.ts`:

```ts
import "medblocks-ui";
```

In templates:

```html
<mb-form :webTemplate="webTemplate" @mb-submit="onSubmit" />
```

All theme customisations maintained in `src/styles/medblocks-overrides.css` using documented CSS custom properties.

### EHRBase API Endpoints Used

| Operation              | Endpoint                                                                                                  |
| ---------------------- | --------------------------------------------------------------------------------------------------------- |
| Create EHR             | `POST /rest/openehr/v1/ehr`                                                                               |
| Get EHR Status         | `GET /rest/openehr/v1/ehr/{ehr_id}/ehr_status`                                                            |
| Update EHR Status      | `PUT /rest/openehr/v1/ehr/{ehr_id}/ehr_status`                                                            |
| Delete EHR             | `DELETE /rest/openehr/v1/ehr/{ehr_id}` _(EHRBase-specific)_                                               |
| Create Composition     | `POST /rest/openehr/v1/ehr/{ehr_id}/composition`                                                          |
| Get Composition (FLAT) | `GET /rest/openehr/v1/ehr/{ehr_id}/composition/{uid}` + `Accept: application/openehr.wt.flat.schema+json` |
| Update Composition     | `PUT /rest/openehr/v1/ehr/{ehr_id}/composition/{uid}`                                                     |
| Delete Composition     | `DELETE /rest/openehr/v1/ehr/{ehr_id}/composition/{uid}`                                                  |
| Get Web Template       | `GET /rest/openehr/v1/definition/template/adl1.4/{template_id}`                                           |

---

## Implementation Milestones

### Milestone 1 — EHR CRUD (Feature 3 + 4)

Hand-coded Vue forms for EHR create, status update, delete. No medblocks-ui dependency needed.
**Estimated scope:** 2–3 days.

### Milestone 2 — medblocks-ui Integration + Composition Create (Feature 1)

Integrate medblocks-ui, apply theme overrides (spike on CSS depth first), wire FLAT preview panel, implement submission with Request/Response panel, Better Beta banner.
**Estimated scope:** ~1 week.

### Milestone 3 — Composition Delete + Update (Feature 2)

Delete with confirmation. Edit with FLAT pre-population + on-demand Web Template fetch.
**Estimated scope:** 3–4 days.

### Milestone 4 — Draft Persistence + Polish

Draft save/resume. Improved error handling across server types.
**Estimated scope:** 2–3 days.

---

## Open Questions

1. **medblocks-ui CSS customisation depth.** How deeply can medblocks-ui be themed via CSS custom properties vs. requiring Shadow DOM piercing? Spike recommended at the start of Milestone 2 before committing to the design system integration.

2. **Terminology / value sets for external bindings.** `DV_CODED_TEXT` fields bound to SNOMED or LOINC won't have value lists in the Web Template. For v1: free text with code + display name pair. Terminology server integration (Snowstorm Lite) deferred to a future PRD.

---

## Success Criteria

- ✅ A developer can create an EHR, submit a composition against the IDCR Vital Signs template, verify it, and delete it — entirely within openEHR Explorer, without Postman or a terminal.
- ✅ The FLAT payload sent to the CDR is visible in full before and after each submission.
- ✅ Submission errors display the full CDR error body, not just an HTTP status code.
- ✅ Repeatable nodes work via medblocks-ui's native repeat controls.
- ✅ Better Platform CRUD shows the Beta warning badge and info banner consistently.
- ✅ Template mismatch on Edit triggers an on-demand Web Template fetch with a loading state.

---

## Related

- PRD-0001: openEHR Explorer — Desktop CDR Browser (prerequisite)
- PRD-0003: openEHR Explorer — Full Better Platform Support (deferred)
- ADR-0001: PRD and ADR Documentation
- ADR-0002: Use medblocks-ui for Web Template Form Rendering
- ADR-0003: Better Platform Server-Type Detection _(to be written alongside PRD-0003)_
- medblocks-ui: https://github.com/medblocks/medblocks-ui
- EHRBase REST API: https://ehrbase.readthedocs.io/en/latest/03_development/04_rest_api/index.html
- openEHR REST API spec: https://specifications.openehr.org/releases/ITS-REST/latest/overview.html
