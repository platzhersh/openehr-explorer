# ADR-0030: Handling Vendor-Specific API Differences per `server_type`

**Date:** 2026-11-07
**Status:** Accepted
**Deciders:** Development Team
**Related:** ADR-0002 (medblocks-ui for form rendering), ADR-0010 (AQL-based listing), ADR-0027 (EHR-level AQL limitations), ADR-0011 (request instrumentation)

## Context

The app targets several CDRs through the openEHR REST API. Where the spec is precise and implemented consistently, one code path serves all of them. In practice each vendor deviates in small ways, and the deviations only show up against a real server, usually as an opaque 4xx.

Adding Better Platform (`cloud.better.care`) for composition creation exposed four independent deviations, each found by trial and error:

1. **Template example.** Better does not implement the standard Definition API `GET /definition/template/adl1.4/{id}/example` (it returns 404). It offers its own `GET <base_url>/rest/v1/template/{id}/example?format=FLAT&exampleFilter=INPUT`, outside the openEHR API root.
2. **FLAT media type.** EHRBase accepts only the older `application/openehr.wt.flat.schema+json`; Better rejects it with a 415 and lists `application/openehr.wt.flat+json` (the spec-current type, also used by FerroEHR) as accepted.
3. **Template id header.** The spec spells the header `openehr-template-id`. Better only recognises `openEHR-TEMPLATE_ID` (with an underscore) and ignores the `templateId` query parameter, answering 400 "Template id is missing".
4. **Web Template shape.** Better returns snake_case keys (`rm_type`, `in_context`, `node_id`, `min_op`) where EHRBase returns camelCase, and omits `name` on structural nodes such as `context` and `category`. medblocks-ui (ADR-0002) reads camelCase, so event groups rendered no fields, and nameless nodes rendered as the literal text "undefined".

These are the same kind of problem ADR-0027 describes for AQL. Without an agreed pattern, each fix risks becoming an ad-hoc `if server_type == ...` scattered through commands and views.

## Decision

Vendor differences are handled by **small, named, `server_type`-keyed functions at the boundary where the difference occurs**, never by forking views or stores per vendor.

1. **One function per deviation, keyed on `ServerType`.** Examples: `flat_composition_content_type` and `template_id_header` in `src-tauri/src/commands/composition.rs`. A `match` on `ServerType` (not a wildcard `if`) so adding a variant forces a decision. The default arm keeps the spec-correct behaviour.
2. **Spec-first, vendor fallback second.** Always try the standard openEHR endpoint first. If the vendor has its own endpoint for the same operation, call it only after the standard one fails (e.g. a 404 on the Definition API `/example`), and only for that `server_type`. Vendor endpoints are built from `base_url` (see CLAUDE.md), not from `api_root()`.
3. **Normalise vendor payloads at the boundary.** Shape differences in responses are fixed once, where the data enters the part of the app that expects one shape: `normalizeWebTemplate` in `src/lib/webtemplate.ts` converts snake_case keys to camelCase and fills missing node names before the template reaches medblocks-ui. Views and components see a single shape.
4. **Fail clearly on input the server can never accept.** Where the cause is user input rather than vendor behaviour (e.g. a bearer token containing a devtools ellipsis), validate when the value is saved and say what is wrong, instead of forwarding it to a server that answers with a generic 500.
5. **Verify against the real server, and record how.** A vendor workaround is only added after reproducing the failure against the actual CDR. The comment on the function states what was observed (error text, accepted media types) so the next reader can tell a still-needed workaround from a stale one.

Adding a new vendor quirk therefore means: write the keyed function (or the normalisation step), cover it with a unit test of the pure part, and add a row to the table below.

### Known vendor deviations

| Area | EHRBase / spec | Better Platform | Handled in |
|---|---|---|---|
| Template example | Definition API `/example` | own `/rest/v1/template/{id}/example` (fallback after 404) | `commands/template.rs` |
| FLAT media type | `...wt.flat.schema+json` (EHRBase), `...wt.flat+json` (spec, FerroEHR) | `...wt.flat+json` | `flat_composition_content_type` |
| Template id header | `openehr-template-id` | `openEHR-TEMPLATE_ID` | `template_id_header` |
| Web Template keys | camelCase, names present | snake_case, `name` missing on `context`/`category` | `normalizeWebTemplate` |

## Consequences

### Positive
- Each deviation is one small, testable, greppable function; the common path stays spec-correct.
- Views, stores and medblocks-ui stay vendor-agnostic.
- New CDRs follow a known recipe rather than inventing one.

### Negative
- The workarounds encode observed behaviour of hosted servers that can change without notice. A fix for one vendor can silently go stale.
- Several `match` arms are not covered by automated tests against a real CDR, since CI has no Better instance; they were verified manually.
- `normalizeWebTemplate` converts every plain snake_case key. A vendor adding a legitimately snake_case key that medblocks expects would be altered (opaque dictionaries such as `localized_names` are deliberately left alone).

### Neutral
- Complements ADR-0027, which applies the same "verify against the real CDR, handle per case" approach to AQL.

## Alternatives Considered

### Per-vendor adapter classes
A trait implemented per CDR, owning every request and response.
- **Pros:** all vendor behaviour in one place.
- **Cons:** most operations are identical across vendors, so each adapter would mostly delegate; heavy for four small deviations.
- **Verdict:** rejected for now; revisit if deviations multiply per vendor.

### Ad-hoc `if server_type == ...` at each call site
- **Cons:** scatters vendor knowledge, hides the default, easy to miss a call site (the template-id header initially covered create but not update).
- **Verdict:** rejected; the keyed-function rule exists to prevent this.

### Ask users to configure content types and headers per profile
- **Cons:** pushes CDR internals onto users who picked a `server_type` precisely so the app would know them.
- **Verdict:** rejected.

## References

- ADR-0002 medblocks-ui, ADR-0010 AQL listing, ADR-0027 AQL limitations
- `src-tauri/src/commands/composition.rs`: `flat_composition_content_type`, `template_id_header`
- `src-tauri/src/commands/template.rs`: `get_template_example` fallback
- `src/lib/webtemplate.ts`: `normalizeWebTemplate`
- `src-tauri/src/commands/server.rs`: `normalize_bearer_token`
