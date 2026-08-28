---
description: Bump the project version across all manifest files, lockfiles, and landing page
argument-hint: <new-version> (e.g. 0.5.0)
---

Bump the project version to `$1`.

## Step 1: Version bump

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
- Do not touch the `#changelog` section of `docs/docs.html` here — that's Step 2, and lands in a separate commit.
- After editing, run `git diff --stat` and confirm exactly 7 files changed, matching the stat of commit `287322b`.
- Commit the changes with this message:

  ```
  chore: bump version to $1

  Update manifest versions, lockfiles, install docs, and landing page
  version badge in preparation for the $1 release.
  ```

- Do NOT push or open a PR unless the user explicitly asks.

## Step 2: Changelog entry

Add a new changelog entry for `$1` in `docs/docs.html`:

1. Find the previous tag: `git describe --tags --abbrev=0` (before the tag for `$1` is created).
2. Review what changed since that tag: `git log <previous-tag>..HEAD --oneline`. Read the underlying commits/PRs as needed to understand user-facing impact — don't just paraphrase commit subjects.
3. Add a new `<h3>v$1 — <short theme>` block as the **first** entry under `<section id="changelog">` in `docs/docs.html` (above the current top entry), matching the existing format: a short thematic title, then a `<ul>` of `<strong>Label:</strong> description` bullets covering user-facing highlights only (skip internal chores, CI tweaks, dependency bumps, etc. unless user-visible).
4. Commit separately with this message:

  ```
  docs: add v$1 changelog entry
  ```

- Do NOT push or open a PR unless the user explicitly asks.

## Step 3: What's New entry

Add a matching entry for `$1` to `src/lib/whats-new.ts` (the in-app "What's New" panel — see PRD-0018):

1. Reuse the same review of `git log <previous-tag>..HEAD` from Step 2 rather than re-deriving it.
2. Add a new object as the **first** entry in the `WHATS_NEW` array (newest first) with `version: "$1"`, today's date, and 3-5 `highlights`. This is a short, hand-curated highlight reel, not a full changelog — pick the handful of changes a user would actually notice, phrased for them (not commit-message paraphrases). Skip pure bugfixes/chores unless user-visible.
3. Where a highlight matches an existing feature tour, wire up its `tourId` and `routePath` (see `src/lib/tours.ts` for available tour IDs and `src/main.ts` for route paths) so the panel can offer to walk the user through it. Leave both fields off when no tour fits.
4. Commit separately with this message:

  ```
  feat(whats-new): add v$1 highlights
  ```

- Do NOT push or open a PR unless the user explicitly asks.
