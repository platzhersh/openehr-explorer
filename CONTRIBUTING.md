# Contributing to openEHR Explorer

Thanks for your interest in contributing! openEHR Explorer is a cross-platform desktop app for browsing, querying, and inspecting openEHR CDR instances (EHRBase, Better Platform, etc.), built with Tauri 2 (Rust) + Vue 3 + TypeScript.

This document covers how to get a working dev environment, the conventions the codebase follows, and what to expect when you open a PR.

## Prerequisites

- **Rust** (stable toolchain) — install via [rustup.rs](https://rustup.rs/)
- **Node.js** (v18+) — install via [nodejs.org](https://nodejs.org/)
- **Platform-specific Tauri dependencies** — see [Tauri Prerequisites](https://tauri.app/start/prerequisites/)

Verify your Rust install:

```bash
cargo --version
rustc --version
```

## Getting started

```bash
git clone https://github.com/platzhersh/openehr-explorer.git
cd openehr-explorer
npm install
npm run tauri dev       # runs the app with hot reload
```

If you need a local openEHR server to develop against, see [`LOCAL_TESTING.md`](./LOCAL_TESTING.md) for spinning up EHRBase via Docker Compose:

```bash
docker-compose up -d
```

## Making a change

1. Fork the repo and create a branch off `main` (`git checkout -b feat/my-change`).
2. Make your change, following the conventions below.
3. Run the checks locally before opening a PR (CI runs the same ones, but faster feedback is nicer for everyone):

   ```bash
   # Frontend
   npm run lint            # oxlint
   npm run fmt:check       # oxfmt
   npx vue-tsc --noEmit    # type check
   npm run build           # full frontend build

   # Rust backend
   cd src-tauri
   cargo fmt -- --check
   cargo clippy -- -D warnings
   cargo test
   ```

4. Open a PR against `main`. Reference any related issue, PRD, or ADR (see below) in the description.

## Code style

- **Frontend**: [oxlint](https://oxc.rs/docs/guide/usage/linter.html) + [oxfmt](https://oxc.rs/docs/guide/usage/formatter.html). Run `npm run lint:fix` and `npm run fmt` to auto-fix most issues.
- **Rust**: standard `rustfmt` + `clippy`, with `clippy -D warnings` enforced in CI — a clippy warning fails the build, same as an error.
- **Dependency versions are pinned**, not left on semver ranges:
  - `package.json`: exact versions, no `^`/`~` (e.g. `"vue": "3.5.31"`)
  - `Cargo.toml`: full versions without a caret; the real pin is `Cargo.lock`, which is committed
  - This keeps builds reproducible — please don't loosen a version range in a PR unless that's the point of the change.

## Documentation conventions

Significant changes are expected to come with documentation, using the same two artifact types the rest of the project uses (`docs/prd/` and `docs/adr/`):

- **PRD** (`PRD-XXXX-short-title.md`) — product requirements: what a feature is and why it exists.
- **ADR** (`ADR-XXXX-short-title.md`) — architecture decision record: a technical choice, its rationale, and its consequences.

Not every PR needs one of these — a bug fix or small UI tweak doesn't. A new feature, a new backend command, or a change to how server profiles/auth/data flow works usually does. See `docs/adr/ADR-0001-use-prd-and-adr-documentation.md` for the reasoning, and browse existing PRDs/ADRs for the expected format before writing a new one.

## Project structure, briefly

- `src-tauri/src/commands/` — Rust backend, one module per domain (`server.rs`, `ehr.rs`, `composition.rs`, `template.rs`, `query.rs`). Commands are registered in `src-tauri/src/lib.rs`.
- `src/stores/` — Pinia stores (one per domain), coordinating with Tauri commands.
- `src/views/` — top-level routed views (EHR browser, composition viewer, template browser, AQL runner, server manager).

See `CLAUDE.md` for a fuller architecture overview, including the frontend↔backend invoke pattern and key design patterns (server profile system, Web Template processing).

## Reporting bugs / requesting features

Please use [GitHub Issues](https://github.com/platzhersh/openehr-explorer/issues). For bugs, include your OS, app version, and steps to reproduce. For feature requests, a short description of the use case is more useful than a full spec — we can work out the design together.

## Releasing

Maintainers: see [RELEASING.md](RELEASING.md) for how the release pipeline works and how to cut a new version.

## License

By contributing, you agree that your contributions will be licensed under the project's [Apache License 2.0](./LICENSE).
