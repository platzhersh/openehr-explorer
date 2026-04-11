# PRD-0016: Non-Intrusive Install Tracking and Usage Analytics

**Date:** 2026-04-07
**Status:** Draft
**Owner:** openEHR Explorer
**Effort estimate:** S (Tier 1: ~0.5 day) / M (Tier 2: ~1-2 days) / L (Tier 3: ~2-3 days)
**Priority:** P1 (Should Have)

---

## Problem Statement

openEHR Explorer is distributed as a desktop application via GitHub Releases (.dmg, .deb, .AppImage, .exe). Currently, the only visibility into adoption is GitHub's per-asset download count — which is:

- **Not visible in the GitHub UI** (only available via API)
- **Not deduplicated** (one user downloading twice = 2 counts)
- **Not actionable** (no OS/arch breakdown, no version tracking, no "active installs" metric)
- **Not real-time** (no way to know if users actually launched the app after downloading)

As an open-source project seeking adoption in the openEHR community, understanding install base and usage patterns is essential for:

- **Prioritizing platform support** (macOS vs Windows vs Linux effort allocation)
- **Gauging adoption** (are people actually using the tool after downloading?)
- **Planning deprecation** (which versions are still in the wild?)
- **Demonstrating traction** (to potential contributors and the openEHR community)

---

## Goals

- Track actual application installs (not just downloads) with app version and OS/platform information
- Provide an auto-update mechanism so users stay on the latest version without manual re-downloads
- Lay the groundwork for opt-in, privacy-respecting usage analytics
- Maintain user trust: no PII collection, no tracking without consent for analytics beyond update checks

---

## Non-Goals

- Collecting personally identifiable information (PII)
- Tracking individual user behavior or sessions (Tier 1-2)
- Building a custom analytics dashboard (use Aptabase's hosted dashboard)
- Requiring users to create accounts or authenticate
- Crash reporting (separate concern, separate tool)

---

## Solution: Three Tiers

### Tier 1 — Tauri Updater Plugin with GitHub Releases (P0)

**What:** Add the official `tauri-plugin-updater` configured to check GitHub Releases for new versions. Every update check naturally sends the current app version and platform to the update endpoint.

**How it works:**

1. On app launch, the updater plugin sends a GET request to a configured endpoint with:
   - Current app version (from `tauri.conf.json`)
   - Platform and architecture (e.g., `darwin-aarch64`, `linux-x86_64`, `windows-x86_64`)
2. The endpoint responds with either "no update available" (204) or update metadata (200 + JSON)
3. If an update is available, the user is prompted to download and install it

**Update endpoint:** GitHub Releases (built-in support in Tauri updater). No custom server required for the basic update flow.

**Tracking value:** GitHub Release download counts will now reflect actual update installations, not just initial downloads. Version distribution becomes visible through release asset download patterns.

**Affected files:**
- `src-tauri/Cargo.toml` — add `tauri-plugin-updater` dependency
- `src-tauri/tauri.conf.json` — add updater configuration with GitHub endpoint
- `src-tauri/src/lib.rs` — register updater plugin
- `src-tauri/capabilities/default.json` — add updater permissions
- `package.json` — add `@tauri-apps/plugin-updater` frontend dependency
- New: `src/components/UpdateNotification.vue` — non-intrusive update prompt UI

### Tier 2 — Lightweight Update Proxy for Install Counting (P1)

**What:** Place a lightweight proxy (Cloudflare Worker or similar edge function) between the app and GitHub Releases. The proxy logs update check requests (version, platform, timestamp) before forwarding to GitHub.

**What gets logged (per update check):**
- App version (e.g., `0.1.0`)
- Platform/architecture (e.g., `darwin-aarch64`)
- Timestamp
- Hashed IP (for unique-install approximation, discarded after aggregation)

**What does NOT get logged:**
- Raw IP addresses (hashed and discarded after daily aggregation)
- Any user identity or hardware fingerprint
- Any application data or usage patterns

**Privacy model:** The proxy sees the same information any HTTP server sees (IP + headers). By hashing and aggregating daily, we reduce this to anonymous counters: "3 unique macOS-aarch64 installs on v0.2.0 today."

**Affected files:**
- New: `infra/update-proxy/` — Cloudflare Worker or Vercel Edge Function
- `src-tauri/tauri.conf.json` — point updater endpoint to proxy URL instead of GitHub directly

### Tier 3 — Opt-In Usage Analytics with Aptabase (P2)

**What:** Integrate [Aptabase](https://aptabase.com) — an open-source, privacy-first analytics platform with an official Tauri plugin. This provides feature-level usage insights while respecting user consent.

**Key properties of Aptabase:**
- Open source (self-hostable if needed)
- Built specifically for desktop/mobile apps
- No cookies, no fingerprinting, no PII
- GDPR-compliant by design
- Official Tauri plugin available (`tauri-plugin-aptabase`)

**Consent model:**
- Analytics are **disabled by default** (opt-in)
- A toggle in the existing Settings page (`src/views/SettingsPage.vue`): "Help improve openEHR Explorer by sharing anonymous usage data"
- Consent preference stored locally and respected immediately
- Users can opt out at any time

**Events to track (when opted in):**
| Event | Properties | Purpose |
|---|---|---|
| `app_launched` | `version`, `os`, `arch` | Active user count |
| `server_connected` | `server_type` (ehrbase/better/generic) | Platform compatibility priorities |
| `ehr_browsed` | — | Feature usage |
| `composition_viewed` | `format` (pretty/json/flat) | Format preference |
| `aql_executed` | — | Feature usage |
| `template_inspected` | — | Feature usage |
| `composition_created` | — | Feature usage |

**Explicitly NOT tracked:**
- Server URLs or hostnames
- Patient data, EHR IDs, or composition content
- Query text or results
- Template names or content
- Any text the user types

**Affected files:**
- `src-tauri/Cargo.toml` — add `tauri-plugin-aptabase` dependency
- `src-tauri/src/lib.rs` — register Aptabase plugin (conditionally)
- `src-tauri/capabilities/default.json` — add Aptabase permissions
- `package.json` — add `@aptabase/tauri` frontend dependency
- `src/views/SettingsPage.vue` — add analytics opt-in toggle
- `src/composables/useAnalytics.ts` — thin wrapper for event tracking with consent check
- `src/stores/settings.ts` — persist analytics consent preference

---

## Implementation Plan

### Phase 1: Auto-Updater (Tier 1)
- [ ] Add `tauri-plugin-updater` to Rust and frontend dependencies
- [ ] Configure updater in `tauri.conf.json` with GitHub Releases endpoint
- [ ] Register plugin in `lib.rs` and add capabilities
- [ ] Build `UpdateNotification.vue` component (non-modal, dismissible)
- [ ] Set up code signing for update bundles (required by Tauri updater)
- [ ] Update CI to generate updater-compatible release artifacts (`.tar.gz` + `.sig` on macOS/Linux, `.nsis` + `.sig` on Windows)
- [ ] Test update flow end-to-end with a pre-release tag

### Phase 2: Update Proxy (Tier 2)
- [ ] Deploy a Cloudflare Worker that proxies update checks to GitHub Releases
- [ ] Add request logging (version, platform, hashed IP, timestamp)
- [ ] Build a simple daily aggregation script
- [ ] Switch app endpoint from GitHub to proxy URL

### Phase 3: Opt-In Analytics (Tier 3)
- [ ] Register for Aptabase app key
- [ ] Add `tauri-plugin-aptabase` to dependencies
- [ ] Add consent toggle to Settings page
- [ ] Implement `useAnalytics` composable with consent gating
- [ ] Instrument key user flows with events
- [ ] Document privacy policy in app and on product website

---

## Acceptance Criteria

- [ ] App checks for updates on launch and displays a non-intrusive notification when an update is available
- [ ] Update check sends app version and platform to the endpoint
- [ ] User can dismiss update notification and continue using the app
- [ ] (Tier 2) Update proxy logs version + platform + daily unique counts
- [ ] (Tier 3) Analytics toggle exists in Settings, defaults to OFF
- [ ] (Tier 3) No analytics events are sent when toggle is OFF
- [ ] (Tier 3) Tracked events contain no PII or clinical data
- [ ] (Tier 3) User can opt out at any time and events stop immediately

---

## Related

- ADR-0017: Use Tauri Updater Plugin with GitHub Releases
- ADR-0018: Use Aptabase for Opt-In Usage Analytics
