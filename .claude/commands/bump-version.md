---
description: Bump the project version across all manifest files, lockfiles, and landing page
argument-hint: <new-version> (e.g. 0.5.0)
---

Bump the project version to `$1`.

Update the version string in all of the following files (these are the exact files touched by prior version bump commits such as `287322b` for 0.3.0 and `48ad2f9` for 0.4.0):

1. `package.json` — top-level `"version"` field
2. `package-lock.json` — top-level `"version"` and `packages[""].version`
3. `src-tauri/Cargo.toml` — `[package]` → `version`
4. `src-tauri/Cargo.lock` — the `[[package]]` entry for `name = "openehr-explorer"`
5. `src-tauri/tauri.conf.json` — top-level `"version"`
6. `docs/index.html` — the `version-badge` paragraph (e.g. `v0.4.0 &middot; macOS …`)
7. `docs/docs.html` — the Linux AppImage install snippet (`openehr-explorer_<version>_amd64.AppImage`, two occurrences)

Rules:
- Only replace the current version with `$1`. Do NOT touch historical version references in `docs/prd/` or `docs/adr/` (these are intentional historical records).
- Do not edit `CHANGELOG.md` or changelog docs — those are handled in a separate commit.
- After editing, run `git diff --stat` and confirm exactly 7 files changed, matching the stat of commit `287322b`.
- Commit the changes with this message:

  ```
  chore: bump version to $1

  Update manifest versions, lockfiles, install docs, and landing page
  version badge in preparation for the $1 release.
  ```

- Do NOT push or open a PR unless the user explicitly asks.
