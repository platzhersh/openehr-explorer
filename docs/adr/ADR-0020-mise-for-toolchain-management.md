# ADR-0020: mise for Toolchain Management

**Date:** 2026-08-07
**Status:** Accepted
**Repo:** `openehr-explorer`

---

## Context

The project needs two toolchains — Rust (stable) and Node — but until now neither was declared anywhere machine-readable. `README.md` said "Rust (stable)" and "Node.js (v18+)" in prose, while `.github/workflows/ci.yml` hardcoded `node-version: 24` in five separate jobs and `dtolnay/rust-toolchain@stable` in four. Three consequences followed:

1. **Contributors could not reproduce CI locally.** A developer on Node 18 passed local checks that CI (Node 24) might fail, and nothing in the repo told them which version CI used.
2. **Version bumps meant editing five job definitions.** Nothing enforced that the five stayed equal; a partial bump would silently produce jobs on mixed versions.
3. **It contradicted the project's own dependency policy.** `CLAUDE.md` mandates pinned versions for every npm and Cargo dependency, with `package-lock.json` and `Cargo.lock` committed — but the runtimes *executing* those pinned dependencies were unpinned.

[mise](https://mise.jdx.dev) is a polyglot version manager (the successor to asdf's model) that reads a declarative `mise.toml`, installs the listed tools, and activates them per-directory. It is the closest fit for a repo that needs two unrelated language toolchains from one file.

---

## Decision

We will declare the toolchain in `mise.toml` at the repo root and treat it as the single source of truth, consumed by both developers and CI.

```toml
[tools]
node = "24"
rust = { version = "stable", components = ["rustfmt", "clippy"] }
```

CI installs these via `jdx/mise-action@v4` in every job, replacing `actions/setup-node` and `dtolnay/rust-toolchain`. A Node or Rust bump is now a one-line change to `mise.toml` that propagates to all five jobs and to every developer's shell.

**mise is optional for developers.** The plain `npm install && npm run tauri dev` flow, backed by a system rustup and Node, remains fully supported and documented first in `README.md`. mise is an accelerant for contributors who want CI parity, not a prerequisite for contributing. Making it mandatory would add an install step to a project whose stated selling point is a zero-dependency install.

`mise.toml` also defines tasks (`setup`, `dev`, `build`, `lint`, `fmt`, `typecheck`, `test`, `ehrbase:up`/`ehrbase:down`) that wrap the existing npm scripts and cargo commands. These are thin conveniences — notably `lint` and `fmt` run the frontend and backend halves together, which no single npm script does. They do not replace the npm scripts, which remain the canonical entry points.

### Rejected alternatives

- **`.nvmrc` + `rust-toolchain.toml`.** Two files, two tools, and `actions/setup-node` reads `.nvmrc` but nothing reads both. Workable, but does not unify the two toolchains and offers no task runner.
- **Devcontainer / Docker-based dev environment.** Tauri development needs native windowing and a GPU-backed WebView; running the app from a container is awkward on every host OS. Docker stays scoped to the local EHRBase test server (`docker-compose.yml`).
- **Pinning Rust to an exact version (e.g. `1.90.0`).** Would match the dependency-pinning policy more literally, but CI has always tracked `stable` and Tauri's MSRV moves with it. Pinning would trade a class of "CI broke on a new stable" failures for a class of "we forgot to bump Rust for eight months" ones. Revisit if a stable release ever breaks the build.

---

## Consequences

**Positive:**

- Node and Rust versions live in one file, versioned with the code and identical for CI and developers.
- New contributors reach a working environment with `mise install && mise run setup`.
- The gap between "passes locally" and "passes CI" narrows to platform differences alone.

**Negative / accepted trade-offs:**

- **A new CI dependency.** `jdx/mise-action` now sits on the critical path of the release pipeline. Mitigated by pinning to the `v4` tag, consistent with how every other action in the workflow is referenced.
- **Slightly slower cold CI runs.** `actions/setup-node` pulls a prebuilt Node from a GitHub-hosted mirror; mise downloads from upstream. mise-action's built-in cache absorbs this on warm runs.
- **npm caching became explicit.** `actions/setup-node`'s `cache: npm` disappeared with the action. Each job that runs `npm ci` now carries an `actions/cache@v4` step keyed on `package-lock.json`, listing both the Unix (`~/.npm`) and Windows (`~/AppData/Local/npm-cache`) cache locations. This is more workflow YAML than before — the cost of decoupling toolchain installation from dependency caching.
- **macOS universal-binary targets moved to an explicit step.** `dtolnay/rust-toolchain`'s `targets:` input is replaced by `rustup target add aarch64-apple-darwin x86_64-apple-darwin`. The targets are deliberately *not* in `mise.toml`, since mise applies them on every platform and Linux/Windows contributors have no use for Darwin targets.
- **mise does not cover Tauri's system dependencies.** WebKit, GTK and platform build tools are still installed by hand (or by apt in CI). `mise install` gets a contributor most of the way to a working environment, not all the way — `README.md` states this explicitly.

---

## Implementation

- `mise.toml` — tool declarations and task definitions.
- `.github/workflows/ci.yml` — `jdx/mise-action@v4` in all five jobs; npm cache steps; explicit `rustup target add` in `build-macos`.
- `README.md` — optional "With mise" subsection under Prerequisites.
- `CLAUDE.md` — notes that toolchain versions are changed in `mise.toml`, not in the workflow.
- `package.json` — `engines.node` set to `>=24`, and the prose prerequisites in `README.md`/`CLAUDE.md`
  raised from the stale "v18+" to v24, so every statement of the minimum agrees with what CI runs.
  `engines` is advisory (npm warns, `.npmrc` does not set `engine-strict`) — the enforcement that
  matters is CI itself.
