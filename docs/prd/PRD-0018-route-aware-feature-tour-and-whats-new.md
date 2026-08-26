# PRD-0018: Route-Aware Feature Tour + "What's New" System

Version: 1.0
Date: 2026-08-25
Status: Implemented
Owner: openEHR Explorer
Repo slug: openehr-explorer
Depends on: none (new subsystem, touches every top-level view)

## Executive Summary

Add a granular, per-screen product tour and a lightweight "What's New" panel so first-time users and returning users after an update can discover features without reading external docs.

Each major view (EHR Browser, Composition Viewer, Template Browser, AQL Runner, Server Manager) gets its own short, skippable walkthrough that auto-starts the first time a user visits that route, and can be replayed at any time via a compass icon in the view's header. Separately, a "What's New" modal summarizes what changed after an app update, shown once per version bump, reopenable from Settings.

## Problem Statement

**Current State:**
The app has no onboarding beyond static documentation (a "Documentation" link in the sidebar) and no in-app changelog. A new user lands on the EHR Browser with no guidance on the search syntax, template context in AQL, or where to upload an OPT. A returning user who updates the app has no way to learn what changed short of reading `RELEASING.md`/GitHub Releases.

**Pain Points:**
- New users don't discover non-obvious features (structured EHR search filters, AQL Layer-3 template-aware autocomplete, FLAT path panel) without stumbling onto them or reading the README.
- There's no per-screen, contextual onboarding — existing help (e.g. the EHR search `?` popover, PRD-0017's lifecycle info icon) is opt-in reference material, not a guided introduction.
- Feature discoverability degrades over time as more screens gain more capability; a single one-time app tour would go stale and annoy repeat users.
- Users who update the app have no signal that anything changed unless they check GitHub Releases manually.

**User Personas most affected:**
- **First-time user** — needs a fast, low-friction orientation per screen without a mandatory multi-step onboarding wizard blocking the whole app.
- **Returning user after an update** — wants a 10-second summary of "what's new for me", not a full changelog.
- **Power user** — must be able to turn all of this off and never see it again, or replay a specific tour on demand when re-learning a screen.

## Goals & Success Metrics

### Goals
- Per-route, opt-out product tours that auto-start once per screen and can be replayed manually.
- A version-gated "What's New" summary shown at most once per release with user-visible highlights.
- Zero backend/network dependency — everything is local state and hand-curated frontend data.
- Fully respects user preference: a single "tours_enabled" toggle turns off all automatic behavior; manual replay always works regardless.

### Success Metrics
- A first-time user visiting each of the five tour-enabled routes sees that route's tour exactly once without further action.
- Tours never re-appear automatically after being completed or skipped, unless explicitly reset from Settings.
- The What's New modal appears exactly once per version increase that has a changelog entry, and never on a fresh install.
- No layout regression in any view header across the five extended views.

## Feature Requirements

### 1. Route-Aware Feature Tour
**Priority: P0 (Must Have)**

A tour is a named, ordered list of steps; each step highlights one DOM element (via a `data-tour="…"` attribute) with a short title/body tooltip and Back/Next/Skip controls. Tours are defined per route in `src/lib/tours.ts` and cover:

| Tour ID | Routes | Steps |
| --- | --- | --- |
| `ehrs` | `ehrs`, `ehr-detail` | Search & filter syntax, search-help popover, create EHR, browse the DIRECTORY |
| `composition` | `composition` | Pretty/JSON/FLAT tabs, Show Paths, Copy JSON |
| `templates` | `templates`, `template-detail` | Filter templates, upload an OPT |
| `aql` | `aql` | Context template autocomplete, Run, Format, Saved Queries, Stored Queries |
| `servers` | `servers` | Add a server profile |
| `contribution` | `contribution` | Audit trail summary, jump to a referenced version |

A tour auto-starts the first time a user navigates to one of its routes, provided:
- The `tours_enabled` setting is on (default: on).
- The tour's ID is not already in the persisted `completed_tours` list.
- No other tour, the analytics consent dialog, or the What's New modal is currently showing.

### 1a. Global (non-route-scoped) Tours
**Priority: P2 (added post-launch)**

Not every tour subject maps to one route. The **Request Inspector** (`inspector` tour) is a drawer mounted once, globally, in `App.vue` and visible on every screen — there's no single navigation event that means "the user is now looking at it". A `Tour` may set `global: true` and an empty `routeNames` array to opt out of route-based auto-start entirely; it's offered only via its own manual "Take a tour" trigger (the compass icon in the Request Inspector's header bar). This avoids racing the route-aware auto-start — the inspector's first HTTP entry can land at the exact moment another route's tour is already auto-starting, and a lower-priority global auto-start would either interrupt it or silently lose the race and never offer itself again.

Finishing (clicking "Done" on the last step) or skipping (Escape, the × button, or "Skip tour") both mark the tour completed — a skip is not distinguished from a completion, since either way the user has seen enough to make an informed choice not to continue.

A step whose target element never appears in the DOM (e.g. an empty state, a collapsed panel) is silently skipped rather than stalling the tour.

### 2. Manual Tour Trigger
**Priority: P0 (Must Have)**

Each of the five tour-enabled views has a small circular compass icon button in its header, titled "Take a tour of the ___". Clicking it always starts that view's tour immediately, ignoring the completed/skipped state — a deliberate replay should always work.

### 3. "What's New" Modal
**Priority: P0 (Must Have)**

`src/lib/whats-new.ts` holds a hand-curated, newest-first list of release entries (version, date, short highlights). On launch, after the app version is fetched and any first-run analytics consent decision is resolved:
- If `last_seen_version` is `null` (fresh install, or upgrading from a version predating this field), the current version is recorded as the baseline and nothing is shown — there's no "before" to compare to, and a brand-new user doesn't need historical release notes.
- Otherwise, every entry newer than `last_seen_version` is shown in a modal (oldest of the new ones first), gated on the same `tours_enabled` toggle.
- Dismissing the modal ("Got it") records the current app version as `last_seen_version`.
- A highlight can optionally link to a tour (`tourId` + `routePath`), letting "What's New" hand off directly into a route-aware tour.

The modal can be reopened at any time from Settings ("View What's New"), independent of version state.

### 4. Settings Integration
**Priority: P1 (Should Have)**

A new "Product Tours" section in Global Settings (`src/views/Settings.vue`), between Updates and Analytics:
- A toggle: "Automatically show feature tours and What's New on updates" (`tours_enabled`).
- "Replay All Tours" — clears `completed_tours` so every tour auto-starts again on next visit.
- "View What's New" — reopens the modal with the latest release entry regardless of `last_seen_version`.

## Technical Design

### Persisted State
Three new fields on `GlobalSettings` (`src-tauri/src/settings.rs`, mirrored in `src/stores/settings.ts`):

```rust
pub tours_enabled: bool,        // default true — master toggle
pub completed_tours: Vec<String>, // tour IDs seen/skipped
pub last_seen_version: Option<String>, // What's New gate
```

All three use `#[serde(default = ...)]` so existing `settings.json` files upgrade cleanly with no migration step.

### Frontend Architecture
- `src/lib/tours.ts` — pure data: `Tour`/`TourStep` types, the `TOURS` array, and `getTourForRoute`/`getTourById` lookups. Route-name aliasing (e.g. `ehrs` and `ehr-detail` both resolve to the `ehrs` tour) is handled here so a single view component reachable via multiple routes only needs one tour definition.
- `src/lib/whats-new.ts` — pure data: `WhatsNewEntry`/`WhatsNewHighlight` types, the `WHATS_NEW` array, and `compareVersions`/`getEntriesSince` helpers (semver-lite major.minor.patch comparison).
- `src/stores/tour.ts` (Pinia) — active tour/step state, `maybeAutoStart` (gated) vs. `start` (ungated, manual), `next`/`prev`/`skipStep`, and persistence via `settingsStore.saveSettings`.
- `src/stores/whatsNew.ts` (Pinia) — `checkForUpdate(currentVersion)` on launch, `showLatest()` for the manual Settings trigger, `dismiss(currentVersion)`.
- `src/components/FeatureTourOverlay.vue` — the spotlight overlay, mounted once globally in `App.vue`. Resolves the current step's target via `document.querySelector(step.target)` (not a Vue ref, since the target lives in whatever view is currently mounted), highlights it with a `box-shadow` cutout, and positions a tooltip card above/below it based on available viewport space. Retries locating the target for ~1.5s before giving up and skipping the step.
- `src/components/WhatsNewModal.vue` — styled to match the existing `AnalyticsConsentDialog.vue` pattern.
- `App.vue` orchestration: settings load → (if first run) analytics consent dialog → What's New check → route-aware tour auto-start on every navigation, each step waiting for the previous one so at most one overlay is ever visible.

### Why `data-tour` attributes instead of CSS classes
Tour step targets use a dedicated `data-tour="…"` attribute rather than reusing structural classes (`.search-input`, `.btn-primary`, …). This keeps the tour's contract with a view explicit and decoupled from styling — a view can be restyled freely without silently breaking a tour step's selector.

## Non-Goals
- No tours for the composition create/edit form (`CompositionForm.vue`) or the Global Settings page itself in this iteration — candidates for a future pass once the pattern proves out.
- ~~No telemetry on tour engagement in this iteration~~ — added: `tour_completed`, `tour_skipped`, `tour_replayed`, `whats_new_shown`, `whats_new_tour_link_clicked`, `tours_reset` (all consent-gated per ADR-0018, see `src/composables/useAnalytics.ts`).
- No remote/CDN-fetched changelog — `WHATS_NEW` ships with the app binary and is updated alongside the code that introduces each feature.

## Acceptance Criteria
- [x] Visiting `/ehrs`, `/templates`, `/aql`, `/servers`, a composition, or a contribution for the first time auto-starts that route's tour.
- [x] A `global: true` tour (the Request Inspector) never auto-starts on navigation and is reachable only via its own manual trigger.
- [x] Completing or skipping a tour persists to `completed_tours` and it never auto-starts again.
- [x] The compass-icon button in each view's header always restarts that view's tour, regardless of completion state.
- [x] A step whose target is missing from the DOM is skipped rather than freezing the tour.
- [x] `tours_enabled = false` disables both auto-start tours and the automatic What's New modal; manual triggers still work.
- [x] A fresh install shows no What's New modal but records a baseline version.
- [x] An app upgrade shows every changelog entry newer than the previously seen version, then records the new version.
- [x] Settings exposes the tours toggle, "Replay All Tours", and "View What's New".
- [x] `npm run lint`, `npm run fmt:check`, `npx vue-tsc --noEmit`, and `npm run test` all pass.
- [x] `cargo fmt -- --check` passes for the Rust changes.

## Alternatives Considered
- **A single, app-wide onboarding wizard on first launch:** Rejected — front-loads every feature before the user has any context for why it matters, and doesn't scale as more screens gain capability. Per-route tours teach a feature exactly when the user is looking at it.
- **Driver.js / Shepherd.js or another tour library:** Rejected per the project's general dependency-minimization stance (see ADR-0007/ADR-0008) — the overlay/spotlight mechanic is straightforward enough (a `box-shadow` cutout + a positioned tooltip) to implement directly, keeping bundle size and the pinned-dependency surface unchanged.
- **Remote-fetched "What's New" content:** Rejected — would require a backend endpoint and network dependency at launch for a purely cosmetic feature; a hand-curated in-repo list ships atomically with the release that introduces each feature.

## Related
- ADR-0007: Pinned Dependency Versions (why no new tour library dependency)
- PRD-0017: Template Lifecycle Indicator with Contextual Help (existing contextual-help pattern this complements)
- PRD-0011: Global Settings Page (where the new "Product Tours" section lives)
