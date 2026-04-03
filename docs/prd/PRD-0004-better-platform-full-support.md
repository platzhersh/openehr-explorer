# PRD-0004: openEHR Explorer — Full Better Platform Support

**Version:** 1.0
**Date:** 2026-04-03
**Status:** Draft
**Owner:** openEHR Explorer (standalone — openEHR ecosystem)
**Repo slug:** `openehr-explorer`
**Depends on:** PRD-0001 (Desktop CDR Browser), PRD-0002 (Composition & EHR CRUD)

---

## Executive Summary

openEHR Explorer currently targets EHRBase as its primary CDR. Better Platform (Better Care) is listed as a supported server type in the Server Profile form, but CRUD operations introduced in PRD-0002 are explicitly marked **Beta** for Better, with known divergences documented and unresolved.

This PRD upgrades Better Platform from Beta to **fully supported**: verified CRUD, automated server-type detection, FLAT format dialect adaptation, and clear documentation of any remaining per-server differences. It also introduces a general **CDR Adapter** abstraction in the codebase so that future CDR support (Nedap, Cerner Millennium openEHR, etc.) can be added without touching core feature code.

---

## Problem Statement

### Why Better Platform matters

Better Platform (formerly Marand Think!EHR) is one of the two most widely deployed commercial openEHR CDR implementations alongside EHRBase. Several national programs (Slovenia, Norway, Northumbria NHS) run on Better. An openEHR developer tool that works fully only on EHRBase covers roughly half the real-world deployment landscape.

For the Explorer to be a genuinely useful ecosystem tool — and to attract community adoption from Better users — it needs to work as well on Better as it does on EHRBase.

### Known divergences (from PRD-0002 Beta observations)

1. **FLAT format dialect.** Better uses a "STRUCTURED" JSON variant in addition to the EHRBase-style FLAT format. The exact FLAT implementation differs in key handling for `DV_CODED_TEXT`, interval types, and context fields. medblocks-ui is primarily tested against EHRBase; its FLAT output may not be accepted by Better's composition endpoint without transformation.

2. **Web Template endpoint.** Better exposes Web Templates at a different path and may return a slightly different JSON structure (different property names, additional fields). The template fetching and parsing code needs to handle both shapes.

3. **EHR management endpoint differences.** Better's EHR endpoints are mostly openEHR REST spec-compliant but differ in some edge cases (e.g. EHR creation response shape, `ehr_status` versioning behaviour).

4. **AQL dialect.** Better supports a superset of AQL with proprietary functions. The AQL Runner (PRD-0001) should not break on Better AQL responses, but Better-specific functions are not autocompleted.

5. **Authentication.** Better Platform commonly uses a proprietary session-based auth (`/rest/v1/session`) in addition to Basic Auth. The Server Profile auth options need to accommodate this.

6. **Error response format.** Better returns errors in a different JSON shape than EHRBase. The generic error panel in PRD-0002 may not parse them cleanly.

7. **Server-type detection.** Currently the server type is manually selected in the Server Profile form. There is no automatic detection. A user who connects to a Better instance but selects "EHRBase" will encounter silent failures on CRUD operations.

---

## Goals & Non-Goals

### Goals

- All CRUD operations from PRD-0002 work correctly on Better Platform with no Beta caveat.
- The Server Profile form can **auto-detect** whether a connected server is EHRBase or Better Platform, and set the server type accordingly.
- The CDR Adapter layer isolates server-specific behaviour so core feature code calls a unified interface.
- The Better Platform Beta badge and banner (introduced in PRD-0002) are removed once this PRD ships.
- Better's session-based auth flow is supported in the Server Profile auth options.

### Non-Goals

- **Supporting Better's proprietary APIs** beyond what is needed for standard Explorer features (EHR browse, composition CRUD, template management, AQL). Better-only features (e.g. Better Studio, Better Sync) are out of scope.
- **Full AQL autocomplete for Better-specific functions** — the AQL Runner remains dialect-agnostic.
- **Supporting additional CDRs** beyond EHRBase and Better in this PRD (though the Adapter pattern introduced here enables it).
- **Certifying the Explorer against Better Platform's test suite** — functional correctness verified by manual testing and community feedback.

---

## Feature Requirements

### Feature 1: Automated Server-Type Detection

**Priority:** P0 (Must Have)

#### 1.1 Detection Logic

When a user adds a new Server Profile or clicks **"Test Connection"**, the app performs a detection probe after confirming basic connectivity. The probe sequence:

1. `GET /rest/openehr/v1/definition/template/adl1.4` (openEHR REST spec endpoint)
   - If response headers include `X-EHRBase-Version` → **EHRBase** detected.
   - If response is 200 and headers indicate Better → **Better Platform** detected.

2. If Step 1 is ambiguous, probe `GET /rest/v1/template` (Better's legacy endpoint).
   - If 200 → **Better Platform** (legacy API mode).

3. If both probes are inconclusive → server type remains **"Unknown / Generic openEHR REST"** and the user is shown a manual selector with a note.

**Detection result UI:**

- Auto-detected type is shown with a `✓ Auto-detected` label next to the server type field.
- User can manually override the auto-detected type if needed.
- Detection result is persisted in the Server Profile; it is not re-run on every connection unless the user clicks **"Re-detect"**.

See **ADR-0003** for the full detection strategy rationale and alternative approaches considered.

#### 1.2 Server Type Indicator

The active server name in the app header (PRD-0001) gains a small server type badge: `EHRBase`, `Better`, or `openEHR` (generic). This is visible at all times so users know which adapter is active.

---

### Feature 2: CDR Adapter Layer

**Priority:** P0 (Must Have)

Introduce a TypeScript **CDR Adapter** abstraction in the frontend codebase. All API calls from feature code go through the adapter interface rather than calling `fetch` directly with hardcoded EHRBase paths.

#### 2.1 Adapter Interface

```typescript
interface CdrAdapter {
  // EHR
  listEhrs(params: PaginationParams): Promise<EhrSummary[]>;
  createEhr(request: CreateEhrRequest): Promise<Ehr>;
  getEhrStatus(ehrId: string): Promise<EhrStatus>;
  updateEhrStatus(ehrId: string, status: EhrStatus): Promise<EhrStatus>;
  deleteEhr(ehrId: string): Promise<void>;

  // Composition
  createComposition(
    ehrId: string,
    flat: FlatComposition,
  ): Promise<CompositionRef>;
  getCompositionFlat(ehrId: string, uid: string): Promise<FlatComposition>;
  updateComposition(
    ehrId: string,
    uid: string,
    flat: FlatComposition,
  ): Promise<CompositionRef>;
  deleteComposition(ehrId: string, uid: string): Promise<void>;

  // Templates
  listTemplates(): Promise<TemplateSummary[]>;
  getWebTemplate(templateId: string): Promise<WebTemplate>;
  uploadTemplate(opt: File): Promise<void>;

  // AQL
  runAql(query: string): Promise<AqlResultSet>;
}
```

#### 2.2 Concrete Implementations

- `EhrBaseAdapter` — implements the interface using EHRBase REST API paths and EHRBase FLAT format.
- `BetterAdapter` — implements the interface using Better REST API paths, handles auth (Basic + session token), and applies any necessary FLAT dialect transformation before submission.

The active adapter is selected at connection time based on the detected (or manually set) server type in the Server Profile and injected via Vue's provide/inject. Feature components call adapter methods; they never reference `EhrBaseAdapter` or `BetterAdapter` directly.

#### 2.3 FLAT Dialect Normalisation

The `BetterAdapter` includes a `normaliseFlatForBetter(flat: object): object` function that transforms EHRBase-style FLAT output (from medblocks-ui) into the format Better's composition endpoint accepts. This transformation is documented inline and covered by unit tests with EHRBase↔Better FLAT fixture pairs.

---

### Feature 3: Better Session-Based Authentication

**Priority:** P0 (Must Have)

Better Platform supports (and sometimes requires) a proprietary session-based auth flow in addition to Basic Auth:

1. `POST /rest/v1/session?username={u}&password={p}` → returns `{ sessionId: "..." }`
2. Subsequent requests include `Ehr-Session: {sessionId}` header
3. Session must be renewed periodically or re-created on 401

**Server Profile form additions:**

- Auth method selector gains a **"Better Session"** option (shown only when server type is Better Platform)
- Fields: username + password (stored in OS keychain, never in plain config)
- Session refresh: automatic on 401, with a max of 3 retries before surfacing an auth error

The `BetterAdapter` manages session lifecycle transparently; feature code does not need to handle it.

---

### Feature 4: Better-Specific Error Parsing

**Priority:** P1 (Should Have)

Better returns errors in a different JSON structure than EHRBase. The generic error panel (PRD-0002) passes through the raw response body regardless, but the structured error display (field-level validation errors, human-readable message extraction) requires server-type-aware parsing.

The `BetterAdapter` includes an `parseErrorResponse(response: Response): CdrError` method that extracts a normalised `{ message, code, details }` object from Better's error shape. This is passed to the same error panel component, which renders it identically regardless of CDR.

---

### Feature 5: Remove Better Beta Badge

**Priority:** P0 (Must Have — exit criterion for this PRD)

Once Features 1–4 are implemented and manually verified against a Better Platform instance:

- Remove the `⚠ Beta` badge from the Better Platform server type selector.
- Remove the Beta info banner from CRUD forms when the active server is Better.
- Update the README and PRD-0002 status to reflect full Better support.

The verification checklist (to be completed before shipping):

- [ ] Create EHR on Better
- [ ] Submit composition (IDCR Vital Signs template or equivalent) on Better
- [ ] Retrieve composition in FLAT format on Better
- [ ] Edit composition on Better
- [ ] Delete composition on Better
- [ ] Update EHR status on Better
- [ ] List and inspect templates on Better
- [ ] Run an AQL query on Better
- [ ] Session auth flow works (create session, request with session header, auto-renew on 401)

---

## Technical Notes

### Detection Heuristics (for ADR-0003)

The detection logic must be resilient to servers that don't expose version headers. The following heuristics are evaluated in order:

| Signal                                                  | EHRBase | Better              |
| ------------------------------------------------------- | ------- | ------------------- |
| `X-EHRBase-Version` response header                     | ✓       | —                   |
| `X-Better-Version` or `server: better` header           | —       | ✓                   |
| `GET /rest/openehr/v1/definition/template/adl1.4` → 200 | ✓       | ✓ (same path)       |
| `GET /rest/v1/template` → 200                           | —       | ✓ (legacy endpoint) |
| Response body contains `"ehrbase"` in `serverInfo`      | ✓       | —                   |

If no signal is conclusive, type defaults to **Generic openEHR REST** (read-only browsing works; CRUD shows a manual type selector prompt before proceeding).

### Adapter Registration

```typescript
// In connection setup
const adapter =
  serverProfile.type === "better"
    ? new BetterAdapter(serverProfile)
    : new EhrBaseAdapter(serverProfile);

app.provide("cdrAdapter", adapter);
```

Feature components:

```typescript
const adapter = inject<CdrAdapter>("cdrAdapter")!;
const compositions = await adapter.listCompositions(ehrId);
```

---

## Implementation Milestones

### Milestone 1 — CDR Adapter Interface + EhrBase Refactor

Define the `CdrAdapter` interface. Refactor all existing API calls (EHR browse, composition viewer, template browser, AQL runner) to go through `EhrBaseAdapter`. No behaviour change — this is a pure refactor.
**Estimated scope:** 3–4 days.

### Milestone 2 — Server-Type Detection + Profile UI

Implement detection probe logic. Update Server Profile form with auto-detect, type badge, re-detect button. Add server type indicator to app header.
**Estimated scope:** 2–3 days.

### Milestone 3 — BetterAdapter: Auth + Read Operations

Implement `BetterAdapter` with session auth and read operations (list EHRs, list compositions, get composition, list templates, get Web Template, run AQL). Manually verify against a Better Platform instance.
**Estimated scope:** ~1 week.

### Milestone 4 — BetterAdapter: CRUD + FLAT Normalisation

Implement CRUD methods in `BetterAdapter`. Implement `normaliseFlatForBetter`. Implement Better error parsing. Manually verify full CRUD checklist.
**Estimated scope:** ~1 week.

### Milestone 5 — Remove Beta Badge + Documentation

Complete the verification checklist. Remove Beta badge and banner. Update README, PRD-0002 status, and changelog.
**Estimated scope:** 1–2 days.

---

## Open Questions

1. **Better Platform test instance availability.** The full verification checklist requires access to a running Better Platform instance. Better offers a community/trial environment — confirm availability before scheduling Milestones 3–5.

2. **Better FLAT dialect specification.** The exact transformation rules for `normaliseFlatForBetter` need to be derived empirically by comparing EHRBase and Better FLAT examples for the same composition. A dedicated spike against a Better instance should produce the fixture pairs needed for unit tests.

3. **Better Web Template JSON shape differences.** Does Better return a Web Template JSON that medblocks-ui can consume directly, or does it require normalisation? This needs to be verified in Milestone 3 before CRUD work begins.

4. **Better EHR Delete.** Better may or may not support `DELETE /ehr/{ehr_id}`. If not, the `BetterAdapter.deleteEhr()` method should surface a clear "Not supported on this server" message rather than a generic HTTP error.

---

## Success Criteria

- ✅ Auto-detection correctly identifies EHRBase vs. Better Platform on first connection for at least 3 known server configurations.
- ✅ All items on the CRUD verification checklist pass against a live Better Platform instance.
- ✅ The Better Beta badge and banner are removed from the UI.
- ✅ A Better Platform user can use the Explorer without being aware of any server-specific code paths — the experience is identical to EHRBase.
- ✅ The `CdrAdapter` interface is documented in the contributor guide so that adding a third CDR is a self-contained task.

---

## Related

- PRD-0001: openEHR Explorer — Desktop CDR Browser
- PRD-0002: Composition & EHR CRUD (introduces Better Beta)
- ADR-0002: Use medblocks-ui for Web Template Form Rendering (medblocks-ui + Better FLAT dialect is a risk tracked here)
- ADR-0003: Better Platform Server-Type Detection Strategy _(to be written alongside this PRD)_
- Better Platform REST API docs: https://better-care.github.io/better-ehrstudio/rest-api/
- openEHR REST API spec: https://specifications.openehr.org/releases/ITS-REST/latest/overview.html
