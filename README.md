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

## Tool Comparison

> **The Postman for openEHR** — but template-aware.

There is no dedicated developer GUI for openEHR. Developers currently reach for generic tools (Postman, curl) or heavyweight alternatives (openEHRTool v2 with Docker, Better Studio for Better Platform only). openEHR Explorer fills that gap.

| | **openEHR Explorer** | openEHRTool v2 | Postman / curl | Better Studio |
|---|:---:|:---:|:---:|:---:|
| **Type** | Desktop app (Tauri) | Web app (Docker) | Generic REST client | Web app (SaaS) |
| **Platforms** | macOS · Win · Linux | Any (Docker) | Any | Browser |
| **License** | MIT | Apache 2.0 | Freemium | Proprietary |

### ⚙️ Setup & Access

| Feature | **openEHR Explorer** | openEHRTool v2 | Postman / curl | Better Studio |
|---|:---:|:---:|:---:|:---:|
| No Docker / backend required | ✅ | ❌ Docker Compose | ✅ | ❌ Account required |
| Installs in < 2 min | ✅ dmg / exe / deb | ⚠️ `docker compose up` | ✅ | ❌ |
| Works with local EHRBase | ✅ | ✅ | ✅ | ❌ |
| Multi-server profiles (save & switch) | ✅ | ❌ | ⚠️ Collections | ❌ |
| Supports Better Platform | ⚠️ Phase 2 | ⚠️ Partial | ✅ | ✅ Native |
| CDR-agnostic (generic openEHR REST) | ✅ | ⚠️ Partial | ✅ | ❌ |

### 🏥 EHR & Composition Browsing

| Feature | **openEHR Explorer** | openEHRTool v2 | Postman / curl | Better Studio |
|---|:---:|:---:|:---:|:---:|
| Paginated EHR list | ✅ | ✅ | ❌ | ✅ |
| Compositions grouped by template | ✅ | ⚠️ | ❌ | ✅ |
| Raw canonical JSON view | ✅ | ✅ | ✅ | ✅ |
| **Template-aware "pretty" view** ¹ | ✅ ⭐ | ❌ | ❌ | ⚠️ |
| FLAT format view | ✅ | ❌ | ⚠️ Manual | ⚠️ |
| Composition version history | ✅ | ⚠️ | ❌ | ✅ |
| One-click copy (IDs, paths, values) | ✅ | ⚠️ | ❌ | ⚠️ |

> ¹ Resolves raw node IDs (`at0006`) to human-readable template labels (`"Systolic"`). No other tool does this.

### 📐 Template Tooling

| Feature | **openEHR Explorer** | openEHRTool v2 | Postman / curl | Better Studio |
|---|:---:|:---:|:---:|:---:|
| Template list & metadata | ✅ | ✅ | ❌ | ✅ |
| Web Template tree inspector | ✅ | ❌ | ❌ | ⚠️ |
| OPT drag-and-drop upload | ✅ | ✅ | ❌ | ✅ |
| **FLAT path copy panel** ² | ✅ ⭐ | ❌ | ❌ | ❌ |
| FLAT path validator | 🔵 Phase 2 | ❌ | ❌ | ❌ |

> ² One-click copy of every FLAT path from the Web Template tree — eliminates the #1 developer pain point when building compositions.

### 🔍 AQL Query Interface

| Feature | **openEHR Explorer** | openEHRTool v2 | Postman / curl | Better Studio |
|---|:---:|:---:|:---:|:---:|
| AQL editor with syntax highlighting | ✅ | ✅ | ⚠️ Raw body | ✅ |
| Tabular result view | ✅ | ✅ | ❌ Raw JSON | ✅ |
| Saved query library | ✅ | ⚠️ | ✅ Collections | ⚠️ |
| Export results to CSV | ✅ | ⚠️ | ❌ | ⚠️ |
| AQL path autocomplete | 🔵 Phase 2 | ❌ | ❌ | ⚠️ |

### 🛠️ Developer Experience

| Feature | **openEHR Explorer** | openEHRTool v2 | Postman / curl | Better Studio |
|---|:---:|:---:|:---:|:---:|
| Keyboard shortcuts (Cmd+K, Cmd+Enter) | ✅ | ❌ | ✅ | ⚠️ |
| Open-source | ✅ MIT | ✅ Apache 2.0 | ⚠️ Freemium | ❌ |
| Composition diff tool | 🔵 Phase 2 | ❌ | ❌ | ❌ |
| Synthetic data generation | 🔵 Phase 2 | ❌ | ❌ | ❌ |

### Summary

| | **openEHR Explorer** | openEHRTool v2 | Postman / curl | Better Studio |
|---|:---:|:---:|:---:|:---:|
| **Phase 1 score** (of 20 features) | **17 / 20** | 9 / 20 | 8 / 20 | 11 / 20 |

#### Unique to openEHR Explorer

- **Template-aware composition rendering** — `at0006` becomes `"Systolic"`. No other tool resolves node IDs to template labels.
- **FLAT path panel** — Every Web Template path, one click to copy. Eliminates the most common source of EHRBase integration errors.
- **Zero-dependency install** — No Docker, no Python, no Node runtime. A native binary that opens in < 2 seconds.
- **Multi-server profile switcher** — Local dev, staging, colleague's EHRBase — switch without restarting or editing config.

---

**Legend:** ✅ Full support · ⚠️ Partial / limited · ❌ Not supported · 🔵 Planned (Phase 2)

*openEHRTool v2: [github.com/crs4/openEHRTool-v2](https://github.com/crs4/openEHRTool-v2) · Better Studio: proprietary, Better Platform only · Scores: ✅ = 1, ⚠️ = 0.5, ❌ = 0*

## Documentation

- [PRD-0001: Desktop CDR Browser](docs/prd/PRD-0001-openehr-explorer-desktop-cdr-browser.md)
- [ADR-0001: PRD and ADR Documentation](docs/adr/ADR-0001-use-prd-and-adr-documentation.md)

## License

MIT
