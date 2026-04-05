# PRD-0011: Global Settings Page

**Date:** 2026-04-05
**Status:** Draft
**Owner:** openEHR Explorer
**Effort estimate:** S (~1 day)
**Priority:** P1 (Should Have)

---

## Problem Statement

openEHR Explorer currently has no global, app-level configuration. All settings are scoped to individual server profiles. This works for CDR-specific settings (URL, auth method) but breaks down for settings that are:

- **App-wide defaults** — sensible values that apply unless a profile overrides them
- **Cross-cutting** — not tied to any single CDR instance (e.g. UI preferences, terminology server)
- **Pre-flight** — need to be configured before any server profile is added

The first concrete need is a **default terminology server URL** (introduced in PRD-0012). Rather than baking it into the server profile form as yet another field, a Global Settings page establishes the right home for app-level configuration and makes future settings (display preferences, pagination defaults, proxy settings, etc.) easy to add.

---

## Goals

- Introduce a Global Settings page reachable from the main navigation
- Persist settings to disk (alongside existing `profiles.json`, or a separate `settings.json`)
- Define a clear two-level override hierarchy: **Global → Profile** (profile wins if set)
- Ship with one initial setting: **Default Terminology Server URL**
- Design the settings schema to be extensible without migration pain

---

## Non-Goals

- Full preferences system (fonts, themes, keyboard shortcuts) — can be added later
- Cloud sync of settings
- Per-template or per-composition settings

---

## Settings Hierarchy

```
Global Settings (settings.json)
  └── terminologyServerUrl: "https://tx.fhir.ch/r4"   ← app-wide default

Server Profile (profiles.json, per profile)
  └── terminologyServerUrl?: "http://localhost:8083"   ← overrides global if set
```

Resolution at runtime:

```
effective_url = profile.terminology_url
               ?? global_settings.terminology_url
               ?? None   // resolution disabled
```

The global default ships pre-populated with `https://tx.fhir.ch/r4` so terminology resolution works out of the box for new installs without any configuration required.

---

## UI Design

### Navigation

Add a **Settings** entry to the left nav panel (gear icon, bottom-pinned, below the existing nav items). Routes to `/settings`.

### Page Layout

Single-page form, no tabs needed for v1. Sections separated by headings as the page grows.

```
┌─ Global Settings ──────────────────────────────────────┐
│                                                         │
│  Terminology                                            │
│  ──────────────────────────────────────────────────     │
│  Default terminology server URL                         │
│  [ https://tx.fhir.ch/r4                          ]     │
│  Used for resolving external codes (SNOMED CT,          │
│  LOINC, etc.) unless a server profile overrides it.     │
│  Leave empty to disable code resolution globally.       │
│                                                         │
│  [ Save ]                                               │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

The `ServerManager.vue` profile form gets a new optional field:

```
  Terminology server URL  (optional — overrides global default)
  [ _________________________________________________ ]
  Leave empty to use the global default.
```

When empty, the profile inherits the global setting. The effective URL is shown as placeholder text: `placeholder="Using global default: https://tx.fhir.ch/r4"`.

---

## Data Model

### `settings.json` (new file, `~/.config/openehr-explorer/settings.json`)

```json
{
  "version": 1,
  "terminology_server_url": "https://tx.fhir.ch/r4"
}
```

`version` field allows future migrations. Start at 1.

### `ServerProfile` struct amendment (Rust)

```rust
pub struct ServerProfile {
    // ... existing fields ...
    pub terminology_url: Option<String>,  // None = use global default
}
```

### Effective URL resolution (Rust helper)

```rust
pub fn effective_terminology_url(
    profile: &ServerProfile,
    settings: &GlobalSettings,
) -> Option<String> {
    profile.terminology_url.clone()
        .or_else(|| settings.terminology_url.clone())
}
```

Called by `terminology.rs` before making any `$lookup` request.

---

## Implementation Plan

### Backend (Rust)

- [x] Define `GlobalSettings` struct in `src-tauri/src/settings.rs`
- [x] Implement load/save to `~/.config/openehr-explorer/settings.json` (same pattern as `profiles.json`)
- [x] On first launch (file absent): write defaults (`terminology_server_url: "https://tx.fhir.ch/r4"`)
- [x] Expose `get_settings` and `save_settings` Tauri commands
- [x] Add `terminology_url: Option<String>` to `ServerProfile`; handle missing key gracefully in existing profile deserialisation (default `None`)
- [x] Add `effective_terminology_url()` helper, used by `terminology.rs`

### Frontend (Vue)

- [x] New route `/settings` → `src/views/Settings.vue`
- [x] Settings form with `terminologyServerUrl` text input and Save button
- [x] Add gear icon nav entry to `App.vue` left panel (bottom-pinned)
- [x] Update `ServerManager.vue` profile form: add optional `terminology_url` field with global-default placeholder
- [x] Settings loaded on app startup via `get_settings` invoke; stored in a Pinia `settingsStore`

---

## Acceptance Criteria

- [x] `/settings` route exists and is reachable from the left nav gear icon
- [x] Default terminology server URL field pre-populated with `https://tx.fhir.ch/r4` on fresh install
- [x] Saving settings persists to `settings.json`; survives app restart
- [x] Server profile form shows optional terminology URL field; empty = inherit global
- [x] Effective URL resolution: profile value wins if set; falls back to global; `terminology.rs` uses `effective_terminology_url()`
- [x] Adding `terminology_url` field to profiles is backward-compatible (existing profiles without the field deserialise correctly with `None`)
- [x] Settings page is extensible: adding a new setting requires only adding a field to `GlobalSettings` and a form row — no structural changes

---

## Future Settings (not in scope now)

To illustrate extensibility — future rows that would live naturally on this page:

| Setting | Default | Notes |
|---------|---------|-------|
| EHR list page size | 50 | Pagination default |
| Composition Pretty pane default | Pretty | or JSON / FLAT |
| AQL result row limit | 100 | Safety cap |
| HTTP request timeout (s) | 30 | For slow CDRs |
| Date/time display format | ISO 8601 | Locale preference |

---

## Related

- **PRD-0012**: Terminology Awareness — introduces `terminology_url`; depends on this PRD for the settings hierarchy
- `src-tauri/src/commands/server.rs` — existing profiles persistence pattern to follow
