# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

openEHR Explorer is a cross-platform desktop application for browsing, querying, and inspecting openEHR CDR instances (EHRBase, Better Platform, etc.). Built with **Tauri 2** (Rust backend) + **Vue 3** + **TypeScript** (frontend).

## Prerequisites

Before running the app, ensure you have:
- **Rust** (stable toolchain) — Install via [rustup.rs](https://rustup.rs/)
- **Node.js** (v18+) — Install via [nodejs.org](https://nodejs.org/)
- **Platform-specific Tauri dependencies** — See [Tauri Prerequisites](https://tauri.app/start/prerequisites/)

**Verify Rust installation:**
```bash
cargo --version          # Should show cargo version
rustc --version          # Should show rustc version
```

If `cargo` is not found, install Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env  # or restart your shell
```

## Common Commands

### Development
```bash
npm install              # Install frontend dependencies
npm run tauri dev        # Run development server with hot reload

# Local EHRBase for testing
docker-compose up -d     # Start local EHRBase server (see LOCAL_TESTING.md)
docker-compose down      # Stop EHRBase server
```

### Dependency Management

**IMPORTANT:** This project uses **pinned versions** (not semver ranges) for all dependencies:
- `package.json`: Use exact versions without `^` or `~` (e.g., `"vue": "3.5.31"` NOT `"^3.5.31"`)
- `Cargo.toml`: Use full versions without caret (e.g., `version = "2.3.1"` NOT `version = "2"`)

**Note:** In Cargo.toml, we specify the full version number (e.g., `"2.3.1"`) which Cargo treats as `"^2.3.1"` by default. The actual pinning happens via `Cargo.lock` which is committed to the repository.

This ensures reproducible builds and prevents unexpected breakage from dependency updates.

### Linting & Formatting
```bash
npm run lint             # Lint frontend (oxlint)
npm run lint:fix         # Lint + auto-fix frontend
npm run fmt              # Format frontend (oxfmt)
npm run fmt:check        # Check frontend formatting

cd src-tauri
cargo fmt                # Format Rust code
cargo fmt -- --check     # Check Rust formatting
cargo clippy             # Run Rust linter
cargo clippy -- -D warnings  # Clippy with warnings as errors (CI mode)
```

### Build
```bash
npm run tauri build      # Build production app (creates DMG on macOS)
npm run build            # Build frontend only (runs vue-tsc + vite build)
```

### Type Checking
```bash
npx vue-tsc --noEmit     # Type check Vue/TypeScript without emitting files
```

### Rust Development
```bash
cd src-tauri
cargo build              # Build Rust backend
cargo test               # Run Rust tests
cargo clippy             # Run Rust linter
```

## Architecture

### Frontend-Backend Communication
The app uses **Tauri's invoke pattern** where Vue frontend calls Rust backend commands. All backend commands are registered in `src-tauri/src/lib.rs:9-31` via `tauri::generate_handler![]`.

**Frontend call pattern:**
```typescript
import { invoke } from "@tauri-apps/api/core";
const result = await invoke<ReturnType>("command_name", { param: value });
```

**Backend command pattern:**
```rust
#[tauri::command]
pub async fn command_name(param: Type) -> Result<ReturnType, String> {
    // Implementation
}
```

### State Management
Frontend uses **Pinia stores** (Vue 3 composition API style) for client-side state:
- `src/stores/server.ts` — Server profiles, active server, connection status
- `src/stores/ehr.ts` — EHR listing and detail state
- `src/stores/template.ts` — Web templates and OPT XML
- `src/stores/query.ts` — AQL query execution and saved queries

Stores coordinate with Tauri commands to fetch/persist data. Server profiles are persisted by the Rust backend in `~/.config/openehr-explorer/profiles.json` (Linux/macOS).

### Rust Backend Structure
- `src-tauri/src/commands/` — Contains all Tauri command modules:
  - `server.rs` — Server profile CRUD, connection testing
  - `ehr.rs` — EHR listing and detail fetching
  - `composition.rs` — Composition retrieval (structured, FLAT, versions)
  - `template.rs` — Template listing, Web Template fetch, OPT upload
  - `query.rs` — AQL execution, saved query management
- All commands use `reqwest` for HTTP calls to openEHR REST APIs
- Authentication (Basic/Bearer) is handled per-profile in `AuthMethod` enum

### Frontend Views
- `src/views/EhrBrowser.vue` — Paginated EHR list + composition grouping
- `src/views/CompositionViewer.vue` — Three-pane viewer (Pretty/JSON/FLAT) with path panel
- `src/views/TemplateBrowser.vue` — Template list + Web Template tree inspector
- `src/views/AqlRunner.vue` — Query editor, saved queries, tabular results, CSV export
- `src/views/ServerManager.vue` — Server profile CRUD UI

### Routing
Vue Router defined in `src/main.ts:6-52`:
- `/ehrs` — EHR browser
- `/ehrs/:ehrId/compositions/:compositionUid` — Composition viewer
- `/templates` + `/templates/:templateId` — Template browser
- `/aql` — AQL runner
- `/servers` — Server manager

## Key Design Patterns

### Server Profile System
Each server profile (`ServerProfile`) includes:
- `server_type`: `"ehrbase" | "better_platform" | "generic"` — determines API path conventions
- `auth_method`: `{ type: "none" | "basic" | "bearer", ... }` — discriminated union for auth

When adding new API integrations, use the `server_type` to branch URL construction logic (see `src-tauri/src/commands/ehr.rs` for examples).

### Web Template Processing
The app fetches "Web Templates" (openEHR operational template JSON representation) and renders them as an interactive tree in `src/components/CompositionTree.vue`. The `src/lib/webtemplate.ts` module contains utilities for traversing and extracting FLAT paths from Web Template nodes.

## Documentation Standards

This project uses **PRDs** (`docs/prd/`) and **ADRs** (`docs/adr/`) for governance:
- **PRD-XXXX-short-title.md**: Product requirements (what/why of features)
- **ADR-XXXX-short-title.md**: Architecture decisions (technical choices, rationale, consequences)

When making significant architectural changes, create an ADR. When planning new features, reference or create a PRD.

## CI/CD

GitHub Actions workflow (`.github/workflows/ci.yml`):
- **frontend** job: Runs `npm run build` (type checks + Vite build)
- **build-macos** job: Builds Tauri app, uploads DMG artifact, publishes to GitHub Releases on tags

To trigger a release, push a git tag starting with `v` (e.g., `v0.2.0`).

## openEHR API Context

When working with openEHR REST APIs:
- **EHRBase** and **Better Platform** have slightly different endpoint conventions (handled via `server_type`)
- **FLAT format** is a denormalized key-value representation of compositions (useful for SDK development)
- **AQL** (Archetype Query Language) is used to query EHR data across compositions
- **OPT** (Operational Template XML) vs **Web Template** (JSON): both describe templates, different formats

## PRD-0003: Composition & EHR CRUD Implementation

### Status: ✅ COMPLETE (100%)

All features from PRD-0003 have been implemented and are ready to use!

### Implemented Features

**Backend (100% Complete):**
- EHR CRUD: `create_ehr`, `update_ehr_status`, `delete_ehr` (`src-tauri/src/commands/ehr.rs:228-426`)
- Composition CRUD: `create_composition`, `update_composition`, `delete_composition` (`src-tauri/src/commands/composition.rs:184-299`)
- All commands registered and working with FLAT format (`application/openehr.wt.flat.schema+json`)

**Stores (100% Complete):**
- `src/stores/ehr.ts` - EHR CRUD functions with proper error handling
- `src/stores/composition.ts` - Composition CRUD functions

**UI Components (100% Complete):**
- `src/components/EhrCreateDialog.vue` - Full EHR creation dialog with subject identity, flags, custom ID
- `src/views/EhrBrowser.vue` - "+ New EHR" button + Delete EHR dialog (requires typing EHR ID to confirm)
- `src/views/CompositionForm.vue` - Complete composition create/edit form with:
  - EHR selector with "Create New EHR" option
  - Context fields (composer, language, territory, time)
  - medblocks-ui `<mb-form>` integration for template rendering
  - FLAT preview panel (toggleable)
  - Edit mode with pre-population from existing compositions
  - Draft persistence (auto-save every 30s, localStorage with 24hr expiry)
  - Request/Response detail panels
  - Full error handling and success messages
- `src/views/CompositionViewer.vue` - Enhanced with Edit and Delete buttons
- `src/views/TemplateBrowser.vue` - "New Composition" button for each template

**Routes Added:**
- `/compose/:templateId` - Create composition from template
- `/compose/:templateId/:ehrId` - Create composition for specific EHR
- `/edit/:ehrId/:compositionUid` - Edit existing composition

**Dependencies:**
- `medblocks-ui` - Loaded via CDN in `index.html` (JSDelivr CDN: 0.1.1)
- `@tauri-apps/plugin-dialog` (2.6.0)
- `@tauri-apps/plugin-fs` (2.4.5)

### Complete Workflows

1. **Create EHR**: Click "+ New EHR" in EHR Browser → Fill form → Auto-navigates to new EHR
2. **Delete EHR**: Open EHR detail → Click "Delete EHR" → Type EHR ID to confirm → Deleted
3. **Create Composition**: Template Browser → "New Composition" → Select EHR → Fill form → Submit → View created composition
4. **Edit Composition**: View composition → Click "Edit" → Modify form → Submit → New version created
5. **Delete Composition**: View composition → Click "Delete" → Confirm → Returns to EHR detail

### Documentation

See `IMPLEMENTATION_SUMMARY.md` for:
- Detailed breakdown of all implemented features
- Testing instructions for local EHRBase
- Architecture decisions and patterns
- Complete file reference
