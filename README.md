<p align="center">
  <img src="docs/assets/og-image.png" alt="openEHR Explorer — Browse, query, and inspect any openEHR CDR, no curl required." width="600">
</p>

<p align="center">
  <a href="https://github.com/platzhersh/openehr-explorer/releases/latest"><img src="https://img.shields.io/github/v/release/platzhersh/openehr-explorer?color=blue" alt="Latest release"></a>
  <a href="https://github.com/platzhersh/openehr-explorer/releases/latest"><img src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-blue" alt="Platforms"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/platzhersh/openehr-explorer" alt="License"></a>
  <a href="https://github.com/platzhersh/openehr-explorer/actions/workflows/ci.yml"><img src="https://github.com/platzhersh/openehr-explorer/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://platzhersh.github.io/openehr-explorer/"><img src="https://img.shields.io/badge/docs-GitHub%20Pages-blue" alt="Documentation"></a>
</p>

A cross-platform desktop application for browsing, querying, and inspecting openEHR CDR instances (EHRBase, Better Platform, etc.).

Built with [Tauri](https://tauri.app) (Rust) + Vue 3 + TypeScript.

## Features

- **Server Connection Manager** — Save and switch between multiple CDR instances with auto version detection
- **EHR Browser** — Paginated list with server-side AQL search (subject, namespace, system, modifiable, hasCompositions)
- **Composition Viewer** — Template-aware "pretty" view with human-readable labels, raw JSON, and FLAT format
- **FLAT Path Panel** — One-click copy of FLAT paths for SDK development
- **Template Browser** — Inspect Web Template trees, view OPT XML, drag-and-drop upload
- **AQL Query Runner** — Execute AQL queries with 3-layer autocomplete, tabular results, saved queries, CSV export
- **Request Inspector** — View all HTTP traffic with request/response details, copy as curl
- **Keyboard Shortcuts** — Navigate with Cmd+1-4, toggle inspector with Cmd+Shift+I, execute queries with Cmd+Enter

## Installation

Download the latest build for macOS, Windows, or Linux from [GitHub Releases](https://github.com/platzhersh/openehr-explorer/releases/latest).

**macOS** — also installable via Homebrew:

```bash
brew install --cask platzhersh/openehr-explorer/openehr-explorer
```

**Windows** — also installable via Scoop:

```bash
scoop bucket add openehr-explorer https://github.com/platzhersh/scoop-openehr-explorer
scoop install openehr-explorer
```

**Linux** — `.deb` (Debian/Ubuntu) or portable `.AppImage` (any distro):

```bash
# .deb
sudo apt install ./openehr-explorer_<version>_amd64.deb

# or .AppImage
chmod +x openehr-explorer_<version>_amd64.AppImage && ./openehr-explorer_<version>_amd64.AppImage
```

See the [installation docs](https://platzhersh.github.io/openehr-explorer/docs.html#installation) for troubleshooting (unsigned builds, Gatekeeper/SmartScreen warnings).

## Prerequisites

Only needed if you're building from source:

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
| **License** | Apache 2.0 | Apache 2.0 | Freemium | Proprietary |

### ⚙️ Setup & Access

| Feature | **openEHR Explorer** | openEHRTool v2 | Postman / curl | Better Studio |
|---|:---:|:---:|:---:|:---:|
| No Docker / backend required | ✅ | ❌ Docker Compose | ✅ | ❌ Account required |
| Installs in < 2 min | ✅ dmg / exe / deb | ⚠️ `docker compose up` | ✅ | ❌ |
| Works with local EHRBase | ✅ | ✅ | ✅ | ❌ |
| Multi-server profiles with version detection | ✅ Auto detect | ❌ | ⚠️ Collections | ❌ |
| Supports Better Platform | ⚠️ Phase 2 | ⚠️ Partial | ✅ | ✅ Native |
| CDR-agnostic (generic openEHR REST) | ✅ | ⚠️ Partial | ✅ | ❌ |

### 🏥 EHR & Composition Browsing

| Feature | **openEHR Explorer** | openEHRTool v2 | Postman / curl | Better Studio |
|---|:---:|:---:|:---:|:---:|
| Paginated EHR list | ✅ | ✅ | ❌ | ✅ |
| Server-side EHR search (subject, namespace, system, etc.) | ✅ AQL-backed | ⚠️ | ❌ | ✅ |
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

> ³ Three layers: (1) AQL keywords & RM types, (2) Static RM paths for EHR/COMPOSITION, (3) Template-specific archetype paths extracted from Web Templates.

### 🔍 AQL Query Interface

| Feature | **openEHR Explorer** | openEHRTool v2 | Postman / curl | Better Studio |
|---|:---:|:---:|:---:|:---:|
| AQL editor with syntax highlighting | ✅ | ✅ | ⚠️ Raw body | ✅ |
| Tabular result view | ✅ | ✅ | ❌ Raw JSON | ✅ |
| Saved query library | ✅ | ⚠️ | ✅ Collections | ⚠️ |
| Export results to CSV | ✅ | ⚠️ | ❌ | ⚠️ |
| **AQL path autocomplete (3-layer CodeMirror)** ³ | ✅ ⭐ | ❌ | ❌ | ⚠️ |

### 🛠️ Developer Experience

| Feature | **openEHR Explorer** | openEHRTool v2 | Postman / curl | Better Studio |
|---|:---:|:---:|:---:|:---:|
| Request Inspector (view all HTTP traffic) | ✅ Bottom drawer | ❌ | ✅ Native | ❌ |
| Keyboard shortcuts (navigation + actions) | ✅ Cmd+1-4, Cmd+Shift+I | ❌ | ✅ | ⚠️ |
| Open-source | ✅ Apache 2.0 | ✅ Apache 2.0 | ⚠️ Freemium | ❌ |
| Composition diff tool | 🔵 Phase 2 | ❌ | ❌ | ❌ |
| Synthetic data generation | 🔵 Phase 2 | ❌ | ❌ | ❌ |

### Summary

| | **openEHR Explorer** | openEHRTool v2 | Postman / curl | Better Studio |
|---|:---:|:---:|:---:|:---:|
| **Phase 1 score** (of 24 features) | **21 / 24** | 9.5 / 24 | 9.5 / 24 | 12 / 24 |

#### Unique to openEHR Explorer

- **Template-aware composition rendering** — `at0006` becomes `"Systolic"`. No other tool resolves node IDs to template labels.
- **3-layer AQL autocomplete** — Keywords, RM paths, and template-aware path suggestions powered by CodeMirror.
- **FLAT path panel** — Every Web Template path, one click to copy. Eliminates the most common source of EHRBase integration errors.
- **Request Inspector** — See every HTTP request/response with tree/raw/FLAT views, copy as curl.
- **Zero-dependency install** — No Docker, no Python, no Node runtime. A native binary that opens in < 2 seconds.
- **Multi-server profile switcher** — Local dev, staging, colleague's EHRBase — switch without restarting or editing config.

---

**Legend:** ✅ Full support · ⚠️ Partial / limited · ❌ Not supported · 🔵 Planned (Phase 2)

*openEHRTool v2: [github.com/crs4/openEHRTool-v2](https://github.com/crs4/openEHRTool-v2) · Better Studio: proprietary, Better Platform only · Scores: ✅ = 1, ⚠️ = 0.5, ❌ = 0*

## Documentation

- [User docs & guides](https://platzhersh.github.io/openehr-explorer/docs.html)
- [PRDs](docs/prd/) — product requirements for each feature
- [ADRs](docs/adr/) — architecture decisions and rationale
- [RELEASING.md](RELEASING.md) — release process

## License

Apache 2.0 — see [LICENSE](LICENSE) for details.
