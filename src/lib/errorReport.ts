/**
 * Opt-in frontend error reporting (ADR-0018, "Crash and error reports").
 *
 * Error *messages* are never sent: they can contain server URLs, EHR IDs, AQL
 * or JSON fragments. Only a bounded error class and a scrubbed `file:line`
 * location are reported, at most once per distinct (event, kind, location) and
 * a small number of events per session so a render loop can't exhaust the
 * Aptabase free-tier quota.
 */

import type { App } from "vue";
import { useAnalytics, type AnalyticsProps } from "../composables/useAnalytics";

export type ErrorEvent = "js_error" | "promise_rejected";

const KNOWN_KINDS = new Set([
  "Error",
  "TypeError",
  "ReferenceError",
  "RangeError",
  "SyntaxError",
  "URIError",
  "EvalError",
]);

/** Max error events per session across all kinds. */
export const MAX_ERROR_EVENTS_PER_SESSION = 10;
const MAX_LOCATION_LEN = 80;

/** Map any thrown value to a fixed enum — never free text. */
export function classifyError(value: unknown): string {
  if (typeof value === "string") return "string";
  if (value instanceof Error) {
    return KNOWN_KINDS.has(value.name) ? value.name : "other";
  }
  if (value && typeof value === "object") return "object";
  return "other";
}

/** `https://host/assets/index-ab12.js` + line 3 → `index-ab12.js:3`. */
export function scrubLocation(filename: string | undefined, line: number | undefined): string {
  if (!filename) return "unknown";
  const segments = (filename.split(/[?#]/)[0] ?? "").split(/[\\/]/).filter(Boolean);
  const base = segments[segments.length - 1] ?? "unknown";
  const out = `${base}:${Number.isFinite(line) ? line : 0}`;
  return out.slice(0, MAX_LOCATION_LEN);
}

/** Pull `file:line` out of the first stack frame of an Error, if any. */
export function locationFromError(value: unknown): string {
  if (!(value instanceof Error) || !value.stack) return "unknown";
  for (const frame of value.stack.split("\n")) {
    // Frames end in `file:line:col` optionally followed by `)`.
    const parts = frame.trim().replace(/\)$/, "").split(":");
    const line = Number(parts[parts.length - 2]);
    const column = Number(parts[parts.length - 1]);
    if (parts.length < 3 || !Number.isInteger(line) || !Number.isInteger(column)) continue;
    const file = parts.slice(0, -2).join(":");
    return scrubLocation(file, line);
  }
  return "unknown";
}

/** Per-session dedupe + cap. Exported factory so tests get a fresh state. */
export function createErrorLimiter(max = MAX_ERROR_EVENTS_PER_SESSION) {
  const seen = new Set<string>();
  return function shouldSend(event: ErrorEvent, props: AnalyticsProps): boolean {
    if (seen.size >= max) return false;
    const key = `${event}|${props.kind}|${props.location}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  };
}

export function installErrorReporting(app?: App): void {
  const shouldSend = createErrorLimiter();

  function report(event: ErrorEvent, value: unknown, location: string) {
    const props: AnalyticsProps = { kind: classifyError(value), location };
    if (!shouldSend(event, props)) return;
    // `track` is consent-gated and never throws.
    useAnalytics()
      .track(event, props)
      .catch(() => {});
  }

  window.addEventListener("error", (e) => {
    const location =
      e.filename || e.lineno ? scrubLocation(e.filename, e.lineno) : locationFromError(e.error);
    report("js_error", e.error ?? e.message, location);
  });

  window.addEventListener("unhandledrejection", (e) => {
    report("promise_rejected", e.reason, locationFromError(e.reason));
  });

  if (app) {
    // Vue swallows component errors into the console, so `window.onerror`
    // never sees them.
    const previous = app.config.errorHandler;
    app.config.errorHandler = (err, instance, info) => {
      report("js_error", err, locationFromError(err));
      if (previous) previous(err, instance, info);
      else console.error(err);
    };
  }
}
