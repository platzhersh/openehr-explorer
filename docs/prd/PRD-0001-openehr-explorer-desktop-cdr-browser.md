# PRD-0001: openEHR Explorer — Desktop CDR Browser

**Version:** 1.1
**Date:** 2026-04-02
**Status:** Draft
**Owner:** openEHR Explorer (standalone — openEHR ecosystem)
**Repo slug:** `openehr-explorer`

---

## Executive Summary

Build a cross-platform desktop application that lets developers browse, query, and inspect openEHR CDR instances (EHRBase, Better Platform, etc.) without writing curl commands or crafting raw JSON. Inspired by Vanya Labs' FHIR client, the openEHR Explorer fills a genuine gap in the ecosystem: a polished, template-aware developer tool that makes the openEHR data model tangible and navigable.

The app connects to local or remote CDR instances, browses EHRs and compositions in a hierarchical view, renders compositions with template-aware labels (not raw `at0006` node IDs), and executes AQL queries with a structured result view.

**Ecosystem position:** openEHR Explorer is a standalone tool — an independent sibling to oehrpy (Python SDK) and Open CIS (reference app). It shares no runtime dependency on either. All three form a coherent developer tooling ecosystem around openEHR without any one requiring the others to be installed.

**Prior art:** A UCL research group attempted a similar Electron-based tool in 2018–2019 (`github.com/ucl-openehr-explorer`) but the project was abandoned after a proof-of-concept stage (vanilla HTML/JS, no framework, 0 stars, last commit April 2019). The name and GitHub org are effectively dormant. openEHR Explorer picks up the same idea with a modern stack and genuine commitment to maintenance.

---

## Problem Statement

**Current State:**

- The only comparable open-source tool is openEHRTool v2 (CRS4) — a FastAPI/Redis/Vue web app requiring Docker Compose setup, focused on admin operations rather than developer exploration.
- Developers routinely use Postman or curl to interact with EHRBase during development, with no domain-specific formatting.
- Composition JSON is verbose and semantically opaque — `at0006` means nothing without the OPT context.
- No tool renders a composition using its template's human-readable labels.
- Onboarding new developers to openEHR requires significant manual effort to understand the data hierarchy.

**Pain Points:**

- Copy-pasting EHR IDs and composition UIDs between Postman calls is tedious.
- Debugging FLAT format path errors requires cross-referencing the Web Template JSON manually.
- No visual way to drill into versioned compositions or understand their archetype structure.
- Running AQL ad-hoc during development requires knowing the exact syntax with no autocomplete or result formatting.
- Switching between EHRBase instances (local dev, staging, colleague's server) has no tooling support.

**User Personas:**

1. **openEHR Developer** — Building a clinical app on top of EHRBase. Needs to verify compositions were stored correctly, debug FLAT paths, test AQL queries, and inspect template structures.
2. **Clinical Informaticist** — Validates that clinical models (archetypes/templates) are correctly capturing the intended data. Wants to see compositions rendered semantically, not as raw JSON.
3. **openEHR Learner** — Getting started with openEHR. Needs a visual tool to understand the EHR → Composition → Archetype hierarchy concretely.
4. **Integration Engineer** — Building a FHIR↔openEHR bridge or data pipeline. Needs to verify stored compositions match expectations and quickly retrieve FLAT paths for mapping configs.

---

## Goals & Success Metrics

### Goals

- Reduce time to inspect a composition from ~5 minutes (curl → copy UID → fetch → parse JSON) to under 10 seconds.
- Make the openEHR data hierarchy (EHR → Versioned Composition → Composition → Archetype → Element) visually navigable.
- Render compositions with template-aware labels so `at0006` becomes "Any Event" and `at0004` becomes "Systolic".
- Provide an AQL query interface with result formatting.
- Support multiple server profiles (local EHRBase, Better Platform, remote staging instances).
- Work cross-platform out of the box (macOS, Windows, Linux).

### Success Metrics

- A developer can connect to a local EHRBase instance and browse all EHRs in < 30 seconds from first launch.
- Compositions are rendered with human-readable archetype labels (not raw node IDs) whenever a matching template is available.
- AQL queries return results in < 3 seconds for typical development datasets (< 10,000 compositions).
- The tool runs without Docker, external services, or a backend process.
- Positive feedback from at least 3 openEHR Discourse community members within 4 weeks of release.

---

## Feature Requirements

### Phase 1: Core Explorer (MVP)

#### 1. Server Connection Manager

**Priority:** P0 (Must Have)

**User Stories:**
- As a developer, I want to add a named server profile (EHRBase, Better) with URL and credentials so I can connect to different environments quickly.
- As a developer, I want to switch between server profiles without restarting the app.
- As a developer, I want the app to remember my server profiles across sessions.

**Functional Requirements:**

- **Server Profile Form**
  - Fields: Name (free text), Base URL, Server Type (EHRBase / Better Platform / Generic openEHR REST), Auth Method (None / Basic Auth / OAuth2 Bearer Token)
  - Test Connection button — fires a `GET /rest/openehr/v1/definition/template/adl1.4` and reports HTTP status
  - Profiles persisted locally (encrypted credentials)

- **Server Switcher**
  - Dropdown/sidebar showing all saved profiles with connection status indicator (green/red)
  - Active server displayed in app header at all times
  - One-click switching, no restart required

- **Supported Backends**
  - EHRBase (primary target, v2.x)
  - Better Platform (secondary target)
  - Any server implementing openEHR REST API 1.0.2

**Technical Notes:**
- Local config stored in OS-appropriate app data dir (`~/.config/openehr-explorer/` or platform equivalent)
- Passwords stored in OS keychain (Keytar or equivalent)

---

#### 2. EHR Browser

**Priority:** P0 (Must Have)

**User Stories:**
- As a developer, I want to see a paginated list of all EHRs on the connected server so I can find the one I'm working with.
- As a developer, I want to click on an EHR and see all its compositions grouped by template.
- As a developer, I want to copy an EHR ID to clipboard in one click.

**Functional Requirements:**

- **EHR List View**
  - Table: EHR ID (truncated with copy button), Subject ID (if present in EHR status), Time Created, Composition Count
  - Pagination (20 per page, configurable)
  - Search by EHR ID or subject external ref
  - Sort by created date (desc default)
  - Refresh button

- **EHR Detail Panel**
  - EHR ID, Time Created, System ID, EHR Status (is_modifiable, is_queryable)
  - Composition list grouped by template ID, showing most recent first
  - Composition count badge per template group
  - Click to open a composition in the Composition Viewer

**Technical Notes:**
- Uses `GET /rest/openehr/v1/ehr` (list endpoint) with pagination params
- EHR Status fetched via `GET /rest/openehr/v1/ehr/{ehr_id}/ehr_status`

---

#### 3. Composition Viewer

**Priority:** P0 (Must Have)

**User Stories:**
- As a developer, I want to see a composition rendered with human-readable labels (not raw node IDs) when a matching template is loaded.
- As a developer, I want to toggle between the template-aware "pretty" view and raw canonical JSON.
- As a developer, I want to see version history for a composition and diff between versions.
- As a developer, I want to copy the FLAT path for any element with one click, for use in my app code.

**Functional Requirements:**

- **Pretty View (template-aware)**
  - Hierarchical tree: Composition → Section → Observation/Evaluation → Data Elements
  - Node labels sourced from the loaded Web Template (human-readable name, not `at0006`)
  - Data values displayed with units and formatting (e.g. `120 mm[Hg]`, `2026-01-03 10:30`)
  - Archetype ID shown as a subtle badge on each node
  - When no template is available, falls back to RM type labels (`OBSERVATION`, `ELEMENT`, etc.)

- **Raw JSON View**
  - Syntax-highlighted canonical JSON (both `application/json` and `application/openehr.wt.flat+json` toggle)
  - Copy full JSON button
  - Collapsible JSON nodes

- **FLAT Path Panel**
  - Side panel listing all FLAT paths in the composition
  - Each path has a one-click copy button
  - Paths are highlighted on hover in the Pretty View tree
  - Useful for building composition builders (e.g. in oehrpy or any other SDK) or debugging path mismatches

- **Version History**
  - Accordion showing all versions (VERSIONED_COMPOSITION) with timestamps and commit audit info
  - Side-by-side diff view between any two versions (canonical JSON diff)

**Technical Notes:**
- Pretty View built by fetching the Web Template (`GET /rest/openehr/v1/definition/template/adl1.4/{template_id}`) and resolving node labels against `aql_path` / `id` fields
- FLAT format requested via `Accept: application/openehr.wt.flat+json` header
- Versions fetched via `GET /rest/openehr/v1/ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version`

---

#### 4. Template Browser

**Priority:** P0 (Must Have)

**User Stories:**
- As a developer, I want to see all uploaded templates on the server with their metadata.
- As a developer, I want to inspect a template's Web Template JSON to understand FLAT paths.
- As a developer, I want to upload a new OPT file by drag-and-drop.

**Functional Requirements:**

- **Template List View**
  - Table: Template ID, Version, Created Date, Archetype Count (from Web Template)
  - Search/filter by template ID
  - Sort by name or date

- **Template Detail View**
  - Tab 1: Web Template tree — visual hierarchy of the template's nodes with `aqlPath`, `rmType`, and allowed value constraints shown per node
  - Tab 2: Raw Web Template JSON (syntax-highlighted, collapsible)
  - Tab 3: Raw OPT XML (syntax-highlighted)
  - Download buttons for both Web Template and OPT

- **Template Upload**
  - Drag-and-drop OPT file (XML) upload zone
  - XML validation before upload
  - Success/error feedback

---

#### 5. AQL Query Runner

**Priority:** P1 (Should Have)

**User Stories:**
- As a developer, I want to type an AQL query and run it against the connected server.
- As a developer, I want query results displayed in a structured table, not raw JSON.
- As a developer, I want to save and reuse frequently used queries.

**Functional Requirements:**

- **Query Editor**
  - Multi-line text editor with monospace font
  - AQL syntax highlighting (basic — keywords `SELECT`, `FROM`, `WHERE`, `ORDER BY`, `LIMIT`)
  - Run button (Cmd/Ctrl+Enter shortcut)
  - Error display with EHRBase error message

- **Results View**
  - Auto-generated table from result columns
  - Column headers from AQL aliases
  - JSON cell viewer for nested values (click to expand)
  - Export to CSV button
  - Row count and execution time displayed

- **Saved Queries**
  - Name + save current query
  - Sidebar list of saved queries
  - Persisted locally per server profile

**Technical Notes:**
- Uses `POST /rest/openehr/v1/query/aql` with `{q: "..."}` body
- Results follow the openEHR REST AQL Response format (`resultSet` array)

---

### Phase 2: Enhanced Developer Experience

**Priority:** P2 (Nice to Have) — Post-MVP

#### 6. FLAT Path Validator

Integrate oehrpy's `FlatValidator` directly in the app: paste a FLAT JSON payload, select a template, and see validation errors highlighted inline. This makes the desktop app a natural companion to oehrpy development.

#### 7. Composition Diff Tool

Standalone tool (not tied to version history): paste two compositions and get a semantic diff — not just JSON diff, but label-aware diff showing which clinical data elements changed.

#### 8. AQL Autocomplete

Autocomplete for `FROM COMPOSITION c` paths using the loaded templates' `aqlPath` values. Makes writing AQL significantly faster.

#### 9. Synthetic Data Generator

Integration point for MapEHR/synthetic data tooling: generate test compositions from a template with randomized data, submit to connected server. Useful for populating dev EHRBase instances.

---

## Technical Architecture

### Technology Choice: Tauri

**Recommended stack:** Tauri (Rust backend) + Vue 3 + TypeScript frontend.

**Rationale:**
- Cross-platform (macOS, Windows, Linux) from single codebase
- No Node.js runtime required for end users — ships as a native binary
- Smaller bundle size than Electron (~10MB vs ~150MB)
- Rust backend handles OS keychain integration (Keytar equivalent), file system access, HTTP requests to CDR
- Vue 3 frontend reuses Open CIS component patterns (shadcn-vue, Vite)
- Tauri's HTTP client in Rust handles CORS-free CDR requests (avoids browser CORS restrictions)

**Alternative considered:** Electron — rejected due to bundle size and resource usage. The target audience (developers) will appreciate the minimal footprint philosophy.

**Alternative considered:** Web app (browser-based) — rejected because local EHRBase instances typically run on `localhost:8080` without CORS headers configured for arbitrary origins, and storing server credentials in a browser context is less secure.

### Data Flow

```
User Action (Vue UI)
  → Tauri IPC Command
    → Rust backend
      → openEHR REST API (local or remote CDR)
        → Response deserialized
          → Sent to Vue via IPC
            → Rendered in component
```

### Key Modules

```
src/
├── commands/           # Tauri IPC commands (Rust)
│   ├── ehr.rs          # EHR list/detail
│   ├── composition.rs  # Composition fetch/versions
│   ├── template.rs     # Template list/upload/fetch
│   ├── query.rs        # AQL execution
│   └── server.rs       # Server profile CRUD + test connection
├── store/              # Local persistence (SQLite via Tauri)
│   ├── profiles.rs     # Server profiles
│   └── queries.rs      # Saved AQL queries
└── ui/ (Vue 3)
    ├── views/
    │   ├── EhrBrowser.vue
    │   ├── CompositionViewer.vue
    │   ├── TemplateBrowser.vue
    │   └── AqlRunner.vue
    ├── components/
    │   ├── CompositionTree.vue      # Template-aware tree renderer
    │   ├── FlatPathPanel.vue        # FLAT path copy helper
    │   └── ServerSwitcher.vue
    └── lib/
        ├── webtemplate.ts           # Web Template node resolution
        └── flatpath.ts              # FLAT path extraction from canonical JSON
```

### Local Storage

- Server profiles: SQLite (via `tauri-plugin-sql`)
- Saved queries: SQLite
- Cached Web Templates: SQLite (keyed by `template_id + server_url`, TTL 1h)
- Credentials: OS keychain via `tauri-plugin-keychain`

---

## UX Design Principles

**Three-panel layout** (inspired by Vanya and database GUI tools like TablePlus):

```
┌──────────────┬────────────────────┬─────────────────────────┐
│  Left Panel  │   Center Panel     │    Right Panel          │
│              │                    │                         │
│ Server       │  EHR List /        │  Composition Viewer /   │
│ Switcher     │  Template List /   │  Template Detail /      │
│              │  AQL Results       │  AQL Result Detail      │
│ Navigation   │                    │                         │
│ (EHRs,       │                    │                         │
│  Templates,  │                    │                         │
│  AQL)        │                    │                         │
└──────────────┴────────────────────┴─────────────────────────┘
```

- **Keyboard-first:** Cmd+K command palette for quick navigation, Cmd+Enter to run AQL
- **Copy everywhere:** Every ID, path, and value has a one-click copy button
- **Progressive disclosure:** Raw JSON always one click away from any pretty view
- **Minimal footprint:** App launches in < 2 seconds, no splash screen

---

## Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Tauri learning curve (Rust) | Medium | High | Start with minimal Rust — use `tauri::http` crate for HTTP, keep business logic in Vue/TS |
| Web Template node resolution is complex for deep archetypes | High | Medium | Start with label resolution only (no full constraint rendering); progressively enhance |
| Better Platform API differences from EHRBase | Medium | Medium | Abstract CDR client behind a trait; implement EHRBase adapter first |
| FLAT path extraction from canonical JSON is non-trivial | Medium | High | Scope Phase 1 to display FLAT paths from the Web Template definition, not by parsing the composition |
| No openEHR ecosystem funding → low adoption | Low | Medium | Release on openEHR Discourse + GitHub; position as standalone community tool alongside oehrpy and Open CIS |

---

## Implementation Plan

### Milestone 1: Skeleton + Server Connection (Week 1–2)

- [ ] Tauri + Vue 3 project scaffold
- [ ] Server profile CRUD (add, edit, delete, persist to SQLite)
- [ ] Test Connection flow (HTTP ping, display status)
- [ ] Server Switcher UI component

**Deliverable:** App launches, user can add a local EHRBase server and see a green connection status.

### Milestone 2: EHR Browser (Week 3–4)

- [ ] EHR list (paginated, sortable)
- [ ] EHR detail (compositions grouped by template)
- [ ] Copy EHR ID to clipboard
- [ ] Basic composition fetch (raw canonical JSON view)

**Deliverable:** User can browse all EHRs on a local EHRBase and open any composition as raw JSON.

### Milestone 3: Template Browser + Pretty View (Week 5–7)

- [ ] Template list view
- [ ] Web Template fetch and local cache
- [ ] Template detail: Web Template tree renderer
- [ ] Composition Pretty View: resolve node labels from cached Web Template
- [ ] FLAT Path Panel

**Deliverable:** Compositions render with human-readable labels. Developer can copy FLAT paths.

### Milestone 4: AQL Runner + Polish (Week 8–9)

- [ ] AQL editor with syntax highlighting
- [ ] AQL execution + table result view
- [ ] Saved queries (persist to SQLite)
- [ ] Export to CSV
- [ ] App icon, about screen, release build pipeline (GitHub Actions)

**Deliverable:** Public release v0.1.0. Announced on openEHR Discourse.

---

## Open Questions

1. **Better Platform auth:** Better uses OAuth2 with specific realm configuration. Is it worth targeting Better in v0.1.0, or EHRBase-only first? Recommendation: EHRBase-only for v0.1.0 — validate the concept before adding auth complexity.

2. **oehrpy integration (Phase 2):** The FLAT Path Validator in Phase 2 could optionally call `oehrpy` as a Python subprocess for validation logic, but this would require Python to be installed separately. Alternative: reimplement the relevant validation in TypeScript/Rust so the tool remains self-contained. The tool has no oehrpy runtime dependency in Phase 1 and any Phase 2 integration should be strictly opt-in.

3. **Distribution:** GitHub Releases only, or also Homebrew tap / winget / Flathub? A Homebrew tap is low-effort and high-visibility for macOS developers and worth adding at v0.1.0.

4. **Relation to Open CIS Admin Interface (PRD-0001):** There is feature overlap with the Open CIS admin panel (template upload, EHR/composition browsing). openEHR Explorer is a standalone tool aimed at any openEHR CDR; PRD-0001 is embedded in the CIS web app for Open CIS users specifically. They share design patterns but remain independent.

---

## Success Criteria

**v0.1.0 is successful if:**
- ✅ Installs with one command / dmg / exe on all three platforms
- ✅ Connects to a local EHRBase in < 30 seconds from first launch
- ✅ Compositions render with template-aware labels when a matching template exists
- ✅ AQL queries execute and return tabular results
- ✅ At least 5 GitHub stars within 2 weeks of Discourse announcement
- ✅ No crash on startup across macOS 13+, Windows 10+, Ubuntu 22.04+

---

## Related

- PRD-0001: Open CIS Admin Interface (internal web-based EHRBase management — overlapping scope, different context)
- PRD-0000: oehrpy Python SDK (ecosystem sibling — no runtime dependency, optional Phase 2 integration)
- openEHRTool v2: https://github.com/crs4/openEHRTool-v2 (closest existing tool — web app, admin-focused, Docker required)
- UCL openEHR Explorer: https://github.com/ucl-openehr-explorer (prior art — abandoned Electron PoC, 2018–2019)
- Vanya Labs: https://vanyalabs.com (FHIR equivalent — inspiration for UX approach)
- Tauri: https://tauri.app
- openEHR REST API spec: https://specifications.openehr.org/releases/ITS-REST/latest/overview.html
