# Changelog

All notable user-facing changes to openEHR Explorer are documented here.

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/),
but entries aren't split into `Added`/`Changed`/`Fixed` sections — each
release gets one themed heading and a flat bullet list of highlights,
matching the format already used on the
[in-app documentation changelog](https://platzhersh.github.io/openehr-explorer/docs.html#changelog).
This file and that page are kept in sync: see [RELEASING.md](RELEASING.md)
for how a release updates both.

Internal changes (chores, CI tweaks, dependency bumps, refactors) are omitted
unless they're user-visible — this is a changelog, not a commit log.

## [0.6.0] - 2026-08-25

FerroEHR Support & Default Server Profile

- **FerroEHR Support:** Added `FerroEHR` as a recognized server type, including automatic version detection against its unauthenticated status endpoint
- **Default Server Profile:** Server profiles can now be marked as the default, so it's preselected automatically on app start instead of requiring a manual switch every time
- **AQL Autocomplete Fix:** Pressing Enter or Escape to accept or dismiss an AQL autocomplete suggestion now works correctly instead of inserting a newline or falling through to the editor

## [0.5.2] - 2026-08-23

AQL Editor Highlighting Fix

- **AQL Syntax Highlighting:** Fixed a regression from the previous release where standard SQL keywords (`SELECT`, `FROM`, `WHERE`, and others) stopped being highlighted in the AQL editor while openEHR-specific keywords were added

## [0.5.1] - 2026-08-23

Linux APT Repo & AQL Editor Highlighting

- **APT Repository:** Debian/Ubuntu users can now install and update via a signed `apt` repository, published automatically on every release alongside the existing `.deb` download
- **AQL Syntax Highlighting:** The AQL editor now highlights openEHR-specific keywords (`CONTAINS`, `EHR`, `COMPOSITION`, `MATCHES`, `EXISTS`, and more) the same way it already highlighted standard SQL keywords
- **Landing Page Refresh:** Real product screenshots and a demo clip replace the old hero mockup, with a fullscreen lightbox for browsing the screenshot gallery and OS-aware download button text

## [0.5.0] - 2026-08-22

Manual Update Checks

- **Check for Updates:** Added a "Check for Updates…" item to the native app menu (macOS app menu, Windows/Linux Help menu) that triggers the same check as the Settings page button
- **Update Download Fix:** Fixed a crash that could occur while downloading and installing an update

## [0.4.3] - 2026-08-22

Windows Install & Server Form Refresh

- **Windows Install via Scoop:** `scoop install openehr-explorer` now works against the published Scoop bucket, auto-published on every release (winget support still planned)
- **Server Form Modal:** Add/Edit Server now opens in a modal dialog instead of an inline panel, for a cleaner server management workflow
- **Linux Install Docs:** Install instructions now cover the `.deb` package alongside the AppImage
- **Troubleshooting:** Documented the macOS Gatekeeper "damaged" workaround for unnotarized builds

## [0.4.2] - 2026-08-07

Signed Auto-Updates & Homebrew

- **Working Auto-Update:** The updater now signs releases with a real Tauri signing key, so in-place updates actually verify and install (previous versions shipped the updater unsigned)
- **Homebrew Cask:** `brew install --cask platzhersh/openehr-explorer/openehr-explorer` is now published and auto-updated on every release
- **SEO basics:** Sitemap, Open Graph/Twitter metadata, and Google Search Console verification for the product website

## [0.4.1] - 2026-05-06

Windows Icon Fix

- **App icon:** Replaced the single-size Windows `.ico` with a multi-resolution icon so the app looks correct in the Start menu, taskbar, and title bar at all sizes

## [0.4.0] - 2026-04-20

Template Lifecycle & OPT Readability

- **Template Lifecycle Indicator:** Templates now show a lifecycle badge (Published, Draft, etc.) with a contextual help popover explaining each state
- **Pretty-Printed OPT XML:** The raw Operational Template XML view is now indented for readability
- **VirusTotal Scanning:** Windows installers are automatically submitted to VirusTotal on tagged releases

## [0.3.0] - 2026-04-12

Auto-Update, Analytics & Security Hardening

- **Auto-Update:** Built-in updater checks GitHub Releases on startup and installs signed updates in-place (toggleable in Settings)
- **Secure Credential Storage:** Server profile credentials are stored in the OS keychain, with an encrypted-file fallback when no keychain is available
- **Opt-in Usage Analytics:** First-run consent dialog enables anonymous Aptabase telemetry to guide development — off by default and switchable any time from Settings
- **Security Hardening:** Strict Content Security Policy, Subresource Integrity for CDN assets, and URL validation for server profiles
- **Settings Files Panel:** View and open the app's config, profile, and log directories directly from Settings
- **Documentation Shortcut:** New Documentation link in the sidebar with a keyboard shortcut for quick access

## [0.2.0] - 2026-04-10

EHR Browser Search & Enhanced Features

- **EHR Browser Search:** Server-side AQL-backed search with attribute filters (subject, namespace, system, modifiable, hasCompositions)
- **Server Version Detection:** Auto-detect and display CDR version for EHRBase and Better Platform
- **Global Keyboard Shortcuts:** Navigate with Cmd+1-4, open settings with Cmd+,, toggle inspector with Cmd+Shift+I

## [0.1.0] - 2026-04-06

Initial Release

- **EHR Browser:** Paginated listing, create, delete EHRs
- **Composition Viewer:** Pretty / JSON / FLAT views, path panel, version navigation
- **Composition CRUD:** Create, edit, delete compositions in FLAT format
- **Template Browser:** List templates, inspect Web Template tree, upload OPT
- **AQL Runner:** Execute queries with 3-layer autocomplete (keywords, RM paths, template-aware paths), sortable results table, saved queries, CSV export, auto-format
- **Server Manager:** Multi-server profiles with EHRBase and Better Platform support
- **Request Inspector:** View raw HTTP requests/responses for debugging with Tree/Raw/FLAT views, copy as curl
