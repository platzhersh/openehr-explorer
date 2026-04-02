# openEHR Explorer

A cross-platform desktop application for browsing, querying, and inspecting openEHR CDR instances (EHRBase, Better Platform, etc.).

Built with [Tauri](https://tauri.app) (Rust) + Vue 3 + TypeScript.

## Features

- **Server Connection Manager** — Save and switch between multiple CDR instances
- **EHR Browser** — Paginated list of EHRs with composition grouping by template
- **Composition Viewer** — Template-aware "pretty" view with human-readable labels, raw JSON, and FLAT format
- **FLAT Path Panel** — One-click copy of FLAT paths for SDK development
- **Template Browser** — Inspect Web Template trees, view OPT XML, drag-and-drop upload
- **AQL Query Runner** — Execute AQL queries with tabular results, saved queries, CSV export

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (v18+)
- Platform-specific dependencies for Tauri: see [Tauri Prerequisites](https://tauri.app/start/prerequisites/)

## Development

```bash
npm install
npm run tauri dev
```

## Build

```bash
npm run tauri build
```

## Documentation

- [PRD-0001: Desktop CDR Browser](docs/prd/PRD-0001-openehr-explorer-desktop-cdr-browser.md)
- [ADR-0001: PRD and ADR Documentation](docs/adr/ADR-0001-use-prd-and-adr-documentation.md)

## License

MIT
