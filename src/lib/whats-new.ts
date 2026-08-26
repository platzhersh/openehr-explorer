/**
 * "What's New" changelog entries — see PRD-0018.
 *
 * Hand-curated, deliberately not generated from git history or the full
 * CHANGELOG: this is a short, user-facing highlight reel, not a commit log.
 * Add one entry per release that ships something worth announcing; skip
 * releases that are pure bugfixes/chores.
 *
 * List newest first. `version` must be a plain `major.minor.patch` string
 * matching the app version (`package.json` / `Cargo.toml`) at release time.
 */

export interface WhatsNewHighlight {
  title: string;
  description: string;
  /** Optional tour to offer alongside this highlight, from `src/lib/tours.ts`. */
  tourId?: string;
  /** Route to navigate to before starting `tourId`, if it differs from the current route. */
  routePath?: string;
}

export interface WhatsNewEntry {
  version: string;
  date: string;
  highlights: WhatsNewHighlight[];
}

export const WHATS_NEW: WhatsNewEntry[] = [
  {
    version: "0.7.0",
    date: "2026-08-25",
    highlights: [
      {
        title: "Feature tours",
        description:
          "Every major screen now has a short, skippable walkthrough of its key features. Look for the compass icon in a view's header to replay one any time.",
      },
      {
        title: "What's New",
        description:
          "This panel — a quick summary of what changed after an update, without digging through the full changelog. Reopen it any time from Settings.",
      },
    ],
  },
];

/** Parses "major.minor.patch" into a comparable tuple. Non-numeric or malformed input sorts as 0.0.0. */
function parseVersion(version: string): [number, number, number] {
  const parts = version.split(".").map((p) => Number.parseInt(p, 10));
  return [parts[0] || 0, parts[1] || 0, parts[2] || 0];
}

/** Returns -1/0/1 as `a` is older/equal/newer than `b`. */
export function compareVersions(a: string, b: string): number {
  const [aMaj, aMin, aPatch] = parseVersion(a);
  const [bMaj, bMin, bPatch] = parseVersion(b);
  if (aMaj !== bMaj) return aMaj < bMaj ? -1 : 1;
  if (aMin !== bMin) return aMin < bMin ? -1 : 1;
  if (aPatch !== bPatch) return aPatch < bPatch ? -1 : 1;
  return 0;
}

/**
 * Entries strictly newer than `lastSeenVersion`, oldest first (so a user who
 * skipped several releases sees them in chronological order). Returns all
 * entries if `lastSeenVersion` is null — callers gate that case themselves
 * (a fresh install shouldn't be shown the full history, see `useWhatsNewStore`).
 */
export function getEntriesSince(lastSeenVersion: string | null): WhatsNewEntry[] {
  const entries = lastSeenVersion
    ? WHATS_NEW.filter((e) => compareVersions(e.version, lastSeenVersion) > 0)
    : WHATS_NEW.slice();
  return entries.sort((a, b) => compareVersions(a.version, b.version));
}
