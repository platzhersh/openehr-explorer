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

### Storybook
```bash
npm run storybook        # Run Storybook dev server (http://localhost:6006)
npm run build-storybook  # Build static Storybook (storybook-static/, gitignored)
```
Component stories live next to the component they document, e.g. `src/components/ToggleSwitch.vue` + `src/components/ToggleSwitch.stories.ts`. See ADR-0021 for scope and rationale.

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
The app uses **Tauri's invoke pattern** where Vue frontend calls Rust backend commands. All backend commands are registered in `src-tauri/src/lib.rs` via `tauri::generate_handler![]`.

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
- `src/stores/composition.ts` — Composition create/edit/delete state
- `src/stores/template.ts` — Web templates and OPT XML
- `src/stores/query.ts` — AQL query execution and saved queries
- `src/stores/settings.ts` — App-wide settings (persisted via the Rust backend)
- `src/stores/inspector.ts` — Request Inspector state (raw HTTP request/response log)

Stores coordinate with Tauri commands to fetch/persist data. Server profiles are persisted by the Rust backend in `~/.config/openehr-explorer/profiles.json` (Linux/macOS), with credentials stored via the OS keychain (see `src-tauri/src/credentials.rs` and ADR-0015).

### Rust Backend Structure
- `src-tauri/src/commands/` — Tauri command modules, one per domain:
  - `server.rs` — Server profile CRUD, connection testing, version detection
  - `ehr.rs` — EHR listing, detail fetching, CRUD, search
  - `composition.rs` — Composition retrieval (structured, FLAT, versions) and CRUD
  - `template.rs` — Template listing, Web Template fetch, OPT upload
  - `query.rs` — AQL execution, saved query management
  - `terminology.rs` — Terminology code lookups (cached via `terminology::TerminologyCache`)
- `src-tauri/src/credentials.rs` — Secure credential storage (OS keychain / encrypted-file fallback)
- `src-tauri/src/inspector.rs` — Request Inspector instrumentation (see ADR-0011)
- `src-tauri/src/settings.rs` — App settings persistence
- All commands use `reqwest` for HTTP calls to openEHR REST APIs
- Authentication (Basic/Bearer) is handled per-profile in `AuthMethod` enum

### Frontend Views
- `src/views/EhrBrowser.vue` — Paginated EHR list + composition grouping, EHR create/delete
- `src/views/CompositionViewer.vue` — Three-pane viewer (Pretty/JSON/FLAT) with path panel
- `src/views/CompositionForm.vue` — Composition create/edit form (medblocks-ui powered)
- `src/views/TemplateBrowser.vue` — Template list + Web Template tree inspector
- `src/views/AqlRunner.vue` — Query editor, saved queries, tabular results, CSV export
- `src/views/ServerManager.vue` — Server profile CRUD UI
- `src/views/Settings.vue` — Global settings page

### Routing
Vue Router defined in `src/main.ts`:
- `/ehrs` + `/ehrs/:ehrId` — EHR browser / detail
- `/ehrs/:ehrId/compositions/:compositionUid` — Composition viewer
- `/templates` + `/templates/:templateId` — Template browser
- `/compose/:templateId` + `/compose/:templateId/:ehrId` — Create composition from template
- `/edit/:ehrId/:compositionUid` — Edit existing composition
- `/aql` — AQL runner
- `/servers` — Server manager
- `/settings` — Global settings

## Key Design Patterns

### Server Profile System
Each server profile (`ServerProfile`) includes:
- `server_type`: `"ehrbase" | "better_platform" | "ferro_ehr" | "generic"` — determines API path conventions
- `auth_method`: `{ type: "none" | "basic" | "bearer", ... }` — discriminated union for auth

When adding new API integrations, use the `server_type` to branch URL construction logic (see `src-tauri/src/commands/ehr.rs` for examples).

### Web Template Processing
The app fetches "Web Templates" (openEHR operational template JSON representation) and renders them as an interactive tree in `src/components/CompositionTree.vue`. The `src/lib/webtemplate.ts` module contains utilities for traversing and extracting FLAT paths from Web Template nodes.

## Documentation Standards

This project uses **PRDs** (`docs/prd/`) and **ADRs** (`docs/adr/`) for governance:
- **PRD-XXXX-short-title.md**: Product requirements (what/why of features)
- **ADR-XXXX-short-title.md**: Architecture decisions (technical choices, rationale, consequences)

When making significant architectural changes, create an ADR. When planning new features, reference or create a PRD.

## Commit & PR Conventions

Follow [Conventional Commits](https://www.conventionalcommits.org/) for commit messages and PR titles: `<type>(<optional scope>): <description>`, e.g. `feat(website): publish working Windows install command`, `fix: gh api --jq doesn't accept extra jq flags`, `docs(readme): add header banner and badges`, `ci: add Windows package manager auto-publish workflows`, `chore: bump version to 0.4.2`. Common types in this repo: `feat`, `fix`, `docs`, `chore`, `ci`, `refactor`.

## CI/CD

GitHub Actions (`.github/workflows/ci.yml`) runs frontend and Rust checks plus a macOS build on every push/PR to `main`. Windows and Linux builds, GitHub Release publishing, the in-app updater manifest, and Homebrew/Scoop/WinGet package updates are all tag-triggered (`v*`).

See **[RELEASING.md](RELEASING.md)** for the full release process and what each CI job does.

## openEHR API Context

When working with openEHR REST APIs:
- **EHRBase** and **Better Platform** have slightly different endpoint conventions (handled via `server_type`)
- **FLAT format** is a denormalized key-value representation of compositions (useful for SDK development)
- **AQL** (Archetype Query Language) is used to query EHR data across compositions
- **OPT** (Operational Template XML) vs **Web Template** (JSON): both describe templates, different formats
