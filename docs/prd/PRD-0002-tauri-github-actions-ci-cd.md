# PRD-0002: GitHub Actions CI/CD — Tauri Build & Release

**Version:** 1.0
**Date:** 2026-04-02
**Status:** Draft
**Owner:** openEHR Explorer
**Repo slug:** `openehr-explorer`
**Depends on:** PRD-0001 (openEHR Explorer Desktop CDR Browser)

---

## Executive Summary

Set up GitHub Actions workflows to automatically build, sign, and publish macOS DMG installers for openEHR Explorer. The pipeline covers CI (lint, test, build verification on every push/PR) and CD (produce signed release artifacts when a version tag is pushed). Windows EXE support is scoped as a follow-up phase but the workflow structure is designed to accommodate it with minimal changes.

---

## Problem Statement

**Current State:**

- There is no CI/CD pipeline. Builds are manual (`npm run tauri build` on a developer machine).
- No automated verification that PRs compile successfully on target platforms.
- No reproducible release process — artifacts depend on whoever runs the build.
- No code signing, which means macOS Gatekeeper will block the app on first launch for end users.

**Pain Points:**

- Contributors cannot verify their changes build correctly without a local Tauri/Rust toolchain.
- Releasing a new version requires manual steps: build locally, create a GitHub Release, upload the DMG.
- Unsigned DMGs trigger macOS Gatekeeper warnings ("app is damaged" or "unidentified developer"), which is a dealbreaker for adoption.
- No path to Windows EXE distribution without repeating manual effort on a Windows machine.

---

## Goals & Success Metrics

### Goals

- Every push to `main` and every PR runs a CI check that verifies the app compiles on macOS.
- Tagged releases (`v*`) produce a signed macOS DMG and attach it to a GitHub Release automatically.
- The workflow structure supports adding Windows EXE builds in a future phase with minimal changes.
- Keep CI fast: the build-check job should complete in under 15 minutes for a warm cache.

### Success Metrics

- 100% of PRs merged to `main` have a passing CI build check.
- A tagged release produces a downloadable DMG on GitHub Releases within 20 minutes of the tag push.
- The DMG installs and launches without Gatekeeper warnings on macOS 13+ (when signing is configured).
- Adding Windows EXE support requires changes to only one workflow file (adding a matrix entry + secrets).

---

## Feature Requirements

### Phase 1: macOS DMG (MVP)

#### 1. CI Workflow — Build Verification

**Priority:** P0 (Must Have)

**Trigger:** Push to `main`, pull requests targeting `main`.

**Jobs:**

1. **Frontend checks** (runs on `ubuntu-latest` for speed):
   - `npm ci` — install dependencies
   - `npm run build` — verify TypeScript compilation and Vite build
   - (Future: `npm run lint`, `npm run test` when lint/test scripts exist)

2. **Tauri build check** (runs on `macos-latest`):
   - Install Rust stable toolchain (via `dtolnay/rust-toolchain`)
   - Cache Cargo registry and target directory (via `actions/cache` or `Swatinem/rust-cache`)
   - Cache npm dependencies (`actions/cache` on `~/.npm`)
   - `npm ci`
   - `npm run tauri build` — full release build without signing
   - Upload the unsigned DMG as a workflow artifact (for manual testing of PR builds)

**Acceptance Criteria:**
- PR status checks block merge if the Tauri build fails.
- Workflow completes in under 15 minutes with warm caches.
- The unsigned DMG artifact is downloadable from the Actions run summary.

#### 2. Release Workflow — Tagged Builds with GitHub Release

**Priority:** P0 (Must Have)

**Trigger:** Push of a tag matching `v*` (e.g., `v0.1.0`, `v0.2.0-beta.1`).

**Jobs:**

1. **Build macOS DMG** (runs on `macos-latest`):
   - Install Rust stable toolchain
   - Restore caches (Cargo, npm)
   - `npm ci`
   - `npm run tauri build`
   - The Tauri build produces a `.dmg` file in `src-tauri/target/release/bundle/dmg/`
   - Sign and notarize the DMG (see §3 below) — if signing secrets are not configured, produce an unsigned build with a warning annotation

2. **Create GitHub Release**:
   - Use `softprops/action-gh-release` (or equivalent)
   - Attach the DMG to the release
   - Auto-generate release notes from commits since the last tag
   - Mark as pre-release if the tag contains `-beta`, `-alpha`, or `-rc`

**Acceptance Criteria:**
- Pushing `v0.1.0` tag creates a GitHub Release with the DMG attached.
- Release notes list commits since the previous tag.
- Pre-release tags are correctly flagged.

#### 3. macOS Code Signing & Notarization

**Priority:** P1 (Should Have)

**Approach:** Use Tauri's built-in signing support via environment variables.

**Required GitHub Secrets:**

| Secret | Purpose |
|--------|---------|
| `APPLE_CERTIFICATE` | Base64-encoded `.p12` Developer ID Application certificate |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12` file |
| `APPLE_SIGNING_IDENTITY` | Certificate identity string (e.g., `Developer ID Application: Name (TEAMID)`) |
| `APPLE_API_ISSUER` | App Store Connect API issuer ID (for notarization) |
| `APPLE_API_KEY` | App Store Connect API key ID |
| `APPLE_API_KEY_PATH` | Base64-encoded `.p8` private key file contents |

**Behavior:**
- If signing secrets are present, the release workflow signs and notarizes the DMG.
- If secrets are absent, the workflow succeeds but produces an unsigned DMG and adds a warning annotation to the workflow run.
- This allows contributors to fork the repo and still get working (unsigned) builds.

**Acceptance Criteria:**
- Signed DMG passes `spctl --assess --type open --context context:primary-signature` verification.
- Notarization succeeds (verified via `xcrun stapler validate`).
- Unsigned builds still complete successfully when secrets are not configured.

#### 4. Caching Strategy

**Priority:** P0 (Must Have)

**Caches:**

| Cache | Key | Path |
|-------|-----|------|
| Cargo registry | `cargo-registry-{hashFiles('**/Cargo.lock')}` | `~/.cargo/registry`, `~/.cargo/git` |
| Cargo build | `cargo-build-{runner.os}-{hashFiles('**/Cargo.lock')}` | `src-tauri/target` |
| npm | `npm-{hashFiles('**/package-lock.json')}` | `~/.npm` |

**Target:** Warm-cache CI build completes in under 10 minutes. Cold-cache build completes in under 20 minutes.

---

### Phase 2: Windows EXE (Future)

#### 5. Windows Build Matrix Entry

**Priority:** P2 (Future)

**Approach:** Extend both CI and release workflows with a matrix strategy:

```yaml
strategy:
  matrix:
    include:
      - os: macos-latest
        target: dmg
      - os: windows-latest
        target: nsis
```

**Windows-specific considerations:**
- Tauri produces an NSIS installer (`.exe`) and/or MSI on Windows.
- Windows code signing requires an EV certificate or Azure Trusted Signing.
- The NSIS installer is the standard Tauri output — no additional configuration needed beyond adding the matrix entry.
- Windows Defender SmartScreen warnings are reduced (but not eliminated) by signing.

**Required secrets for Windows signing (future):**

| Secret | Purpose |
|--------|---------|
| `WINDOWS_CERTIFICATE` | Base64-encoded `.pfx` code signing certificate |
| `WINDOWS_CERTIFICATE_PASSWORD` | Password for the `.pfx` file |

**Acceptance Criteria:**
- Tagged releases produce both a macOS DMG and a Windows EXE installer.
- Windows installer runs without errors on Windows 10+.

#### 6. Linux AppImage (Future)

**Priority:** P3 (Nice to Have)

Add `ubuntu-22.04` to the build matrix. Tauri produces AppImage and `.deb` bundles on Linux. No code signing required. Low effort to add once the matrix pattern is established.

---

## Technical Design

### Workflow Files

```
.github/
└── workflows/
    ├── ci.yml          # Build verification on push/PR
    └── release.yml     # Tagged release builds + GitHub Release
```

### ci.yml — Structure

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  frontend:
    runs-on: ubuntu-latest
    steps:
      - Checkout
      - Setup Node 20
      - npm ci
      - npm run build

  build-macos:
    runs-on: macos-latest
    steps:
      - Checkout
      - Setup Node 20
      - Setup Rust stable
      - Restore Cargo + npm caches
      - npm ci
      - npm run tauri build
      - Upload DMG artifact
```

### release.yml — Structure

```yaml
name: Release

on:
  push:
    tags: ['v*']

jobs:
  build-macos:
    runs-on: macos-latest
    environment: release
    steps:
      - Checkout
      - Setup Node 20
      - Setup Rust stable
      - Restore caches
      - npm ci
      - Configure signing (conditional on secrets)
      - npm run tauri build
      - Upload DMG artifact

  publish:
    needs: build-macos
    runs-on: ubuntu-latest
    steps:
      - Download all artifacts
      - Create GitHub Release with artifacts
      - Generate release notes
```

### Tauri Build Output Paths

| Platform | Format | Path |
|----------|--------|------|
| macOS | DMG | `src-tauri/target/release/bundle/dmg/openEHR Explorer_<version>_aarch64.dmg` |
| macOS | .app | `src-tauri/target/release/bundle/macos/openEHR Explorer.app` |
| Windows (future) | NSIS | `src-tauri/target/release/bundle/nsis/openEHR Explorer_<version>_x64-setup.exe` |
| Linux (future) | AppImage | `src-tauri/target/release/bundle/appimage/openEHR Explorer_<version>_amd64.AppImage` |

### macOS Architecture Targets

- **Phase 1:** Build on `macos-latest` (Apple Silicon / aarch64). This produces an ARM64 DMG.
- **Phase 1 stretch:** Add a universal binary target (`--target universal-apple-darwin`) to support both Intel and Apple Silicon Macs from a single DMG. This requires installing both `aarch64-apple-darwin` and `x86_64-apple-darwin` Rust targets.
- Recommendation: Start with `aarch64` only (simpler, faster CI). Add universal binary if users request Intel support.

---

## Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| macOS code signing certificate cost ($99/yr Apple Developer Program) | Low | Certain | Required for distribution. Budget for it. Unsigned builds work for development. |
| CI build times are slow (Rust compilation) | Medium | High | Aggressive caching of `target/` directory. `Swatinem/rust-cache` typically reduces Rust build from ~15min to ~3min. |
| GitHub-hosted macOS runners are slower and more expensive (10x minute multiplier) | Medium | Certain | Use caching to minimize build time. Consider self-hosted runner if costs become significant. |
| Tauri build fails on CI but works locally | Medium | Medium | Pin Rust toolchain version in workflow. Use same Node version as local dev. |
| Apple notarization fails or is slow | Low | Medium | Notarization typically completes in 1–5 minutes. Retry with backoff. Use App Store Connect API (not `altool`) for reliability. |

---

## Implementation Plan

### Milestone 1: CI Workflow (1–2 days)

- [ ] Create `.github/workflows/ci.yml`
- [ ] Frontend check job (ubuntu, npm build)
- [ ] macOS Tauri build job with caching
- [ ] Upload unsigned DMG as workflow artifact
- [ ] Verify PR checks work correctly

**Deliverable:** Every PR gets a green/red build status and a downloadable DMG artifact.

### Milestone 2: Release Workflow (1–2 days)

- [ ] Create `.github/workflows/release.yml`
- [ ] Build DMG on tag push
- [ ] Create GitHub Release with DMG attached
- [ ] Auto-generate release notes
- [ ] Pre-release detection for beta/alpha/rc tags

**Deliverable:** Pushing `v0.1.0` tag produces a GitHub Release with a downloadable DMG.

### Milestone 3: Code Signing (1 day, requires Apple Developer account)

- [ ] Configure GitHub repository secrets for Apple signing
- [ ] Add signing environment variables to release workflow
- [ ] Add notarization step
- [ ] Verify signed DMG passes Gatekeeper
- [ ] Graceful fallback when secrets are absent

**Deliverable:** Release DMGs are signed and notarized. No Gatekeeper warnings on install.

### Milestone 4: Windows EXE (Future)

- [ ] Add `windows-latest` to build matrix in both workflows
- [ ] Configure Windows code signing secrets
- [ ] Test NSIS installer on Windows 10+
- [ ] Attach EXE to GitHub Releases alongside DMG

**Deliverable:** Tagged releases produce both macOS DMG and Windows EXE.

---

## Open Questions

1. **Apple Developer account:** Is there an existing Apple Developer account for signing, or does one need to be created? The $99/yr cost is required for notarized distribution.

2. **Universal binary vs ARM-only:** Should the initial DMG target Apple Silicon only (faster builds, simpler) or produce a universal binary (broader compatibility)? Recommendation: ARM-only for v0.1.0, add universal binary based on user demand.

3. **Auto-update:** Tauri supports built-in auto-update via `tauri-plugin-updater`. Should the release workflow also publish an update manifest (`latest.json`) for in-app updates? Recommendation: Defer to a separate PRD — auto-update adds complexity (update server, signature keys) that is orthogonal to CI/CD.

4. **Branch protection:** Should `main` branch require passing CI checks before merge? Recommendation: Yes — enable branch protection rules once the CI workflow is verified working.

---

## Related

- PRD-0001: openEHR Explorer Desktop CDR Browser (the application being built and released)
- Tauri GitHub Actions guide: https://v2.tauri.app/distribute/pipelines/
- `tauri-apps/tauri-action`: Official Tauri GitHub Action for building and releasing
- Apple Developer Program: https://developer.apple.com/programs/
- `softprops/action-gh-release`: GitHub Action for creating releases
- `Swatinem/rust-cache`: Rust-specific caching for GitHub Actions
