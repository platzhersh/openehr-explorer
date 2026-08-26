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

Add a new changelog entry for `$1` in **both** `docs/docs.html` and the
root `CHANGELOG.md` — they must always describe the same release in the
same words; drifting apart defeats the point of having both.

1. Find the previous tag: `git describe --tags --abbrev=0` (before the tag for `$1` is created).
2. Review what changed since that tag: `git log <previous-tag>..HEAD --oneline`. Read the underlying commits/PRs as needed to understand user-facing impact — don't just paraphrase commit subjects.
3. Add a new `<h3>v$1 — <short theme>` block as the **first** entry under `<section id="changelog">` in `docs/docs.html` (above the current top entry), matching the existing format: a short thematic title, then a `<ul>` of `<strong>Label:</strong> description` bullets covering user-facing highlights only (skip internal chores, CI tweaks, dependency bumps, etc. unless user-visible).
4. Mirror the same theme and bullets as a new `## [$1] - <YYYY-MM-DD>` section at the **top** of `CHANGELOG.md` (below the intro, above the previous top entry) — same bullet text, just HTML→Markdown (`<strong>Label:</strong>` → `**Label:**`, `<code>` → backticks, entities like `&amp;`/`&ldquo;` → plain characters). Use the date the `v$1` tag will actually be created (per RELEASING.md step 3) — usually today, but if you know the merge/tag is happening on a different day, use that date instead. Note: this repo's release tags are lightweight (`git tag v$1`, not `git tag -a`), so Git records no separate tag-creation timestamp to verify against after the fact — `git log -1 --format=%ad v$1` only returns the tagged *commit's* author date, which can predate when the tag was actually pushed. Pick the date up front rather than trying to reconstruct it later.
5. Commit both files together with this message:

  ```text
  docs: add v$1 changelog entry
  ```

- Do NOT push or open a PR unless the user explicitly asks.

## Step 3: Product tour / What's New check

While you have the full list of user-facing changes in front of you from
Step 2, decide whether any of them are substantial enough to surface inside
the app itself (see PRD-0018):

- **New "What's New" entry** (`src/lib/whats-new.ts`): if this release ships
  something a returning user would want a heads-up about, add a
  `WhatsNewEntry` for `$1` with 1-3 `highlights` — reuse the same wording as
  the changelog bullets, trimmed to a sentence. Skip this for pure bugfix
  releases; the What's New modal is for announcing things, not apologizing
  for regressions.
- **New or updated feature tour** (`src/lib/tours.ts`): if this release adds
  a new screen, or a major new capability on an existing screen that isn't
  obvious from the UI alone, either add a new `Tour` (with matching
  `data-tour` attributes on the new elements) or add a step to an existing
  one. This is a safety net, not the primary path — ideally the feature's
  own PR already did this, the same way OEH-29 wired up tours for the five
  existing screens.

Either of these is app source, not documentation — commit each one
separately from Step 2's changelog commit (and from each other, if you add
both):

  ```text
  feat: add v$1 "What's New" entry
  ```

  ```text
  feat: add tour for <screen>
  ```

If nothing here applies, say so and move on — don't force an entry.
