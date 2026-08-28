# ADR-0020: CONTRIBUTION Audit Trail View

**Date:** 2026-08-25
**Status:** Accepted
**Repo:** `openehr-explorer`

---

## Context

[OEH-28](https://linear.app/platzh1rsch/issue/OEH-28) identified CONTRIBUTION support as a gap versus [ehr-ctrl](https://www.ehr-ctrl.info/), found during the competitor comparison refresh (OEH-13). ehr-ctrl exposes the openEHR `CONTRIBUTION` REST endpoint; openEHR Explorer did not.

A `CONTRIBUTION` is openEHR's audit-trail record of a single commit: who committed it, when, why (`change_type`/`description`), and which VERSIONed objects (a COMPOSITION, an EHR_STATUS, ...) were part of that one commit. The standard REST API exposes exactly one operation for it:

```
GET /ehr/{ehr_id}/contribution/{contribution_uid}
```

There is **no** "list contributions for an EHR" endpoint in the openEHR REST API spec — contributions can only be fetched by UID. This constrains the UX: the app cannot show a browsable contribution list the way it does for EHRs or compositions.

The backend already had an unused `get_composition_versions` command (revision history for a composition) with no frontend wired up to it, so composition version history wasn't visible anywhere in the app either.

## Decision

**1. Where it lives:** Contributions are surfaced within the existing EHR Browser detail panel, as a new **Contributions** tab alongside **Detail**/**JSON** — not as a new top-level nav item. A contribution is meaningless without its EHR context (the endpoint itself is EHR-scoped: `/ehr/{ehr_id}/contribution/{uid}`), and the app already treats compositions the same way (nested under the EHR, not top-level). A top-level "Contributions" nav item would imply a browsable list that the API cannot provide, which would be misleading.

**2. Discovery, given no list endpoint:** Two entry points, both landing on the same `/ehrs/:ehrId/contributions/:contributionUid` route (`ContributionViewer.vue`):
   - **Manual lookup** — a UID input in the EHR Browser's Contributions tab, for users who already know a contribution UID (e.g. from server logs or an external tool).
   - **From version history** — the composition Versions tab (also new; it's what finally wires up `get_composition_versions`) lists a composition's revision history with a "View Contribution" button per version. Clicking it calls a new `get_composition_version_contribution` command, which fetches the full VERSION object (`GET /ehr/{ehr_id}/versioned_composition/{uid}/version/{version_uid}`) to read its `contribution.id.value` — the lightweight revision-history response doesn't carry that reference, so a second, on-demand request is needed only when the user actually asks to see it.

**3. Versions touched:** The CONTRIBUTION response's `versions` array is rendered as a list of `(type, id)` pairs. Entries of type `COMPOSITION` get an "Open" link back into the existing Composition Viewer; other types (e.g. `EHR_STATUS`) are shown but not linked, since the app has no viewer for them yet.

## Consequences

- No auto-discovered "all contributions for this EHR" view exists, and cannot exist without a non-standard, server-specific extension. This is called out directly in the Contributions tab's UI copy so it doesn't read as a bug.
- The version-history round trip to resolve a contribution UID costs one extra HTTP request per lookup (only when the user clicks "View Contribution"), rather than eagerly fetching it for every row in the version list.
- Composition version history is now visible in the app for the first time (previously dead backend code), which is a useful side effect beyond the CONTRIBUTION scope itself.
