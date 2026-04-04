# ADR-0011: Centralized Request Instrumentation via Tauri Events

**Date:** 2026-04-04
**Status:** Accepted
**Deciders:** openEHR Explorer contributors
**Related:** PRD-0005 (Request Inspector)

## Context

openEHR Explorer makes all CDR HTTP calls through the Rust backend using `reqwest`. Users have no visibility into what the app sends or receives — when a 422 error occurs during composition creation, they must switch to Postman or curl to debug. Every command module (`ehr.rs`, `composition.rs`, `template.rs`, `query.rs`, `server.rs`) independently constructs and sends HTTP requests, with no shared observability layer.

We need to decide how to capture request/response data for a new Request Inspector panel.

## Decision

We introduce a centralized `send_instrumented` function in `src-tauri/src/inspector.rs` that wraps every `reqwest` call in the application. Instead of calling `.send().await` directly on a `RequestBuilder`, all command modules call `send_instrumented(app, client, builder)` which:

1. Builds the request and captures method, URL, headers, and body before sending
2. Executes the request and measures wall-clock duration
3. Reads the full response body into a string (capped at 2 MB)
4. Emits a `cdr-inspector-entry` Tauri event carrying a serialized `RequestLogEntry`
5. Returns an `InstrumentedResponse` struct to the caller

The frontend receives entries via `listen()` from `@tauri-apps/api/event` in a dedicated Pinia store (`useInspectorStore`), completely decoupled from the command return path.

### Key design choices

**Tauri events over command return values.** Commands return domain-specific data (EHR lists, composition JSON, etc.). Piggybacking inspector data on command responses would require changing every command's return type and every frontend caller. Events decouple the inspector from the command contract — any future command gets instrumented by calling `send_instrumented` without touching its return type.

**Single instrumentation function over middleware.** `reqwest` supports middleware via `reqwest_middleware`, but that adds a dependency and changes the client construction pattern across all modules. A single wrapper function achieves the same result with less indirection. It also gives us explicit control over what gets captured (e.g., body truncation, header redaction).

**Header redaction by default.** `Authorization` and `Cookie` headers are replaced with `[REDACTED]` in emitted events. This prevents sensitive credentials from persisting in the frontend store or being accidentally exported. A future "Show sensitive headers" toggle can disable this per-session.

**Response body as String, not bytes.** All openEHR REST API responses are text-based (JSON, XML, plain text). Storing the body as a `String` simplifies frontend rendering and avoids base64 encoding overhead for the 99% case.

## Consequences

### Positive

- Every HTTP call in the app is automatically visible in the Inspector with zero additional frontend work per command
- Adding instrumentation to a new command requires only changing `.send().await` to `send_instrumented(...)` — a one-line change
- The inspector store and UI are completely independent of domain stores — no prop drilling, no coupling
- Sensitive headers are never exposed to the frontend by default

### Negative

- Response bodies are read into memory in full (up to 2 MB cap) before being returned to the caller, where they are parsed again from the string. This doubles memory usage per response. For the typical openEHR API response sizes (< 100 KB), this is negligible.
- The `InstrumentedResponse` struct changes the ergonomics of response handling — callers use `serde_json::from_str(&resp.body)` instead of `response.json().await`. This is a mechanical refactor across all command modules.
- The event emission is fire-and-forget (`let _ = app.emit(...)`) — if the frontend is not listening, entries are silently dropped. This is acceptable since the inspector is observational, not functional.

## Alternatives Considered

### 1. reqwest middleware crate
Would provide transparent interception but adds a dependency, changes client construction, and makes it harder to control redaction and truncation per-request.

### 2. Return inspector data alongside command results
Would avoid events but requires changing every command's return type to a wrapper like `Result<(T, RequestLogEntry), String>` and updating every frontend caller. High coupling, high churn.

### 3. Logging to file + frontend file reader
Would decouple completely but introduces file I/O latency, requires cleanup, and doesn't provide real-time streaming to the UI.
