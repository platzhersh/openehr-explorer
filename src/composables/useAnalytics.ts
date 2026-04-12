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
 */
export type AnalyticsEvent =
  | "app_launched"
  | "server_connected"
  | "ehr_browsed"
  | "composition_viewed"
  | "composition_created"
  | "composition_edited"
  | "aql_executed"
  | "template_inspected"
  | "settings_changed";

export function useAnalytics() {
  const settings = useSettingsStore();

  async function track(event: AnalyticsEvent, props?: AnalyticsProps): Promise<void> {
    // Consent gate — if the user has not opted in, drop the event on the floor.
    if (!settings.settings.analytics_enabled) return;

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

  return { track };
}
