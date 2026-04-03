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
