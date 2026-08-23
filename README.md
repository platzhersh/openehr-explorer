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

## Screenshots

<p align="center">
  <img src="docs/assets/demo.gif" alt="Screen recording of openEHR Explorer: browsing EHRs, opening a composition, and switching between the Pretty, JSON, and FLAT views" width="800">
</p>

<table>
  <tr>
    <td><img src="docs/assets/screenshots/01-ehr-browser.webp" alt="EHR Browser with a list of EHRs and a selected EHR's compositions"></td>
    <td><img src="docs/assets/screenshots/04-templates.webp" alt="Template Browser with an interactive OPT tree"></td>
  </tr>
  <tr>
    <td><img src="docs/assets/screenshots/02-composition-pretty.webp" alt="Composition Viewer's template-aware Pretty tab"></td>
    <td><img src="docs/assets/screenshots/06-aql-runner.webp" alt="AQL Runner with a query and tabular results"></td>
  </tr>
</table>

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

There is no dedicated developer GUI for openEHR. Developers currently reach for generic tools (Postman, curl) or heavyweight alternatives (openEHRTool v2 with Docker, Better Studio for Better Platform only, ehr-ctrl). openEHR Explorer fills that gap.

See the full, always-up-to-date **[Tool Comparison](https://platzhersh.github.io/openehr-explorer/compare.html)** on the docs site for a feature-by-feature breakdown against openEHRTool v2, Postman/curl, Better Studio, and ehr-ctrl.

## Documentation

- [User docs & guides](https://platzhersh.github.io/openehr-explorer/docs.html)
- [Tool comparison](https://platzhersh.github.io/openehr-explorer/compare.html)
- [PRDs](docs/prd/) — product requirements for each feature
- [ADRs](docs/adr/) — architecture decisions and rationale
- [RELEASING.md](RELEASING.md) — release process

## License

Apache 2.0 — see [LICENSE](LICENSE) for details.
