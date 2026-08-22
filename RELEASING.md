# Releasing openEHR Explorer

This document describes how to cut a release. Releases are **tag-triggered**:
pushing a `v*` tag to `main` kicks off the entire build-and-publish pipeline.
There is no separate "click a button to release" step beyond that.

## TL;DR

```bash
# 1. Bump the version (see below)
/bump-version 0.5.0

# 2. Update the changelog (manual, see below)

# 3. Open a PR, get it merged to main

# 4. Tag the merge commit and push the tag
git checkout main && git pull
git tag v0.5.0
git push origin v0.5.0
```

Pushing the tag is the only manual trigger. Everything else — building for
all three platforms, creating the GitHub Release, generating the updater
manifest, and updating the Homebrew/Scoop/WinGet package listings — happens
automatically in CI.

## Step by step

### 1. Bump the version

Use the `/bump-version <version>` slash command
(`.claude/commands/bump-version.md`). It updates the version string in all
of the files that need to stay in sync:

- `package.json`
- `package-lock.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `src-tauri/tauri.conf.json`
- `docs/index.html` (version badge)
- `docs/docs.html` (Linux AppImage install snippet)

It does **not** touch the changelog — that's step 2, on purpose, so version
bumps and changelog writing stay separate commits.

### 2. Update the changelog

Add a new `<h3>` entry under `<section id="changelog">` in `docs/docs.html`,
following the existing format (version + one-line theme, then a bullet list
of user-facing highlights). This is a manual, hand-written summary — it is
not generated from commit history.

> Note: as of this writing the changelog section lags a few versions behind
> the latest tag. Worth reconciling it as part of the next release so it
> doesn't drift further.

### 3. Merge to `main`

Open a PR with the version bump + changelog update, get it reviewed, and
merge it. The tag in the next step should point at this merge commit.

### 4. Tag and push

```bash
git checkout main && git pull
git tag v0.5.0          # must match the version.
git push origin v0.5.0
```

Pushing the tag is what triggers the release build in `.github/workflows/ci.yml`.
The frontend/rust check jobs also run again on the tag, but the interesting
jobs are the platform builds:

| Job | Runs on | What it does |
|---|---|---|
| `build-macos` | tag push (also every push/PR, but only uploads on a tag) | Universal DMG, code-signs if Apple secrets are configured, uploads DMG + updater artifacts to the GitHub Release |
| `build-windows` | tag push, or manual `workflow_dispatch` with `build-windows: true` | NSIS installer, uploads to the Release, submits the installer to VirusTotal (tag pushes only) |
| `build-linux` | tag push, or manual `workflow_dispatch` with `build-linux: true` | `.deb` + `.AppImage`, uploads to the Release |
| `publish-updater-manifest` | after all three builds, tag push only | Generates `latest.json` (used by the in-app Tauri updater) from the release assets and uploads it |
| `publish-homebrew-cask` | after `build-macos`, tag push only | Pushes an updated Cask to `platzhersh/homebrew-openehr-explorer` (needs `HOMEBREW_TAP_TOKEN`; no-ops if unset) |
| `publish-scoop-bucket` | after `build-windows`, tag push only | Pushes an updated manifest to `platzhersh/scoop-openehr-explorer` (needs `SCOOP_BUCKET_TOKEN`; no-ops if unset) |

The GitHub Release itself is created automatically by `softprops/action-gh-release`
the first time a build job uploads to that tag — you don't create it by hand,
and it's published (not a draft) as soon as it exists.

### 5. WinGet (automatic, separate workflow)

Once the GitHub Release transitions to `released` (i.e. it's published — which
happens as soon as the jobs above create it), `.github/workflows/winget.yml`
fires on the `release: [released]` event and updates the `winget-pkgs`
manifest via `winget-releaser`. This requires `WINGET_TOKEN` (a classic PAT
with `public_repo` scope); it no-ops silently if unset.

> Caveat: `winget-releaser` can only update an *existing* WinGet manifest.
> The very first submission for a new package has to be authored and opened
> by hand — see issue OEH-5.

### 6. Verify

After the workflow run finishes:

- Check the [Releases page](https://github.com/platzhersh/openehr-explorer/releases)
  — all three installers (`.dmg`, `.exe`, `.deb`/`.AppImage`) plus `latest.json`
  should be attached.
- If you're testing the auto-updater, launch a build from the *previous*
  version and confirm it detects and installs the new one.
- Check the Homebrew tap and Scoop bucket repos picked up the version bump
  (if their tokens are configured).

## Manual/ad-hoc builds (not a release)

You can build Windows or Linux installers off any branch without cutting a
release by running the `CI` workflow manually (`workflow_dispatch`) from the
Actions tab, with `build-windows` and/or `build-linux` set to `true`. This
builds the installers as workflow artifacts only — it does **not** create or
attach anything to a GitHub Release, since that only happens on `refs/tags/*`.

## Required secrets

Most of the pipeline degrades gracefully (skips the optional step) if a
secret isn't configured, rather than failing the release:

| Secret | Used for | Behavior if missing |
|---|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Signing updater artifacts | Updater artifacts (`.sig`) aren't produced; auto-update won't work for that release |
| `APPLE_CERTIFICATE` / `APPLE_CERTIFICATE_PASSWORD` / `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` / `APPLE_SIGNING_IDENTITY` | macOS code signing + notarization | Falls back to ad-hoc signing (`-`); users see "unidentified developer" instead of notarized |
| `APTABASE_APP_KEY` | Opt-in usage analytics | Analytics plugin disables itself at runtime |
| `HOMEBREW_TAP_TOKEN` | Publishing the Homebrew Cask | `publish-homebrew-cask` skips with a notice |
| `SCOOP_BUCKET_TOKEN` | Publishing the Scoop manifest | `publish-scoop-bucket` skips with a notice |
| `WINGET_TOKEN` | Publishing the WinGet manifest | `winget.yml` job condition is false; nothing runs |
| `VT_API_KEY` | VirusTotal scan of Windows installers | Scan step is skipped (only runs on tag pushes anyway) |

None of these being unset blocks a release from happening — they only widen
or narrow which distribution channels get updated.
