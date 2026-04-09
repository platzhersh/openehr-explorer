# PRD-0013: EHR Browser — AQL-Backed Attribute Search

**Version:** 1.1
**Date:** 2026-04-09
**Status:** Draft
**Owner:** openEHR Explorer (standalone — openEHR ecosystem)
**Repo slug:** openehr-explorer
**Depends on:** PRD-0001 (Desktop CDR Browser)

## Executive Summary

The EHR Browser currently filters the already-loaded (paginated) EHR list in-memory. This means a developer looking for a specific patient's EHR, or all EHRs modified on a given system, must page through the entire CDR one page at a time. For any CDR with more than a few dozen EHRs this is unworkable.

This PRD specifies replacing the current client-side filter with a proper AQL-backed server-side search supporting nine search dimensions: subjectId, subjectNamespace, ehrId, systemId, modifiable, hasCompositions, and three date-of-creation predicates (created-on, created-before, created-after). Every search fires a real AQL query against the CDR, and all requests are logged in the existing Request Inspector panel — making the EHR Browser a transparent teaching tool as well as a practical one.

## Problem Statement

### Current State

The EHR Browser (`src/views/EhrBrowser.vue`) loads a paginated list of EHRs via `GET /rest/openehr/v1/ehr` and runs a `String.includes()` filter locally over the visible page. The search field placeholder reads "Search by EHR ID or subject…" but only matches what is already in the DOM.

### Pain Points

- A CDR with 500+ EHRs (common in any real development environment seeded with synthetic data) requires dozens of page-turns to locate a specific patient.
- Subject ID is the natural identifier developers use (it mirrors the source system's patient identifier), but it is not loaded for every row — it is only available after fetching each EHR's status individually.
- There is no way to ask "show me all EHRs that have at least one composition" — a common filtering need when debugging ingestion pipelines.
- The search action produces no visible HTTP traffic, making it useless as a learning tool and impossible to debug.
- The current UX implies server-side search ("Search by EHR ID or subject…") but silently delivers client-side filtering — surprising and misleading.

### Why AQL

openEHR REST APIs expose a `GET /rest/openehr/v1/ehr` endpoint with limited filtering parameters (EHRBase supports `subject_id` and `subject_namespace`). The full set of attributes requested — including systemId, modifiable, and hasCompositions — is only addressable via AQL queries against the `EHR e` root class and its `e/ehr_status` sub-object. AQL is also the natural language of the CDR and the most instructive approach for the openEHR Explorer's educational mission.

## Goals & Success Metrics

### Goals

1. Allow developers to find any EHR on the server in under 5 seconds regardless of CDR size.
2. All search operations produce AQL queries visible in the Request Inspector.
3. Support the nine search dimensions identified by the requester: subjectId, subjectNamespace, ehrId, systemId, modifiable, hasCompositions, created-on, created-before, created-after.
4. Maintain backward compatibility: the existing unfiltered list view (no search terms entered) continues to use the current pagination approach.
5. The search UX must not feel like a query builder — it should feel like a search bar with typed attribute filtering.

### Success Metrics

- A developer can locate a specific EHR by subject ID on a 1,000-EHR CDR in under 5 seconds.
- All nine search dimensions produce syntactically correct AQL and return results on EHRBase 2.x.
- The Request Inspector logs every AQL query fired by the search, including the full query string and HTTP status.
- No regression to the existing paginated list view when no search terms are entered.

## Feature Requirements

### 1. Search Input — Attribute Syntax

**Priority:** P0 (Must Have)

The single search bar is retained but gains typed attribute syntax alongside free-text EHR ID matching.

#### Supported Syntax

| Input example | Interpretation |
|---|---|
| `fde80e0e-04eb-49a6…` | EHR ID prefix match (existing behaviour, now via AQL) |
| `subject:6f4b5848-4731-4e06` | Subject ID contains match |
| `namespace:patnr` | Subject namespace exact match |
| `system:dev.cistec.io` | System ID exact match |
| `modifiable:true` | EHR status is_modifiable = true |
| `modifiable:false` | EHR status is_modifiable = false |
| `hasCompositions:true` | EHR has at least one composition |
| `hasCompositions:false` | ⚠️ **Not supported** (AQL limitation) |
| `created-on:2026-03-12` | EHR created on that calendar day (00:00:00–23:59:59) |
| `created-before:2026-03-12` | EHR created strictly before 2026-03-12T00:00:00 |
| `created-after:2026-03-12` | EHR created strictly after 2026-03-12T23:59:59 |

Multiple terms can be combined with a space (implicit AND):

- `namespace:patnr modifiable:true`
- `subject:6f4b5848 hasCompositions:true`
- `created-after:2026-03-01 created-before:2026-03-31`
- `namespace:patnr created-on:2026-04-02`

`created-before` and `created-after` may be combined freely to express a date range. `created-on` may not be combined with `created-before` or `created-after` — if both are present, `created-on` takes precedence and the others are ignored with a warning in the help popover.

#### UX Details

- The placeholder text changes to: `"EHR ID, or subject:…  namespace:…  system:…  modifiable:…  hasCompositions:…  created-on:…"`
- A small `?` help icon next to the search bar opens a popover listing the full syntax reference table above, including date format notes.
- Unrecognised prefixes are treated as EHR ID prefix matches (safe fallback).
- Search is triggered on Enter key or after a 600 ms debounce. It is not triggered on every keystroke to avoid spamming the CDR.
- A "Clear" `×` button appears when the search field is non-empty; clicking it resets to the unfiltered paginated view.
- While a search is executing a spinner replaces the result count badge.
- Date values are validated client-side before the AQL is fired: must match `YYYY-MM-DD` (ISO 8601 date). Invalid values show an inline error: `"created-on: expects a date in YYYY-MM-DD format."` No AQL is fired.

### 2. AQL Query Construction

**Priority:** P0 (Must Have)

The Rust backend (`src-tauri/src/commands/ehr.rs`) gains a new Tauri command:

```
search_ehrs(profile_id: String, criteria: EhrSearchCriteria) -> Vec<EhrSummary>
```

`EhrSearchCriteria` is a typed struct (serialised from the Vue frontend):

```rust
pub struct EhrSearchCriteria {
    pub ehr_id_prefix: Option<String>,
    pub subject_id: Option<String>,
    pub subject_namespace: Option<String>,
    pub system_id: Option<String>,
    pub modifiable: Option<bool>,
    pub has_compositions: Option<bool>,
    pub created_on: Option<String>,      // YYYY-MM-DD, expands to >= day_start AND <= day_end
    pub created_before: Option<String>,   // YYYY-MM-DD, strict: < day_start
    pub created_after: Option<String>,    // YYYY-MM-DD, strict: > day_end
}
```

The backend resolves each date to ISO 8601 datetime boundaries before interpolating into AQL:

- `created_on: 2026-03-12` -> `>= '2026-03-12T00:00:00' AND <= '2026-03-12T23:59:59'`
- `created_before: 2026-03-12` -> `< '2026-03-12T00:00:00'`
- `created_after: 2026-03-12` -> `> '2026-03-12T23:59:59'`

If `created_on` is set, `created_before` and `created_after` are ignored (precedence rule documented in the help popover).

#### AQL Templates

The command builds the AQL dynamically from the active criteria. Base query:

```sql
SELECT
  e/ehr_id/value,
  e/time_created/value,
  s/subject/external_ref/id/value AS subject_id,
  s/subject/external_ref/namespace AS subject_namespace,
  s/is_modifiable AS modifiable,
  s/is_queryable AS queryable,
  e/system_id/value AS system_id
FROM EHR e
CONTAINS EHR_STATUS s
```

Predicate clauses are appended to a `WHERE` block based on which criteria are non-null:

| Criterion | AQL predicate |
|---|---|
| `ehr_id_prefix` | `e/ehr_id/value LIKE '<value>%'` |
| `subject_id` | `s/subject/external_ref/id/value LIKE '%<value>%'` |
| `subject_namespace` | `s/subject/external_ref/namespace = '<value>'` |
| `system_id` | `e/system_id/value = '<value>'` |
| `modifiable: true` | `s/is_modifiable = true` |
| `modifiable: false` | `s/is_modifiable = false` |
| `has_compositions: true` | EXISTS sub-query (see below) |
| `has_compositions: false` | NOT EXISTS sub-query (see below) |
| `created_on: 2026-03-12` | `e/time_created/value >= '2026-03-12T00:00:00' AND e/time_created/value <= '2026-03-12T23:59:59'` |
| `created_before: 2026-03-12` | `e/time_created/value < '2026-03-12T00:00:00'` |
| `created_after: 2026-03-12` | `e/time_created/value > '2026-03-12T23:59:59'` |

A `LIMIT 200` clause is always appended. If results are exactly 200, a banner is shown: "Showing first 200 results — refine your search to narrow down."

All AQL strings are built by string interpolation with parameterised escaping: values are single-quoted and any embedded single-quotes are doubled (`'` -> `''`). No raw user input reaches the AQL string unescaped.

### 3. Request Inspector Integration

**Priority:** P0 (Must Have)

Every `search_ehrs` call must log the following in the existing Request Inspector panel:

| Field | Content |
|---|---|
| Method | POST |
| URL | `<server_base_url>/rest/openehr/v1/query/aql` |
| Request body | `{ "q": "<full AQL string>" }` (pretty-printed, collapsible) |
| Status | HTTP status code (200 / 4xx / 5xx) |
| Duration | Wall-clock ms |
| Row count | Number of EHRs returned |

The AQL string shown in the Request Inspector is the final rendered query (with values interpolated), not the template.

### 4. Result Display

**Priority:** P0 (Must Have)

Search results replace the paginated list in the EHR list panel. Layout changes from the current state:

- Header changes from "EHRs" to "EHRs — Search Results (N)" when a search is active.
- A "Back to list" link appears below the header to return to the unfiltered paginated view.
- Each result row shows: EHR ID (truncated + copy button), Subject ID (if returned), Subject Namespace, Time Created.
- Clicking a result row opens the EHR Detail panel exactly as in the current paginated list.
- The `modifiable` and `queryable` flags returned by the AQL query pre-populate the EHR Detail panel without a separate `GET /ehr/{id}/ehr_status` round-trip.

### 5. Error Handling

**Priority:** P1 (Should Have)

| Error scenario | UI behaviour |
|---|---|
| AQL syntax error (CDR returns 400) | Inline error banner below search bar: "Search failed: <CDR error message>". Request Inspector row shows 400 + error body. |
| Network timeout | Inline error banner: "Search timed out. Check the server connection." |
| Empty result set | Empty state illustration with message: "No EHRs match your search." and a "Clear search" link. |
| Invalid modifiable value (not true/false) | Inline validation message before firing: "modifiable: expects 'true' or 'false'." No AQL fired. |
| Invalid date format (not YYYY-MM-DD) | Inline validation message before firing: "created-on: expects a date in YYYY-MM-DD format (e.g. 2026-03-12)." No AQL fired. |
| created-on combined with created-before/created-after | Warning badge in help popover: "created-on overrides created-before/created-after." Search proceeds using created-on only. |

### 6. Search History (Nice to Have)

**Priority:** P2 (Nice to Have)

- The last 10 search queries (as typed strings) are persisted in-memory for the session.
- A dropdown appears below the search bar when the user focuses it, showing recent searches.
- Clicking a history entry restores the search string and re-executes.
- History is not persisted to disk (session-only, no privacy concerns).

## Non-Goals

- Full AQL query builder UI — the search bar does not expose arbitrary AQL construction. Developers who need full AQL access use the AQL Runner.
- Composition-level search — searching within compositions (e.g., "EHRs where blood pressure > 140") is out of scope for this PRD.
- Sorting search results — results are returned in the order the CDR returns them (typically insertion order). Sorting is a future enhancement.
- Saved search profiles — persisting named searches to disk is deferred to a future PRD.

## Known Limitations

### hasCompositions:false Not Supported

The `hasCompositions:false` filter is **not currently supported** due to AQL limitations. AQL does not provide a clean way to query for "EHRs that do NOT contain any compositions" because:

1. The `NOT EXISTS` clause in AQL requires a correlated subquery, which EHRBase does not support in the expected syntax
2. AQL's containment model is designed for positive assertions (what IS contained), not negative assertions (what is NOT contained)
3. Alternative approaches (LEFT JOIN patterns, COUNT aggregations) are not supported by the AQL grammar

**Workaround:** Users who need to find EHRs without compositions can:
- Use `hasCompositions:true` to find EHRs *with* compositions, then infer the inverse from the full list
- Use the paginated list view and manually inspect composition counts
- Use external filtering after fetching all EHRs via the REST API

This limitation may be addressed in a future PRD if AQL or CDR implementations provide better support for negation queries.

## Technical Notes

### AQL vs REST Endpoint Trade-off

EHRBase's `GET /rest/openehr/v1/ehr` supports `subject_id` and `subject_namespace` query parameters natively, returning a proper paginated response. For the two-attribute case this REST call would be more efficient than AQL. However:

- It does not support systemId, modifiable, hasCompositions, or combined multi-attribute predicates.
- Using AQL uniformly means all search traffic is visible in the Request Inspector with a consistent representation.
- The educational value of showing AQL for every search is consistent with the tool's mission.

Therefore all search operations use `POST /rest/openehr/v1/query/aql` exclusively.

### EHRBase Compatibility

Tested target: EHRBase 2.x. The `CONTAINS EHR_STATUS s` clause and `s/subject/external_ref/...` paths are standard openEHR AQL and supported by EHRBase 2.2+.

### Frontend Changes

`src/views/EhrBrowser.vue` changes:

- The `searchQuery` ref changes from a simple string filter to a parsed `EhrSearchCriteria` object.
- A `parseSearchInput(raw: string): EhrSearchCriteria` utility function handles tokenisation.
- `loadEhrs()` (existing paginated load) is retained unchanged and called when searchQuery is empty.
- A new `searchEhrs(criteria: EhrSearchCriteria)` function invokes the new Tauri command and updates the result list.
- The existing `filteredEhrs` computed property is removed; results are always driven by the backend.

### Rust Backend Changes

`src-tauri/src/commands/ehr.rs` additions:

- `EhrSearchCriteria` struct (serde-deserializable from TypeScript).
- `build_ehr_search_aql(criteria: &EhrSearchCriteria) -> String` — pure function, unit-testable.
- `search_ehrs` Tauri command — constructs AQL, fires `POST /query/aql`, maps response to `Vec<EhrSearchResult>`.
- Escape utility `escape_aql_string(s: &str) -> String` — doubles single quotes.
- Unit tests for `build_ehr_search_aql` covering: single criterion, multi-criterion AND, empty criteria, SQL-injection-style inputs with embedded quotes, created-on alone, created-before + created-after range, and created-on + created-after conflict.

## Change Log

| Version | Date | Author | Changes |
|---|---|---|---|
| 1.0 | 2026-04-09 | openEHR Explorer Team | Initial draft |
| 1.1 | 2026-04-09 | openEHR Explorer Team | Added created-on, created-before, created-after search criteria; date boundary resolution; conflict rule; timezone open question |
