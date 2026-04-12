# ADR-0018: Use Aptabase for Opt-In Usage Analytics

**Date:** 2026-04-07
**Status:** Proposed
**Related:** PRD-0016 (Non-Intrusive Install Tracking and Usage Analytics)

---

## Context

As openEHR Explorer grows its user base, we need lightweight usage analytics to understand which features are used, which platforms are active, and where to focus development effort. However, as a tool used in clinical/health-IT environments, analytics must be:

- **Opt-in** (not enabled by default)
- **Privacy-first** (no PII, no fingerprinting, no cookies)
- **Transparent** (users can see exactly what is tracked)
- **GDPR-compliant** (no consent = no data)
- **Non-intrusive** (no performance impact, no UI clutter)

Several analytics platforms were evaluated for Tauri desktop app compatibility.

---

## Decision

We will **use [Aptabase](https://aptabase.com) with its official Tauri plugin** for opt-in, privacy-respecting usage analytics.

### Why Aptabase

Aptabase is purpose-built for desktop and mobile apps with privacy as a core constraint:

- **Official Tauri plugin** (`tauri-plugin-aptabase`) — first-class integration, not a web analytics shim
- **Open source** — can be self-hosted if data sovereignty requirements change
- **No cookies, no fingerprinting** — compliant with GDPR/ePrivacy without a consent banner
- **Lightweight** — events are batched and sent in background, minimal performance impact
- **Free tier** — sufficient for early-stage project (up to 20K events/month)

### Consent Model

```
┌─────────────────────────────────────────────┐
│ Settings Page                               │
│                                             │
│ Analytics                                   │
│ ┌─────────────────────────────────────────┐ │
│ │ Help improve openEHR Explorer by        │ │
│ │ sharing anonymous usage data       [OFF]│ │
│ │                                         │ │
│ │ We collect feature usage counts, app    │ │
│ │ version, and platform info. No patient  │ │
│ │ data, server URLs, or personal info     │ │
│ │ is ever collected. Learn more →         │ │
│ └─────────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
```

- **Default: OFF** — no events sent until the user explicitly enables analytics
- Consent preference persisted in the settings store (`src/stores/settings.ts`) and local config
- Toggling OFF immediately stops all event collection
- No "nag" prompts — the toggle lives in Settings and is never shown as a popup

### Implementation

**Analytics composable (`src/composables/useAnalytics.ts`):**

```typescript
import { trackEvent } from '@aptabase/tauri';
import { useSettingsStore } from '@/stores/settings';

export function useAnalytics() {
  const settings = useSettingsStore();

  function track(event: string, props?: Record<string, string | number>) {
    if (!settings.analyticsEnabled) return;
    trackEvent(event, props);
  }

  return { track };
}
```

All analytics calls go through this composable. If consent is not given, `track()` is a no-op.

**Plugin registration (`src-tauri/src/lib.rs`):**

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_aptabase::Builder::new("<app-key>").build())
    // ... other plugins
```

The Aptabase plugin initializes but only sends events when the frontend explicitly calls `trackEvent()` — which is gated behind the consent check.

### Local development: separate dev Aptabase app

To avoid polluting the production dashboard with developer clicks, local dev
builds target a **separate Aptabase project** rather than the production one.
Both projects live in the same Aptabase account on the free tier; the only
difference is the app key.

| Environment | App key source | Project name |
|---|---|---|
| Production release binaries | `secrets.APTABASE_APP_KEY` in GitHub Actions | `openehr-explorer` |
| Local `npm run tauri dev` | `.env.analytics.local` (gitignored) | `openehr-explorer-dev` |

Rationale:

- **Clean metrics.** Real-user feature-adoption data isn't drowned out by
  developers repeatedly clicking "Run AQL" during debugging.
- **No infrastructure.** Cheaper and simpler than self-hosting Aptabase via
  Docker or wiring up `A-DEV-*` local keys. Two free-tier projects cost
  nothing and take about 60 seconds to provision.
- **Same code path.** Dev and prod binaries exercise the exact same
  `tauri-plugin-aptabase` integration — we're not conditionally compiling
  instrumentation out or stubbing the transport, so regressions in the
  analytics pipeline surface during development, not after release.
- **Key is never committed.** The dev key lives in `.env.analytics.local` at
  the repo root. `*.local` is already covered by `.gitignore`, so the file
  is invisible to git. The `npm run dev:analytics` script sources this file
  before invoking `tauri dev`, and falls back to an empty key (plugin
  auto-disables) if the file is missing — so cloning the repo and running
  `npm run dev:analytics` never surprises a new contributor with network
  traffic.

See `scripts/dev-with-analytics.sh` for the wrapper and
`README.md` for the one-time setup steps.

### Event Schema

Events are deliberately coarse-grained to avoid accidental PII collection:

| Event | Properties | Purpose |
|---|---|---|
| `app_launched` | `version`, `os`, `arch` | Active installs, platform distribution |
| `server_connected` | `server_type` | CDR platform usage (ehrbase/better/generic) |
| `ehr_browsed` | — | Feature adoption |
| `composition_viewed` | `format` | Preferred viewing format |
| `aql_executed` | — | Feature adoption |
| `template_inspected` | — | Feature adoption |
| `composition_created` | — | Feature adoption |
| `composition_edited` | — | Feature adoption |
| `settings_changed` | `setting_key` | Which settings users customize |

**Hard rules:**
- Never include free-text fields (query text, template names, server URLs)
- Never include identifiers (EHR IDs, composition UIDs, user names)
- Never include clinical data of any kind
- Property values are always from a known enum or a version string

---

## Consequences

### Positive

- Feature usage data informs development priorities (e.g., "nobody uses FLAT format view" or "80% of users connect to EHRBase")
- Platform distribution data helps allocate testing and packaging effort
- Privacy-first design means no legal exposure and no user trust concerns
- Open-source tool (Aptabase) aligns with the project's open-source values
- Self-hosting option available if the project is adopted by organizations with strict data policies

### Negative

- **Opt-in means low sample rates initially:** Most users won't toggle analytics on. This is acceptable — even 10-20% opt-in provides directional data, and we prioritize trust over completeness.
- **Aptabase dependency:** Adds a third-party service dependency. Mitigated by: the plugin is open source, events are non-critical (app works fine without them), and we can self-host if the service changes terms.
- **Free tier limits (20K events/month):** Sufficient for current scale. If exceeded, either self-host or upgrade — a good problem to have.

### Neutral

- Aptabase's hosted dashboard provides basic visualizations; no custom analytics infrastructure needed
- The analytics composable pattern adds ~20 lines of code to the codebase
- Events are batched and sent asynchronously — no UI thread impact

---

## Alternatives Considered

1. **PostHog:** Feature-rich but designed for web apps. The Tauri integration is unofficial (web SDK shimmed via webview). Heavier than needed, and the free tier has a 1M event limit that could create unexpected costs at scale.

2. **Plausible Analytics:** Excellent for websites, but designed around page views and referrers — not a natural fit for desktop app feature tracking. No official Tauri plugin.

3. **Custom telemetry endpoint (Cloudflare Worker + D1/R2):** Full control, but requires building and maintaining an analytics pipeline, dashboard, and data retention policies. Premature for the project's current stage.

4. **No analytics (rely on GitHub issues and community feedback):** Status quo. Provides qualitative feedback but no quantitative data on feature usage or platform distribution. Community feedback is biased toward power users and bug reporters.

5. **Matomo:** Self-hosted option with strong privacy. But designed for web analytics (sessions, page views), not desktop app events. Would require significant adaptation.

---

## References

- [Aptabase — Privacy-First Analytics for Apps](https://aptabase.com)
- [Aptabase Tauri Plugin](https://github.com/aptabase/tauri-plugin-aptabase)
- [Aptabase Self-Hosting Guide](https://github.com/aptabase/aptabase)
- PRD-0016: Non-Intrusive Install Tracking and Usage Analytics
- ADR-0017: Use Tauri Updater Plugin with GitHub Releases
