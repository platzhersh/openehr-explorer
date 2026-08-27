/**
 * Route-aware feature tour definitions — see PRD-0018.
 *
 * Each `Tour` is keyed by a stable `id` and targets one or more Vue Router
 * route *names* (a single view component can be reached via more than one
 * route, e.g. `ehrs` and `ehr-detail` both render `EhrBrowser.vue`).
 *
 * Steps target elements via `data-tour="<value>"` attributes rather than
 * structural CSS classes, so the tour keeps working if a component is
 * restyled — the attribute is a stable contract between a view and its
 * tour, not an implementation detail.
 *
 * Keep step targets to elements that render immediately (headers, buttons,
 * inputs) rather than ones that depend on async data (a specific EHR row,
 * a template card) — those may not exist yet when the tour auto-starts
 * right after navigation, and a missing target is silently skipped rather
 * than blocking the tour (see `useTourStore`).
 *
 * Most tours are route-aware (auto-start the first time their route is
 * visited) or `global` (offered only via a manual trigger, e.g. the Request
 * Inspector). The `app-intro` tour (`APP_INTRO_TOUR_ID`) is a third case:
 * it targets the always-mounted app chrome — the sidebar and Request
 * Inspector bar — rather than any one screen, so it auto-starts once at
 * launch instead of on navigation. See `maybeAutoStartIntro` in
 * `useTourStore`.
 */

export interface TourStep {
  /** CSS selector for the element this step highlights, e.g. `[data-tour="ehr-search"]`. */
  target: string;
  title: string;
  body: string;
}

export interface Tour {
  id: string;
  /** Route names (from `src/main.ts`) that this tour applies to. Empty for a `global` tour. */
  routeNames: string[];
  /**
   * True for a tour whose subject isn't a single route — e.g. the Request
   * Inspector, a drawer mounted globally in `App.vue` and visible on every
   * screen. A global tour has no entry in `TOUR_ID_BY_ROUTE` and never
   * auto-starts via `App.vue`'s route watcher; it's offered only through its
   * own manual "Take a tour" trigger (see `RequestInspector.vue`). Route-aware
   * auto-start would otherwise race whichever route's tour is already
   * showing right as the first request lands, silently dropping the intro
   * for anyone who happened to lose that race.
   */
  global?: boolean;
  /** Short label shown on the manual "Take a tour" trigger and in Settings. */
  label: string;
  steps: TourStep[];
}

/**
 * The app-wide intro tour's id. Unlike every other tour, it isn't offered
 * via route navigation (see `maybeAutoStartIntro` in `useTourStore`) — it
 * auto-starts once, at launch, before the destination route's own tour gets
 * a turn. Exported so `App.vue`/`useTourStore` don't have to hardcode the
 * string.
 */
export const APP_INTRO_TOUR_ID = "app-intro";

export const TOURS: Tour[] = [
  {
    id: APP_INTRO_TOUR_ID,
    routeNames: [],
    global: true,
    label: "App Tour",
    steps: [
      {
        target: '[data-tour="server-select"]',
        title: "Pick your server",
        body: "Every screen works against whichever server profile is selected here. Add EHRBase, Better Platform, FerroEHR, or any generic openEHR REST server from the Server Manager, then switch between them any time.",
      },
      {
        target: '[data-tour="nav-tabs"]',
        title: "Find your way around",
        body: "Browse EHRs and compositions, inspect templates, run AQL queries, and manage server profiles — each gets its own screen here, or jump straight there with Ctrl/Cmd+1 through 4.",
      },
      {
        target: '[data-tour="nav-docs"]',
        title: "Full documentation",
        body: "Opens the online docs in your browser whenever you need more depth than these tours cover (Ctrl/Cmd+Shift+D).",
      },
      {
        target: '[data-tour="nav-settings"]',
        title: "Global settings",
        body: "Analytics, update checks, and product tour preferences all live here — including replaying this tour or any single screen's tour later.",
      },
      {
        target: '[data-tour="inspector-header"]',
        title: "See every request",
        body: "The Request Inspector at the bottom of the window logs every HTTP call the app makes to the connected CDR. It's always available; toggle it with Ctrl/Cmd+Shift+I.",
      },
    ],
  },
  {
    id: "ehrs",
    routeNames: ["ehrs", "ehr-detail"],
    label: "EHR Browser",
    steps: [
      {
        target: '[data-tour="ehr-search"]',
        title: "Search EHRs",
        body: "Type an EHR ID prefix here for a quick lookup — for anything more specific, use the Filters button instead.",
      },
      {
        target: '[data-tour="ehr-search-filters"]',
        title: "Build filters without typing",
        body: "Click Filters to narrow the list with real form controls — subject, namespace, system, modifiable, has compositions, and whether the EHR has directory entries. Applied filters show up as removable chips below the search box. Prefer typing? The same dialog has a shortcut-syntax reference too.",
      },
      {
        target: '[data-tour="ehr-create"]',
        title: "Create an EHR",
        body: "Spin up a brand-new EHR record on the active server without leaving this screen.",
      },
      {
        target: '[data-tour="ehr-directory-tab"]',
        title: "Browse the DIRECTORY",
        body: "Switch to the Directory tab to explore the EHR_STATUS DIRECTORY — a versioned folder hierarchy linking compositions into a navigable structure, where the server has one set.",
      },
    ],
  },
  {
    id: "composition",
    routeNames: ["composition"],
    label: "Composition Viewer",
    steps: [
      {
        target: '[data-tour="composition-tabs"]',
        title: "Three ways to read a composition",
        body: "Switch between Pretty (the rendered tree), JSON (raw canonical JSON), and FLAT (the denormalised key-value form used by SDKs).",
      },
      {
        target: '[data-tour="composition-paths"]',
        title: "Show archetype paths",
        body: "Reveals a side panel mapping each field in the tree to its AQL / FLAT path — handy when you're writing queries or building integrations.",
      },
      {
        target: '[data-tour="composition-copy"]',
        title: "Copy the raw JSON",
        body: "One click copies the composition's canonical JSON to your clipboard.",
      },
    ],
  },
  {
    id: "templates",
    routeNames: ["templates", "template-detail"],
    label: "Template Browser",
    steps: [
      {
        target: '[data-tour="template-filter"]',
        title: "Filter templates",
        body: "Narrow the list by template ID or concept name as you type.",
      },
      {
        target: '[data-tour="template-upload"]',
        title: "Upload an OPT",
        body: "Drop an Operational Template XML file here (or click to browse) to publish it to the active server.",
      },
    ],
  },
  {
    id: "aql",
    routeNames: ["aql"],
    label: "AQL Runner",
    steps: [
      {
        target: '[data-tour="aql-context-template"]',
        title: "Add template-aware autocomplete",
        body: "Pick a template here to get path completions for its archetypes while you type your query.",
      },
      {
        target: '[data-tour="aql-run"]',
        title: "Run your query",
        body: "Execute with this button or Ctrl/Cmd+Enter from the editor.",
      },
      {
        target: '[data-tour="aql-format"]',
        title: "Tidy up",
        body: "Auto-format the query for readability with Shift+Alt+F.",
      },
      {
        target: '[data-tour="aql-saved-queries"]',
        title: "Save queries for later",
        body: "Save the current query by name, then reload it from this panel any time.",
      },
      {
        target: '[data-tour="aql-stored-queries"]',
        title: "Execute stored queries",
        body: "Server-defined STORED_QUERY definitions show up here — pick one to see its AQL, fill in its parameters, and run it without leaving the editor.",
      },
    ],
  },
  {
    id: "servers",
    routeNames: ["servers"],
    label: "Server Manager",
    steps: [
      {
        target: '[data-tour="server-add"]',
        title: "Connect a CDR",
        body: "Add a server profile for EHRBase, Better Platform, FerroEHR, or any generic openEHR REST server.",
      },
    ],
  },
  {
    id: "contribution",
    routeNames: ["contribution"],
    label: "Contribution Viewer",
    steps: [
      {
        target: '[data-tour="contribution-header"]',
        title: "See exactly what changed",
        body: "Every write to the EHR — creating or updating a composition — is wrapped in a CONTRIBUTION with a full audit trail: who changed it, when, and why. Open any version listed below directly in the Composition Viewer.",
      },
    ],
  },
  {
    id: "inspector",
    routeNames: [],
    global: true,
    label: "Request Inspector",
    steps: [
      {
        target: '[data-tour="inspector-header"]',
        title: "Every request, right here",
        body: "The Request Inspector logs every HTTP call the app makes to the connected CDR — including the AQL queries you run. It's always available at the bottom of the window; toggle it with Ctrl/Cmd+Shift+I.",
      },
      {
        target: '[data-tour="inspector-log-list"]',
        title: "Pick any request",
        body: "Select an entry to inspect its full request and response — headers, body, status, and timing.",
      },
      {
        target: '[data-tour="inspector-detail-tabs"]',
        title: "Request vs. Response",
        body: "Switch tabs to see exactly what was sent versus what came back. Use Copy as curl on the Request tab to replay it outside the app.",
      },
    ],
  },
];

const TOURS_BY_ID = new Map(TOURS.map((t) => [t.id, t]));
const TOUR_ID_BY_ROUTE = new Map(TOURS.flatMap((t) => t.routeNames.map((r) => [r, t.id])));

export function getTourById(id: string): Tour | undefined {
  return TOURS_BY_ID.get(id);
}

/** Resolve the tour (if any) that should be offered for a given route name. */
export function getTourForRoute(routeName: string | null | undefined): Tour | undefined {
  if (!routeName) return undefined;
  const id = TOUR_ID_BY_ROUTE.get(routeName);
  return id ? TOURS_BY_ID.get(id) : undefined;
}
