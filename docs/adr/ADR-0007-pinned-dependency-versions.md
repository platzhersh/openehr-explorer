# ADR-0007: Pinned Dependency Versions (No Semver Ranges)

**Date:** 2026-04-03

## Status

Accepted

---

## Context

The openEHR Explorer uses dependencies from two package ecosystems:
- **npm (Node.js)** - Frontend dependencies (Vue, Vite, Tauri npm packages)
- **Cargo (Rust)** - Backend dependencies (Tauri crates, reqwest, serde)

Both ecosystems use semantic versioning (semver) and support version ranges:

**npm conventions:**
- `^1.2.3` - Compatible with 1.2.3, allows updates to 1.x.x (not 2.0.0)
- `~1.2.3` - Compatible with 1.2.3, allows updates to 1.2.x (not 1.3.0)
- `1.2.3` - Exact version only
- `*` or `latest` - Any version (highly discouraged)

**Cargo conventions:**
- `"1.2.3"` - Compatible with 1.2.3, allows updates to 1.x.x (caret by default)
- `"=1.2.3"` - Exact version only
- `">= 1.2.3"` - Any version >= 1.2.3

### The Problem with Version Ranges

During implementation of PRD-0003, we encountered multiple dependency issues:

1. **medblocks-ui version mismatch:**
   - `package.json` specified `"medblocks-ui": "^1.0.0"`
   - `npm install` failed: "No matching version found for medblocks-ui@^1.0.0"
   - Actual latest version was `0.1.1`, not `1.0.0`
   - The `^1.0.0` range was incorrect and prevented installation

2. **Tauri plugin version divergence:**
   - npm package versions (e.g., `@tauri-apps/plugin-dialog@2.6.0`)
   - Rust crate versions (e.g., `tauri-plugin-dialog 2.6.0`)
   - **These version numbers are not synchronized!**
   - npm `@tauri-apps/plugin-dialog@2.0.0` might correspond to Rust crate `2.3.0`
   - Using version ranges allowed silent mismatches

3. **Cargo exact version syntax error:**
   - Used `"=2.3.4"` in `Cargo.toml` (thinking it meant exact version)
   - Cargo build failed: `failed to select a version for the requirement tauri-plugin-dialog = "=2.3.4"`
   - Rust doesn't support `"="` prefix syntax; use `"2.3.4"` for caret or explicitly `">= 2.3.4, < 3.0.0"` for ranges

4. **Breaking changes in minor updates:**
   - Dependency maintainers sometimes introduce breaking changes in minor/patch versions
   - `^2.3.0` could pull in `2.4.0` which breaks the build
   - CI/CD builds become non-reproducible (today's build ≠ tomorrow's build)

### Current Practice in Open Source

**Most open-source projects use ranges:**
- Convenient (automatic security updates)
- Reduces maintenance burden
- Assumes semver compliance

**Some projects pin versions:**
- Docker images (reproducible builds)
- Mobile apps (strict control)
- Critical infrastructure (no surprises)

---

## Decision

We will use **pinned dependency versions** (exact versions without semver ranges) in both `package.json` and `Cargo.toml`.

## npm (package.json)

**Rule:** Use exact versions without `^`, `~`, or any range operator.

**Before:**
```json
{
  "dependencies": {
    "vue": "^3.5.31",
    "vue-router": "^4.6.4",
    "@tauri-apps/api": "^2.10.1"
  }
}
```

**After:**
```json
{
  "dependencies": {
    "vue": "3.5.31",
    "vue-router": "4.6.4",
    "@tauri-apps/api": "2.10.1"
  }
}
```

**Verification:**
```bash
# Check for any unpinned versions
grep -E '[\^~\*]|latest' package.json
# Should return nothing
```

## Cargo (Cargo.toml)

**Rule:** Use plain version strings (Cargo's default caret behavior is acceptable for local dev, but we specify exact versions for clarity).

**Before:**
```toml
[dependencies]
tauri = "2"
tauri-plugin-dialog = "=2.3.4"  # Syntax error!
reqwest = { version = "0.12", features = ["json"] }
```

**After:**
```toml
[dependencies]
tauri = { version = "2.3.1", features = [] }
tauri-plugin-dialog = "2.6.0"
reqwest = { version = "0.12.4", features = ["json"] }
```

**Notes:**
- Do not use `"="` prefix (invalid Rust syntax)
- Specify minor and patch versions, not just major (`"2.3.1"`, not `"2"`)
- For feature flags, use object syntax: `{ version = "X.Y.Z", features = [...] }`

## Dependency Update Process

**Manual updates only:**

1. Check for outdated dependencies:
   ```bash
   npm outdated              # npm
   cargo outdated            # Cargo (requires cargo-outdated: cargo install cargo-outdated)
   ```

2. Review changelogs for breaking changes

3. Update `package.json` / `Cargo.toml` with new exact versions

4. Test thoroughly

5. Commit with message: `"chore(deps): update <package> from X.Y.Z to A.B.C"`

**No automatic updates:**
- No Dependabot auto-merge
- No `npm update` without explicit version changes
- No CI/CD that auto-updates dependencies

---

## Consequences

### Positive

- **Reproducible builds:** Same source code + dependencies = identical build every time
- **Predictability:** No surprise breakages from transitive dependency updates
- **Explicit control:** Every version change is intentional and reviewed
- **Easier debugging:** When a bug appears, the dependency versions haven't changed
- **CI/CD reliability:** CI builds don't fail randomly due to new dependency versions
- **Clear audit trail:** Git history shows exactly when and why each dependency was updated
- **Tauri version alignment:** Explicit versions prevent npm/Cargo version mismatches

### Negative

- **Manual maintenance:** Must manually update dependencies (not automatic)
- **Security updates delayed:** Won't automatically receive security patches (mitigated by regular manual checks)
- **Outdated dependencies:** Risk of falling behind if updates are neglected (mitigated by scheduled dependency reviews)
- **Initial setup time:** Must research actual available versions (e.g., `npm view medblocks-ui versions` to find `0.1.1` not `1.0.0`)

### Mitigation Strategies

**Security updates:**
- Run `npm audit` and `cargo audit` in CI/CD
- Set up GitHub Dependabot alerts (manual review, no auto-merge)
- Schedule monthly dependency review

**Staying current:**
- Review dependencies quarterly (Q1, Q2, Q3, Q4)
- Prioritize security patches and critical bug fixes
- Batch non-critical updates to reduce churn

**Documentation:**
- Document why specific versions are pinned (e.g., "2.6.0 fixes bug #123")
- Keep CHANGELOG.md updated with dependency changes

---

## Special Cases

### Peer Dependencies

Some npm packages have peer dependencies with ranges. We can't control these directly, but we:

1. Pin the direct dependency exactly
2. Let npm resolve peers (usually works)
3. Document any peer dependency issues in CLAUDE.md

**Example:**
```json
{
  "dependencies": {
    "vue": "3.5.31"  // Pinned
  },
  "peerDependencies": {
    // Managed by package author, not us
  }
}
```

### Dev Dependencies

**Also pinned.** Dev dependencies affect build reproducibility:

```json
{
  "devDependencies": {
    "vite": "6.4.1",           // Pinned
    "typescript": "5.6.3",     // Pinned
    "@vitejs/plugin-vue": "5.2.4"  // Pinned
  }
}
```

### Transitive Dependencies (lockfiles)

**npm:** `package-lock.json` already pins transitive dependencies. Our rule ensures top-level dependencies are also pinned in `package.json`.

**Cargo:** `Cargo.lock` pins transitive dependencies. We pin direct dependencies in `Cargo.toml` for clarity.

**Both lockfiles must be committed to git** for reproducibility.

---

## Migration Path

### Existing Projects

If the project already uses version ranges:

1. Run `npm list` and `cargo tree` to see resolved versions
2. Update `package.json` and `Cargo.toml` to use those exact versions
3. Commit: `"chore(deps): pin all dependency versions"`
4. Document in PR: "Switching to pinned versions per ADR-0007"

### Example Migration

**Before:**
```json
"dependencies": {
  "vue": "^3.5.0",
  "@tauri-apps/api": "^2.0.0"
}
```

**After (using currently installed versions):**
```bash
npm list --depth=0
# vue@3.5.31
# @tauri-apps/api@2.10.1

# Update package.json manually to:
"dependencies": {
  "vue": "3.5.31",
  "@tauri-apps/api": "2.10.1"
}
```

---

## Examples from This Project

### npm Dependencies (package.json)

```json
{
  "dependencies": {
    "vue": "3.5.31",
    "vue-router": "4.6.4",
    "pinia": "3.0.4",
    "@tauri-apps/api": "2.10.1",
    "@tauri-apps/plugin-opener": "2.5.3",
    "@tauri-apps/plugin-dialog": "2.6.0",
    "@tauri-apps/plugin-fs": "2.4.5"
  },
  "devDependencies": {
    "@vitejs/plugin-vue": "5.2.4",
    "typescript": "5.6.3",
    "vite": "6.4.1",
    "vue-tsc": "2.2.12",
    "@tauri-apps/cli": "2.10.1"
  }
}
```

### Cargo Dependencies (src-tauri/Cargo.toml)

```toml
[dependencies]
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
tauri = { version = "2.3.1", features = [] }
tauri-plugin-opener = "2.6.0"
tauri-plugin-http = "2.4.5"
tauri-plugin-dialog = "2.6.0"
tauri-plugin-fs = "2.6.0"
reqwest = { version = "0.12.18", features = ["json"] }
base64 = "0.22.1"
```

**Note:** Tauri npm package versions differ from crate versions!
- npm `@tauri-apps/api@2.10.1`
- Rust `tauri 2.3.1`

Both are pinned to avoid any confusion.

---

## Alternatives Considered

### A. Use Semver Ranges with Lockfiles

**Rejected.** Lockfiles (`package-lock.json`, `Cargo.lock`) do pin transitive dependencies, but:
- Top-level `package.json` ranges still allow `npm install` to pull newer versions
- Fresh clones or CI cache misses could resolve different versions
- Doesn't prevent version mismatch issues (medblocks-ui `^1.0.0` would still fail)

### B. Use Ranges for Patch Versions Only (`~1.2.3`)

**Rejected.** Still allows automatic updates (1.2.4, 1.2.5, etc.), which can introduce bugs. The cost of manually updating patches is low compared to debugging a broken build.

### C. Pin Production Dependencies, Range Dev Dependencies

**Rejected.** Dev dependencies affect the build (Vite, TypeScript compiler). A new Vite version could break the build or change output. All dependencies must be pinned for reproducibility.

### D. Use Dependabot with Auto-Merge

**Rejected.** Auto-merge removes human review. We want every dependency update to be intentional and tested.

---

## Related

- ADR-0008: CDN-Based Web Components for medblocks-ui (context: why we couldn't use npm package)
- PRD-0003: Composition & EHR CRUD (exposed dependency version issues)

---

## References

- npm semver documentation: https://docs.npmjs.com/cli/v6/using-npm/semver
- Cargo version specification: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html
- Reproducible builds: https://reproducible-builds.org/
- Docker best practices (pin versions): https://docs.docker.com/develop/dev-best-practices/

---

## Review Schedule

**Quarterly dependency review:**
- Q1 (January): Check for security updates
- Q2 (April): Update major dependencies if needed
- Q3 (July): Security updates
- Q4 (October): Plan major updates for next year

Between reviews, only update dependencies for:
- Critical security vulnerabilities (CVE alerts)
- Blocking bugs that prevent development
- New features that require specific versions
