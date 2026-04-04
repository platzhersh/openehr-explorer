# PRD-0005: Request Inspector — Live CDR Traffic Panel

**Version:** 1.0
**Date:** 2026-04-04
**Status:** Draft
**Owner:** openEHR Explorer (standalone — openEHR ecosystem)
**Repo slug:** `openehr-explorer`

---

## Executive Summary

Add a persistent **Request Inspector** panel to openEHR Explorer that captures every HTTP call the app makes to the connected CDR instance — method, URL, request headers, request body, response status, response headers, and response body — and presents them in a structured, navigable format. The feature is modelled on the browser Network tab and Postman's Console, but specialized for the openEHR REST API: response bodies are not just pretty-printed JSON, they are rendered as interactive trees that understand openEHR's data model (compositions, archetypes, FLAT paths, DV types).

The Inspector is central infrastructure, not an add-on. Every action in the app — creating an EHR, uploading a template, committing a composition, running AQL — emits to the same log. The user always knows exactly what went over the wire.

---

## Problem Statement

### Current State

openEHR Explorer makes HTTP calls through Tauri's Rust backend (`reqwest`). The user sees the resulting data rendered in the UI but has no visibility into the underlying API conversation. When something goes wrong — a 422 from EHRBase, a malformed FLAT composition, an unexpected AQL result — the developer must open a terminal and replay the request with curl to understand what happened.

### Pain Points

- **No observability:** Users cannot see what URL was called, with what headers, or what the exact request body was. This is especially painful during template uploads and composition commits where serialization bugs are common.
- **Postman dependency:** Developers keep a parallel Postman collection open to debug EHRBase interactions, defeating the purpose of having a dedicated tool.
- **openEHR-specific opacity:** Raw JSON responses from EHRBase are verbose and semantically opaque. Viewing a composition response as a flat JSON blob (`"at0006"`, `"DV_QUANTITY"`) requires a mental model most users do not yet have.
- **AQL debugging gap:** AQL responses contain `resultSet` arrays of mixed-type rows. There is no way to trace which AQL statement produced which response in the current UI.

### User Personas & Jobs to Be Done

| Persona | Job to Be Done |
|---|---|
| openEHR Developer | "Show me exactly what I sent to EHRBase when this composition was rejected with 422." |
| Integration Engineer | "Let me copy the curl equivalent of this request to reproduce it in CI." |
| openEHR Learner | "Show me what a CREATE EHR request actually looks like — method, URL, body, response." |
| Clinical Informaticist | "Let me verify that the stored composition contains the right FLAT path values for this archetype element." |

---

## Goals & Success Metrics

### Goals

- Make every CDR HTTP interaction visible, inspectable, and reproducible without leaving the app.
- Render request and response bodies in multiple views (Raw JSON, Tree, FLAT paths) so developers can understand both the transport and the openEHR semantics simultaneously.
- Generate a `curl` equivalent for any captured request so it can be reproduced externally.
- Persist the log across navigation within a session; clear on server profile switch.

### Success Metrics

- A developer can find the exact request body that caused a 4xx response within 5 seconds of the error occurring.
- The "Copy as curl" action produces a command that reproduces the request verbatim when pasted into a terminal.
- 90% of response bodies are rendered as an interactive tree (not just raw JSON) without additional user action.
- Positive Discourse feedback: community members cite the Inspector as a differentiator vs. raw curl workflows.

---

## Feature Requirements

### 1. Inspector Panel — Layout & Persistence

**Priority:** P0

The Inspector is a **bottom drawer** that slides up from the bottom edge of the application window, similar to browser DevTools. It is always accessible regardless of which view is active (EHR Browser, Template Browser, AQL Runner, etc.).

```
┌──────────────┬────────────────────┬──────────────────────────┐
│  Left Panel  │   Center Panel     │    Right Panel           │
│  (nav)       │   (main content)   │    (detail)              │
│              │                    │                          │
│              │                    │                          │
│              │                    │                          │
├──────────────┴────────────────────┴──────────────────────────┤
│  ▼  Request Inspector                     [Clear] [⬆ Expand] │
├──────────────────────────────────────────────────────────────┤
│  Request Log (left, ~35%)  │  Request Detail (right, ~65%)  │
│                            │                                 │
│  POST /ehr  201  12ms  ●   │  [Request] [Response]          │
│  GET  /ehr/...  200  8ms   │                                 │
│  PUT  /composition  422 ●  │  ┌ Tree ┬ Raw ┬ FLAT ┐         │
│  GET  /template/...  200   │  │ ...  │     │      │         │
│                            │  └──────────────────┘         │
└────────────────────────────┴───────────────────────────────-┘
```

**Functional Requirements:**

- Drawer has three height states: **Collapsed** (tab bar only, ~32px), **Half** (split with main content, default), **Expanded** (full window height).
- Toggle with keyboard shortcut `Cmd+Shift+L` (macOS) / `Ctrl+Shift+L` (Windows/Linux).
- Drawer state (height) is persisted per-session in localStorage.
- A red dot badge on the collapsed tab indicates the most recent request resulted in a 4xx or 5xx.

---

### 2. Request Log — Entry List

**Priority:** P0

The left pane of the Inspector shows a scrollable, reverse-chronological list of all CDR requests made since the session started (or since the last manual clear).

**Each log entry displays:**

```
[METHOD]  [path fragment]           [STATUS]  [duration]  [indicator]
POST      /rest/openehr/v1/ehr      201       23ms        ●
GET       /rest/openehr/v1/ehr/a4…  200       11ms
PUT       /composition/a3b…::1      422       47ms        ● (error)
```

**Functional Requirements:**

- Method shown as a coloured badge: `GET` (blue), `POST` (green), `PUT` (amber), `DELETE` (red).
- HTTP status coloured: 2xx green, 3xx blue, 4xx orange, 5xx red.
- Duration in milliseconds (wall-clock time from Tauri command invocation to response received).
- URL truncated to path fragment only; full URL visible on hover tooltip and in detail panel.
- Each entry is selectable; clicking populates the detail panel on the right.
- Most recent entry is auto-selected and auto-scrolled into view.
- **Filter bar** above the list: filter by method (multi-select checkboxes), status class (2xx / 4xx / 5xx), and free-text search on URL path.
- **Clear button** removes all entries from the current log. A confirmation popover asks "Clear request history?" to prevent accidental loss.
- Entry count badge in the drawer header (e.g., "14 requests").

---

### 3. Request Detail Panel

**Priority:** P0

The right pane shows the full detail of the selected log entry, with two top-level tabs: **Request** and **Response**.

#### 3.1 Request Tab

Sections (collapsible):

**Summary**
```
Method:   POST
URL:      http://localhost:8080/rest/openehr/v1/ehr
Duration: 23ms
Time:     2026-04-04T14:32:11.421Z
```

**Request Headers** — key/value table, each row has a copy button.

**Request Body** — shown only when present (POST, PUT). Rendered in the same three-view tabs as response bodies (see §3.2).

**Copy as curl** — button that generates a complete `curl` command including `-X METHOD`, `-H` headers, and `-d` body. Copied to clipboard with one click. Output example:
```sh
curl -X POST http://localhost:8080/rest/openehr/v1/ehr \
  -H "Content-Type: application/json" \
  -H "Authorization: Basic ZWhy..." \
  -d '{"_type":"EHR_STATUS","is_modifiable":true,...}'
```

#### 3.2 Response Tab

Sections (collapsible):

**Summary**
```
Status:       201 Created
Content-Type: application/json
Size:         1.2 KB
```

**Response Headers** — key/value table with copy buttons.

**Response Body** — the centrepiece of the Inspector (see §4).

---

### 4. Response Body Renderer

**Priority:** P0

The response body is rendered in a **tabbed view** with three modes. The active tab is remembered per-session.

#### Tab 1: Tree View (default)

An interactive, collapsible/expandable tree that renders the JSON response body. Inspired by the composition tree in `CompositionTree.vue` but generalized to any JSON response.

**Tree node rendering rules:**

| JSON structure | Tree rendering |
|---|---|
| Object `{}` | Expandable node showing key count. Children are key → value pairs. |
| Array `[]` | Expandable node showing item count. Children indexed `[0]`, `[1]`, etc. |
| String | Leaf node, value shown inline. Copy button on hover. |
| Number / Boolean / null | Leaf node, value shown inline with type-specific colour (number: blue, boolean: amber, null: grey). |

**openEHR-aware enrichments** — when the response is detected as an openEHR object (presence of `"_type"` keys), the tree applies additional rendering:

- `"_type"` values are rendered as coloured pills (e.g., `COMPOSITION`, `OBSERVATION`, `DV_QUANTITY`, `CODE_PHRASE`) using the openEHR brand orange `#FF861C`.
- `"archetype_node_id"` values (e.g., `at0006`) are shown with a secondary label if a matching Web Template is cached for the current session — the human-readable name is shown in parentheses: `at0006 (Any event)`.
- `"value"` fields inside `DV_QUANTITY` nodes show the magnitude and units concatenated: `120 mm[Hg]`.
- `"value"` fields inside `DV_DATE_TIME` nodes are formatted as locale date-time strings.
- `"uid"` / `"object_id"` fields get a copy button.

**Performance:** Trees with > 500 nodes are rendered with virtual scrolling (only visible nodes are in the DOM). Nodes beyond depth 3 are collapsed by default.

**Controls:**
- `Expand All` / `Collapse All` buttons.
- `Collapse to depth…` dropdown (1, 2, 3, 4, All).
- Search box: highlights matching keys or values across the tree (case-insensitive substring match).

#### Tab 2: Raw JSON

Syntax-highlighted, read-only JSON editor (Monaco editor, same instance as the AQL editor). Line numbers shown. `Copy All` button in the top-right corner.

When the response body is not JSON (e.g., plain text error from EHRBase, OPT XML), the raw content is shown as monospace plain text without syntax highlighting.

#### Tab 3: FLAT View

Available only when the response body contains a composition (detected by `"_type": "COMPOSITION"` at root or inside a `resultSet` row).

Renders the composition as a FLAT path → value table:

```
openEHR-EHR-COMPOSITION.encounter.v1/context/start_time    2026-04-04T14:00:00Z
openEHR-EHR-COMPOSITION.encounter.v1/content[at0001]/...   120
...
```

- FLAT path derivation uses the same logic as `src/lib/webtemplate.ts`.
- Each row has a copy button for the path and for the value.
- Filter box narrows the list by path fragment.
- If no matching Web Template is cached, a notice is shown: "Load the template to enable FLAT path resolution."

---

### 5. Tauri Backend Integration

**Priority:** P0

All HTTP calls in the Rust backend must be routed through a shared instrumented HTTP client that emits an event to the frontend after each request completes.

**Rust-side changes:**

- Introduce a `RequestLogEntry` struct in a new `src-tauri/src/inspector.rs` module:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct RequestLogEntry {
    pub id: String,             // UUID v4
    pub timestamp: String,      // ISO 8601
    pub method: String,
    pub url: String,
    pub request_headers: HashMap<String, String>,
    pub request_body: Option<String>,
    pub status: u16,
    pub response_headers: HashMap<String, String>,
    pub response_body: Option<String>,
    pub duration_ms: u64,
}
```

- A `log_request` helper wraps every `reqwest` call: it records the wall-clock start time, executes the request, captures the response body as a `String` (up to a configurable cap, default 2 MB), and emits a `cdrinspector://entry` Tauri event carrying the serialized `RequestLogEntry`.
- The `log_request` helper is called from all command modules (`ehr.rs`, `composition.rs`, `template.rs`, `query.rs`, `server.rs`).
- Sensitive headers (`Authorization`, `Cookie`) are redacted to `[REDACTED]` in the stored entry. A global setting "Show sensitive headers" (off by default) disables this redaction for the session.
- Response body cap: bodies larger than 2 MB are truncated and a `body_truncated: true` flag is set. A notice is shown in the UI.

**Frontend-side changes:**

- A Pinia store `useInspectorStore` listens to `cdrinspector://entry` events via `listen()` from `@tauri-apps/api/event`.
- The store holds an ordered array of `RequestLogEntry` objects (capped at 500 entries; oldest entries are evicted when the cap is reached).
- The store is reset when the active server profile changes.
- Components access the store directly — no prop drilling.

---

### 6. Context Linking (Phase 2)

**Priority:** P2

When a request is initiated by a specific user action (e.g., clicking "Create EHR" in the EHR Browser), the resulting log entry is tagged with the originating context:

```rust
pub source_context: Option<String>, // e.g., "EhrBrowser::createEhr"
```

In the log list, entries show a subtle context tag badge: `EHR Browser`, `Template Upload`, `AQL Runner`, etc. This lets users filter the log to "only show requests from the AQL Runner" without reading URLs.

Implementation note: context is passed as an optional string parameter to `log_request` from each call site.

---

### 7. Export

**Priority:** P1

- **Export as HAR** — exports the current session log as an HTTP Archive (`.har`) file, compatible with browser DevTools and Postman import. Each `RequestLogEntry` maps to a HAR `entry` object.
- **Export as curl script** — exports all requests in the current session as a shell script of sequential `curl` commands with comments indicating the originating action.

Both exports are triggered from a dropdown menu in the Inspector drawer header.

---

## Non-Goals

- **Request replay / editing:** The Inspector is read-only. Users cannot modify and re-send a captured request from within the app (use the "Copy as curl" flow for that). A full request builder is out of scope for this PRD.
- **Persistent log across sessions:** The log is session-scoped. Clearing the app or switching profiles resets it. Long-term persistence to SQLite is not planned.
- **Network-level capture:** The Inspector captures requests made by the Tauri Rust backend only. It does not intercept arbitrary system network traffic.
- **WebSocket / SSE monitoring:** EHRBase does not currently expose WebSocket endpoints; this is not relevant.

---

## Technical Design — Key Decisions

### Why a bottom drawer rather than a dedicated route?

The Inspector must be accessible while the user is actively working in any other view. A route would require navigation away. A bottom drawer (like browser DevTools or JetBrains IDE tools) allows the user to see both the active view and the request log simultaneously, which is the primary use case (e.g., "I just clicked Create Composition — let me see what went to the CDR right now").

### Why emit Tauri events rather than a synchronous Tauri command response?

Commands in Tauri return a single value per invocation. Emitting an event decouples the Inspector from the command return path: the frontend command handler focuses on updating the UI with the result; the Inspector store independently receives the full raw request/response detail. This separation keeps individual command handlers clean and makes the Inspector trivially addable to any future command without changes to the caller.

### Tree renderer vs. Monaco for the default view

Monaco is already a dependency (AQL editor). However, using Monaco for JSON display requires disabling editing and managing large-document performance carefully. A purpose-built tree renderer gives better control over openEHR-specific enrichments (`_type` pills, `at` code labels, DV type formatting) and virtual scrolling for large compositions. Monaco is retained for the Raw JSON tab where syntax highlighting and line numbers are the primary value.

### Response body cap at 2 MB

EHRBase can return large AQL result sets. Storing unlimited response bodies in the Pinia store (JavaScript heap) would cause memory pressure. 2 MB covers all typical development payloads (compositions are typically 10–100 KB; AQL results up to 500 KB). Users hitting the cap are directed to export the raw HTTP response via curl.

---

## UI Mockup — Request Log Entry (annotated)

```
┌─────────────────────────────────────────────────────────────────────┐
│  ▼ Request Inspector  [14 requests]  [Filter ▾] [Clear] [Export ▾]  │
├──────────────────────────────┬──────────────────────────────────────┤
│ POST /ehr                201 │ ● Request  ○ Response                │
│ GET  /ehr/a4b…/status    200 │                                      │
│ POST /composition        422 │ Summary                              │
│ GET  /template/enc…      200 │  Method   POST                       │
│ GET  /ehr                200 │  URL      http://localhost:8080/…    │
│ POST /query/aql          200 │  Duration 23ms                       │
│                              │  Time     2026-04-04 14:32:11        │
│ [●=error indicator]          │                                      │
│                              │ Request Headers ▾                    │
│                              │  Content-Type   application/json     │
│                              │  Authorization  [REDACTED]           │
│                              │                                      │
│                              │ Request Body ▾                       │
│                              │  [Tree] [Raw] [FLAT]                 │
│                              │  ▶ {}  EHR_STATUS (2 keys)          │
│                              │    is_modifiable  true               │
│                              │    is_queryable   true               │
│                              │                                      │
│                              │ [Copy as curl]                       │
└──────────────────────────────┴──────────────────────────────────────┘
```

---

## Implementation Plan

### Milestone 1: Backend Instrumentation (Week 1)

- [ ] Create `src-tauri/src/inspector.rs` with `RequestLogEntry` struct and `log_request` wrapper.
- [ ] Instrument all existing commands in `ehr.rs`, `composition.rs`, `template.rs`, `query.rs`.
- [ ] Emit `cdrinspector://entry` Tauri events carrying serialized entries.
- [ ] Unit tests: verify entry is emitted with correct method, URL, status, duration.

**Deliverable:** Every CDR call produces a structured log entry in the Tauri event bus.

### Milestone 2: Inspector Store + Drawer Shell (Week 2)

- [ ] `src/stores/inspector.ts` — Pinia store with `entries`, `selected`, `filter` state; listens to Tauri events.
- [ ] `src/components/inspector/InspectorDrawer.vue` — bottom drawer with three height states; toggle shortcut.
- [ ] `src/components/inspector/RequestLog.vue` — scrollable entry list with method/status badges, duration, filter bar.
- [ ] Wire drawer into `App.vue` layout.

**Deliverable:** Drawer visible, log populates in real time as user interacts with EHR Browser.

### Milestone 3: Detail Panel — Request Tab (Week 3)

- [ ] `src/components/inspector/RequestDetail.vue` — tabbed Request / Response panel.
- [ ] Summary section, headers table, Copy as curl generator.
- [ ] Request body renderer (Raw JSON via Monaco, Tree via new `JsonTree.vue`).

**Deliverable:** Clicking a log entry shows full request detail including curl copy.

### Milestone 4: Response Body Tree Renderer (Week 4–5)

- [ ] `src/components/inspector/JsonTree.vue` — generic recursive tree component with virtual scrolling for large nodes.
- [ ] openEHR enrichments: `_type` pills, `at` code label resolution from cached Web Templates, DV type formatting.
- [ ] Collapse-to-depth controls, search/highlight.
- [ ] FLAT view tab (path/value table) for COMPOSITION responses.

**Deliverable:** Response bodies rendered as navigable, openEHR-aware trees.

### Milestone 5: Export + Polish (Week 6)

- [ ] HAR export (session log → `.har` file via Tauri `save_file` dialog).
- [ ] curl script export (session log → `.sh` shell script).
- [ ] Sensitive header redaction setting (Settings screen).
- [ ] Response body truncation notice at 2 MB.
- [ ] Error badge on collapsed drawer tab.
- [ ] Keyboard shortcut `Cmd/Ctrl+Shift+L`.

**Deliverable:** Feature-complete Inspector ready for v0.2.0 release.

---

## Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|---|---|---|---|
| Large composition responses (> 2 MB) degrade UI | High | Low | 2 MB body cap + truncation notice; virtual scrolling in tree |
| Virtual scrolling complexity for JsonTree | Medium | Medium | Use `vue-virtual-scroller` (already in ecosystem) rather than building from scratch |
| Sensitive credential leakage in exported HAR | High | Medium | Redact `Authorization` / `Cookie` by default in all exports; warn user before export |
| Rust `log_request` wrapper adds latency | Low | Low | Measure: body cloning is in-memory, event emission is async. < 1ms overhead expected |
| At code label resolution requires cached Web Template | Low | High | Graceful degradation: show raw `at0006` if template not cached, with a hint to load the template |

---

## Open Questions

1. **Inspector in v0.1.0 or v0.2.0?** The backend instrumentation (Milestone 1) is low-risk and could ship in v0.1.0 as a foundation, with the UI shipping in v0.2.0. This avoids delaying the initial release while ensuring the wiring is in place.

2. **Drawer vs. side panel?** The bottom drawer mirrors browser DevTools ergonomics and preserves horizontal space in the three-panel layout. An alternative right-side-panel approach would compress the existing detail panel. Bottom drawer is recommended; revisit if user feedback indicates a preference.

3. **Should the Inspector replace the need for a built-in Request Builder?** A future PRD could add request editing and replay (making the Inspector a full Postman equivalent). For now, "Copy as curl" is the escape hatch. The `RequestLogEntry` data model is designed to support replay without structural changes.

4. **AQL response FLAT view:** AQL `resultSet` rows may contain composition fragments. Should the FLAT tab attempt to render individual result rows as FLAT paths? Deferred to Phase 2 — the complexity of partial composition resolution is significant.

---

## Success Criteria

**PRD-0005 is successful if:**
- Every CDR request produces a visible log entry within 100ms of the response being received.
- "Copy as curl" produces a terminal-executable command that reproduces the exact request.
- Composition response bodies render as an openEHR-enriched tree (not just raw JSON) without any user configuration.
- The drawer can be opened, resized, and closed without interfering with the main three-panel layout.
- The Inspector adds zero visible latency (< 5ms overhead) to CDR calls as measured by the existing duration display.
- At least 2 openEHR Discourse members mention the Inspector as a reason they prefer openEHR Explorer over Postman for CDR development.

---

## Related

- PRD-0001: openEHR Explorer — Desktop CDR Browser (parent feature set)
- `src-tauri/src/commands/` — Rust command modules to be instrumented
- `src/components/CompositionTree.vue` — existing tree renderer (JsonTree.vue will share patterns)
- `src/lib/webtemplate.ts` — FLAT path utilities reused by FLAT view tab
- Postman Console — UX inspiration for log entry list
- Browser DevTools Network tab — UX inspiration for drawer layout and detail panel
- HAR 1.2 spec — https://w3c.github.io/web-performance/specs/HAR/Overview.html (used for Export as HAR implementation reference)
