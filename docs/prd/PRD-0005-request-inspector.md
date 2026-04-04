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

- Method shown as a coloured badge: `GET` (blue), `POST` (green), `PUT` (amber), `DELETE` (red).
- HTTP status coloured: 2xx green, 3xx blue, 4xx orange, 5xx red.
- Duration in milliseconds.
- URL truncated to path fragment only; full URL visible on hover tooltip and in detail panel.
- Each entry is selectable; clicking populates the detail panel on the right.
- Most recent entry is auto-selected and auto-scrolled into view.
- **Filter bar** above the list: filter by method (multi-select checkboxes), status class (2xx / 4xx / 5xx), and free-text search on URL path.
- **Clear button** removes all entries from the current log.
- Entry count badge in the drawer header.

---

### 3. Request Detail Panel

**Priority:** P0

The right pane shows the full detail of the selected log entry, with two top-level tabs: **Request** and **Response**.

#### 3.1 Request Tab

Sections (collapsible):

**Summary** — Method, URL, Duration, Time.

**Request Headers** — key/value table, each row has a copy button.

**Request Body** — shown only when present (POST, PUT). Rendered in the same three-view tabs as response bodies.

**Copy as curl** — button that generates a complete `curl` command including `-X METHOD`, `-H` headers, and `-d` body. Copied to clipboard with one click.

#### 3.2 Response Tab

Sections (collapsible):

**Summary** — Status, Content-Type, Size.

**Response Headers** — key/value table with copy buttons.

**Response Body** — the centrepiece of the Inspector (see §4).

---

### 4. Response Body Renderer

**Priority:** P0

The response body is rendered in a **tabbed view** with three modes. The active tab is remembered per-session.

#### Tab 1: Tree View (default)

An interactive, collapsible/expandable tree that renders the JSON response body.

**Tree node rendering rules:**

| JSON structure | Tree rendering |
|---|---|
| Object `{}` | Expandable node showing key count. Children are key → value pairs. |
| Array `[]` | Expandable node showing item count. Children indexed `[0]`, `[1]`, etc. |
| String | Leaf node, value shown inline. Copy button on hover. |
| Number / Boolean / null | Leaf node, value shown inline with type-specific colour. |

**openEHR-aware enrichments** — when the response is detected as an openEHR object (presence of `"_type"` keys), the tree applies additional rendering:

- `"_type"` values are rendered as coloured pills using the openEHR brand orange `#FF861C`.
- `"archetype_node_id"` values show human-readable name in parentheses if a matching Web Template is cached.
- `"value"` fields inside `DV_QUANTITY` nodes show magnitude and units concatenated.
- `"uid"` / `"object_id"` fields get a copy button.

**Performance:** Nodes beyond depth 3 are collapsed by default.

**Controls:**
- `Expand All` / `Collapse All` buttons.
- Search box: highlights matching keys or values across the tree.

#### Tab 2: Raw JSON

Syntax-highlighted, read-only JSON. `Copy All` button. Non-JSON shown as plain text.

#### Tab 3: FLAT View

Available only when the response body contains a composition (detected by `"_type": "COMPOSITION"`).

Renders the composition as a FLAT path → value table with filter and copy buttons.

---

### 5. Tauri Backend Integration

**Priority:** P0

All HTTP calls in the Rust backend are routed through a shared instrumented HTTP client that emits an event to the frontend after each request completes.

**Rust-side changes:**

- `RequestLogEntry` struct in `src-tauri/src/inspector.rs`
- `send_instrumented` helper wraps every `reqwest` call
- Sensitive headers (`Authorization`, `Cookie`) redacted by default
- Response body cap: 2 MB, with `body_truncated` flag

**Frontend-side changes:**

- Pinia store `useInspectorStore` listens to `cdr-inspector-entry` events
- Capped at 500 entries; oldest evicted
- Store reset on server profile change

---

### 6. Context Linking (Phase 2)

**Priority:** P2

Entries tagged with originating context (e.g., "EhrBrowser::createEhr"). Deferred.

---

### 7. Export

**Priority:** P1

- **Export as HAR** — HTTP Archive file compatible with browser DevTools.
- **Export as curl script** — shell script of sequential `curl` commands.

Both triggered from Inspector drawer header dropdown. Deferred to Phase 2.

---

## Non-Goals

- Request replay / editing
- Persistent log across sessions
- Network-level capture
- WebSocket / SSE monitoring
