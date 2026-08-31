/**
 * useAnalytics — thin wrapper around the `tauri-plugin-aptabase` track_event
 * command with consent gating.
 *
 * Per ADR-0018, all analytics events are:
 *   1. Opt-in only. If `settings.analyticsEnabled` is false this is a no-op.
 *   2. Routed through a single choke-point so it's easy to audit every call
 *      site and guarantee no PII ever leaks.
 *   3. Coarse-grained: only strings/numbers from a known enum or a version
 *      string are permitted in the properties payload.
 *
 * We talk to the Rust plugin directly via `invoke("plugin:aptabase|track_event")`
 * rather than pulling in `@aptabase/tauri` because the published npm package
 * still targets `@tauri-apps/api ^1.0.0` (this project is on 2.x). Talking to
 * the plugin over IPC sidesteps that peer-dep conflict — the command shape is
 * stable and documented.
 *
 * If the Aptabase app key was not injected at build time the Rust plugin
 * disables itself and every `track_event` call becomes a server-side no-op,
 * so this composable is always safe to call regardless of build configuration.
 */

import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "../stores/settings";

/** Allowed property value types — enforced by Aptabase. */
export type AnalyticsPropValue = string | number;
export type AnalyticsProps = Record<string, AnalyticsPropValue>;

/**
 * Known event names — enumerated so call sites cannot invent ad-hoc events
 * that might accidentally leak data.
 *
 * Guidelines for adding new events:
 *   - Prefer writes/destructive actions over navigation. Reads fire many
 *     times per session and blow the Aptabase free-tier quota (20k/month).
 *   - Names are snake_case, subject_verb order (`ehr_created`, not
 *     `create_ehr`).
 *   - Props MUST be from a fixed enum or a version string. Never free text
 *     the user typed (query bodies, server URLs, IDs, …).
 */
export type AnalyticsEvent =
  // --- app lifecycle ---
  /**
   * `session_started` is the ONE event that is NOT gated on the consent
   * toggle — see `trackUngated` below and ADR-0018 for the rationale. It
   * fires once per launch with a single `consent: "yes" | "no"` prop so we
   * can compute the opt-in rate and see total session counts. No version,
   * OS, or any other data rides on it.
   */
  | "session_started"
  | "app_launched"
  // --- server ---
  | "server_connected"
  | "server_profile_created"
  | "server_profile_updated"
  | "server_profile_deleted"
  // --- dashboard (OEH-17) ---
  | "dashboard_viewed"
  | "dashboard_refreshed"
  // --- EHR ---
  | "ehr_browsed"
  | "ehr_created"
  | "ehr_deleted"
  | "ehr_searched"
  // --- DIRECTORY (OEH-27) ---
  | "directory_created"
  | "directory_updated"
  | "directory_deleted"
  // --- composition ---
  | "composition_viewed"
  | "composition_created"
  | "composition_edited"
  | "composition_deleted"
  // --- contribution ---
  | "contribution_viewed"
  // --- AQL ---
  | "aql_executed"
  | "aql_query_saved"
  | "aql_query_loaded"
  | "aql_query_deleted"
  | "aql_results_exported"
  | "aql_stored_query_viewed"
  | "aql_stored_query_executed"
  // --- template ---
  | "template_inspected"
  | "template_uploaded"
  | "template_opt_exported"
  // --- terminology ---
  | "terminology_query_run"
  // --- settings / engagement ---
  | "settings_saved"
  | "documentation_opened"
  | "inspector_toggled"
  | "update_check_triggered"
  // --- feature tours / What's New (PRD-0018) ---
  /**
   * `tour_skipped` includes a `step_index` prop (0-based) — a small
   * bounded integer, not free text — so we can see how far into a tour
   * people typically get before dismissing it.
   */
  | "tour_completed"
  | "tour_skipped"
  | "tour_replayed"
  | "whats_new_shown"
  | "whats_new_tour_link_clicked"
  | "tours_reset";

export function useAnalytics() {
  const settings = useSettingsStore();

  /**
   * Send an event to Aptabase via the Rust plugin, IF the user has opted
   * in. Drops silently otherwise. This is what nearly every call site in
   * the app should use — `ehr_created`, `aql_executed`, and so on.
   */
  async function track(event: AnalyticsEvent, props?: AnalyticsProps): Promise<void> {
    // Consent gate — if the user has not opted in, drop the event on the floor.
    if (!settings.settings.analytics_enabled) return;
    await sendEvent(event, props);
  }

  /**
   * Send an event to Aptabase REGARDLESS of the consent toggle.
   *
   * This deliberately bypasses the consent gate and must be used for
   * `session_started` only, which fires exactly once per launch with a
   * `consent: "yes" | "no"` prop. The event exists so we can see the
   * denominator ("how many total sessions happened?") and compute an
   * opt-in rate. Opted-out users therefore send exactly ONE anonymous
   * event per launch — and nothing else — with no version, no OS, no
   * feature usage, no IDs, no URLs.
   *
   * See ADR-0018 "Consent model" for the rationale and the wording of
   * the user-facing disclosure.
   */
  async function trackUngated(event: AnalyticsEvent, props?: AnalyticsProps): Promise<void> {
    await sendEvent(event, props);
  }

  async function sendEvent(event: AnalyticsEvent, props?: AnalyticsProps): Promise<void> {
    try {
      await invoke("plugin:aptabase|track_event", {
        name: event,
        props: props ?? null,
      });
    } catch (err) {
      // Analytics must NEVER break the UI. Log and move on.
      // eslint-disable-next-line no-console
      console.debug("[analytics] track_event failed:", err);
    }
  }

  return { track, trackUngated };
}
