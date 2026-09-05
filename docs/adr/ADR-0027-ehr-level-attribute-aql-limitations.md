# ADR-0027: Working Around EHR-Level Attribute Limitations in AQL

**Date:** 2026-09-05
**Status:** Accepted
**Deciders:** Development Team
**Related:** ADR-0010 (AQL-Based Listing for EHR and Composition Browsing), PRD-0013 (AQL-Backed EHR Search), Linear OEH-56

## Context

ADR-0010 established AQL as the universal, CDR-agnostic way to list and query EHRs and compositions, in preference to REST list endpoints whose shape varies by CDR vendor. That decision assumed AQL's own documented capabilities — `SELECT`, `WHERE`, `ORDER BY`, `LIMIT`/`OFFSET` — would behave consistently across compliant CDRs. Building PRD-0013's EHR search feature against a real EHRBase instance disproved that assumption specifically for attributes read off the `EHR` root object itself (`e/ehr_id/value`, `e/time_created/value`, `e/system_id/value` — as opposed to attributes reached through a `CONTAINS`-ed archetype like `EHR_STATUS` or `COMPOSITION`), in three separate, independently-discovered ways:

1. **`WHERE` on `e/time_created/value`** — EHRBase rejects this outright with an explicit error: `"Not implemented: WHERE: identified path 'time_created/value' for type EHR not supported"`. Loud failure, easy to detect and handle (`build_ehr_search_aql` returns an error for `created-on`/`created-before`/`created-after` rather than sending an AQL query the CDR will reject).
2. **`ORDER BY e/time_created/value` / `ORDER BY e/ehr_id/value`** — also rejected outright, but with a *different* class of error message (`is_order_by_unsupported` in `src-tauri/src/commands/ehr.rs` pattern-matches on it), discovered while building the sortable EHR list view. Also loud, also detectable.
3. **`WHERE e/ehr_id/value LIKE '<prefix>%'`** — the one that took real user-reported behavior to catch: EHRBase returns **HTTP 200 with an empty result set**, silently, even when a matching EHR genuinely exists. No error, no signal that the predicate didn't apply — the query looks like it ran correctly and simply found nothing. Exact equality (`e/ehr_id/value = '<id>'`) on this same path, by contrast, works fine (`get_ehr_detail`'s composition lookup already relied on it before this was investigated).

Three limitations discovered independently, over three different features, is a pattern, not a coincidence: EHRBase's AQL implementation treats `EHR`-level attributes as second-class inside `WHERE`/`ORDER BY` — sometimes rejecting the query, sometimes silently not applying the clause — while the same attributes work fine in `SELECT`. Nothing in the openEHR AQL specification itself says this should be true; it's specifically an EHRBase implementation gap (confirmed by testing, not by reading EHRBase's own documentation of its limitations, which doesn't mention it).

## Decision

We will treat "an `EHR`-level attribute in a `WHERE` or `ORDER BY` clause" as **untrustworthy by default** for any new AQL query added to this codebase, and follow this fallback ladder rather than assuming the naive AQL translation of a filter/sort request will work:

1. **Prefer a REST endpoint the openEHR spec defines for the exact operation**, if one exists and is implemented consistently enough across our supported CDRs (e.g. `GET /ehr/{id}` for an exact-ID lookup — used instead of an AQL `WHERE e/ehr_id/value = ...` query wherever a full ID is already in hand, such as `get_ehr_detail`'s own EHR-status fetch).
2. **Exact equality on an EHR-level attribute in AQL is fine** — confirmed working for `e/ehr_id/value = '<id>'` even combined with a `CONTAINS` clause. Use it freely; the failure mode is specifically pattern-matching and ordering, not equality.
3. **For anything else (prefix/substring match, sorting) that maps to a `WHERE`/`ORDER BY` on an EHR-level path**, don't assume it works — verify against a real CDR before shipping. When it's confirmed broken:
   - If the CDR *errors* (case 1/2 above): catch the specific error and either fall back to an unsorted/unfiltered query with a clear "not supported by this server" signal to the user (`list_ehrs`'s `sort_applied: false`), or reject the request client-side with an explanit message before even sending it (`build_ehr_search_aql`'s date-filter rejection).
   - If the CDR *silently no-ops* (case 3 above, the dangerous one): don't rely on the predicate at all. Fetch via whatever AQL shape **does** work (typically an unfiltered, paginated `SELECT ... FROM EHR e LIMIT n OFFSET m`) and apply the filter **client-side** in Rust, bounded by a scan cap so a search on a huge, mostly-non-matching CDR doesn't scan indefinitely (`search_ehrs`'s `scan_for_ehr_id_prefix`, capped at `EHR_ID_SCAN_CAP` = 2000 rows).
4. **Document the discovery** in the relevant PRD's "Known Limitations" section, quoting the actual observed CDR behavior (error text, or "silently returns empty") — not just "AQL doesn't support X" — so the next person debugging a similar symptom on a different feature recognizes the pattern instead of re-diagnosing it from scratch. This ADR exists so that recognition can happen at the "three strikes" level, not just the per-feature level PRD-0013 already provides.

### What this explicitly does not mean

This is not a blanket "avoid AQL `WHERE`/`ORDER BY` on `EHR`" rule — most of AQL's `WHERE`/`ORDER BY` support, including on contained objects like `EHR_STATUS` and `COMPOSITION`, works as documented and is used throughout this codebase without incident (e.g. `s/subject/external_ref/id/value LIKE '%<value>%'` in the same `search_ehrs` feature). The limitation is specific to the `EHR` root class's own attributes, and even there, specific to certain operators (equality is fine, pattern-matching and ordering are not, on the CDR tested). Don't generalize this ADR into avoiding AQL predicates more broadly than the evidence supports.

## Consequences

### Positive
- A documented, named pattern ("EHR-level attribute in WHERE/ORDER BY: verify before trusting") that the next contributor hitting a fourth instance of this can recognize immediately, rather than rediscovering through the same user-bug-report → investigation cycle PRD-0013 went through for `ehr_id_prefix`.
- The exact-match-first, client-side-scan-fallback pattern (`scan_for_ehr_id_prefix`) is now precedent for any future EHR-level filter that turns out to have the same silent-no-op behavior, rather than each feature inventing its own workaround.
- A full EHR ID search is *more* reliable after this decision than the naive AQL approach would ever have been (a single exact-match query, correct regardless of CDR size) — the workaround for the broken case (partial prefix) doesn't come at the cost of the common case (a pasted, complete ID).

### Negative
- The client-side scan fallback (case 3) is inherently bounded and imperfect: `search_ehrs`'s partial-prefix scan can still miss a match beyond its 2,000-row cap on an unusually large CDR, and — as CodeRabbit flagged in PR #199's review, tracked in Linear OEH-56 — pages via `LIMIT`/`OFFSET` with no `ORDER BY`, which the AQL spec leaves undefined, without being able to add a stable order (the same EHRBase limitation this ADR is about makes `ORDER BY e/ehr_id/value` unavailable). This is accepted, not silently ignored, but it is a real residual correctness gap next to what a database-level `LIKE` on an indexed, ordered column would give for free.
- Every new EHR-level filter/sort feature now carries a testing tax: it must be verified against a real CDR (not just unit-tested as an AQL string) before being trusted, since the failure mode (silent no-op) won't be caught by a unit test that only checks the generated query text.
- Client-side filtering means fetching and discarding rows the CDR itself could have filtered, if only the predicate worked — real wasted network and CPU work in the broken cases, compared to the ideal (working `WHERE`/`ORDER BY`) case ADR-0010 assumed AQL would provide uniformly.

### Neutral
- This doesn't change ADR-0010's core decision (AQL over CDR-specific REST endpoints for listing) — it refines it with a documented exception for one specific, narrow class of AQL usage that didn't behave as ADR-0010 assumed.

## Alternatives Considered

### Assume the naive AQL translation works, fix each break as a one-off bug
This is what happened for `time_created` (PRD-0013's original "Known Limitations" section) and `ORDER BY` (`is_order_by_unsupported`) before this ADR — and it's exactly why the third instance (`ehr_id_prefix`) shipped broken and needed a user bug report to catch, since nothing connected the earlier two discoveries into a pattern worth checking against proactively.
- **Verdict:** rejected as an ongoing policy (each individual fix was still correct) — this ADR is what promotes "fix it when it breaks" into "check for it before shipping."

### Detect the EHRBase quirk generically and auto-rewrite affected AQL queries
E.g. a query-preprocessing layer that recognizes `WHERE <ehr-level-path> LIKE ...` and automatically rewrites it into a client-side-scan plan, without each call site (like `search_ehrs`) needing to know about the limitation.
- **Pros:** centralizes the workaround; a future EHR-level filter would get the safe behavior "for free."
- **Cons:** meaningful engineering investment (a real AQL-aware rewrite layer) for a limitation currently observed in exactly one CDR, on exactly one class of attribute, across two call sites — and the *correct* rewrite differs by case (equality: leave alone; pattern-match: scan-and-filter; `ORDER BY`: drop and flag unsorted), which a generic rewriter would need to know regardless.
- **Verdict:** rejected as premature — revisit if a fourth or fifth instance of this pattern appears and the case-by-case handling starts to feel repetitive rather than novel each time.

### Give up on AQL for EHR-level filtering, always use REST endpoints per CDR
Branch on `server_type` for every EHR-level filter/sort operation and use whatever proprietary REST parameters each CDR happens to expose (e.g. EHRBase's `GET /ehr?subject_id=&subject_namespace=`).
- **Pros:** would sidestep AQL's `EHR`-root limitations entirely, for CDRs where such REST parameters exist.
- **Cons:** reopens exactly the CDR-specific-adapter problem ADR-0010 rejected, and doesn't even fully solve it — no supported CDR exposes a REST parameter for an EHR ID *prefix* search, so the client-side scan fallback would still be needed regardless.
- **Verdict:** rejected — ADR-0010's reasoning still holds; REST is used opportunistically (case 1: an ID already in hand) but not as the general strategy.

## References

- ADR-0010 — AQL-Based Listing for EHR and Composition Browsing
- PRD-0013 — EHR Browser: AQL-Backed Attribute Search (Known Limitations section: `time_created`, `ehr_id_prefix`)
- Linear OEH-56 — tracks the residual `LIMIT`/`OFFSET`-without-`ORDER BY` pagination risk this ADR accepts rather than fully closes
- `src-tauri/src/commands/ehr.rs`: `is_order_by_unsupported`, `build_ehr_search_aql`/`build_ehr_search_aql_paged`, `scan_for_ehr_id_prefix`
- PR #199 — the `ehr_id_prefix` fix and its CodeRabbit review, which prompted this ADR
