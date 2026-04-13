/**
 * Template lifecycle classification.
 *
 * The openEHR AOM 2 / ADL 2 specification defines a fixed set of lifecycle
 * states for archetypes and templates. In practice, OPTs produced by older
 * or non-conformant tooling (e.g. Ocean Template Designer) ship with values
 * outside this vocabulary — notably `Initial`, `AuthorDraft`, or empty.
 *
 * This module provides a pure classifier used by the Template Browser to
 * decide whether to warn the user about a non-standard value.
 *
 * See: https://specifications.openehr.org/releases/AM/latest/AOM2.html
 */

export const SPEC_LIFECYCLE_VALUES = [
  "unmanaged",
  "in_development",
  "release_candidate",
  "published",
  "superseded",
  "deprecated",
  "obsolete",
] as const;

export type SpecLifecycleValue = (typeof SPEC_LIFECYCLE_VALUES)[number];

export type LifecycleClassification = "spec" | "non-standard" | "empty";

/**
 * Classify a lifecycle_state string from an OPT.
 *
 * - Empty/whitespace/undefined → `"empty"`
 * - Case-insensitive match against spec vocabulary → `"spec"`
 * - Anything else (e.g. "Initial", "AuthorDraft") → `"non-standard"`
 */
export function classifyLifecycle(value: string | undefined | null): LifecycleClassification {
  if (!value || value.trim() === "") return "empty";
  const normalised = value.trim().toLowerCase();
  if ((SPEC_LIFECYCLE_VALUES as readonly string[]).includes(normalised)) {
    return "spec";
  }
  return "non-standard";
}

/**
 * Human-readable descriptions of each spec lifecycle value, used to
 * populate the contextual-help popover in the Template Metadata panel.
 */
export const LIFECYCLE_DESCRIPTIONS: Record<SpecLifecycleValue, string> = {
  unmanaged: "Not under any governance process; typically a working draft not yet submitted.",
  in_development: "Actively being designed or revised; not ready for production use.",
  release_candidate: "Proposed for release; undergoing final review.",
  published: "Officially released and suitable for production use.",
  superseded: "Replaced by a newer version; kept for historical reference.",
  deprecated: "Still valid but actively discouraged; migration advised.",
  obsolete: "No longer valid for use.",
};

/**
 * Semantic colour hint for a recognised spec lifecycle value. Non-standard
 * values always take the amber/warning colour regardless of this table.
 */
export type LifecycleSemanticTone = "positive" | "accent" | "neutral" | "muted";

export function semanticToneFor(value: SpecLifecycleValue): LifecycleSemanticTone {
  switch (value) {
    case "published":
      return "positive";
    case "release_candidate":
      return "accent";
    case "in_development":
    case "unmanaged":
      return "neutral";
    case "superseded":
    case "deprecated":
    case "obsolete":
      return "muted";
  }
}
