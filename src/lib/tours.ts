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
 */

export interface TourStep {
  /** CSS selector for the element this step highlights, e.g. `[data-tour="ehr-search"]`. */
  target: string;
  title: string;
  body: string;
}

export interface Tour {
  id: string;
  /** Route names (from `src/main.ts`) that this tour applies to. */
  routeNames: string[];
  /** Short label shown on the manual "Take a tour" trigger and in Settings. */
  label: string;
  steps: TourStep[];
}

export const TOURS: Tour[] = [
  {
    id: "ehrs",
    routeNames: ["ehrs", "ehr-detail"],
    label: "EHR Browser",
    steps: [
      {
        target: '[data-tour="ehr-search"]',
        title: "Search & filter EHRs",
        body: "Type an EHR ID prefix, or use structured filters like subject:, namespace:, system:, modifiable:, and hasCompositions: to narrow the list.",
      },
      {
        target: '[data-tour="ehr-search-help"]',
        title: "Full filter syntax",
        body: "Click the ? icon any time for the complete list of supported search filters with examples.",
      },
      {
        target: '[data-tour="ehr-create"]',
        title: "Create an EHR",
        body: "Spin up a brand-new EHR record on the active server without leaving this screen.",
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
