import { autocompletion, type CompletionContext, type Completion } from "@codemirror/autocomplete";
import type { AqlPathEntry } from "./aqlPathIndex";
import { parseAliases, findAlias } from "./aliasParser";

/**
 * Layer 1: AQL Keywords
 */
const KEYWORD_COMPLETIONS: Completion[] = [
  { label: "SELECT", type: "keyword", info: "Specifies which fields to return" },
  { label: "FROM", type: "keyword", info: "Specifies the root RM type to query from" },
  { label: "WHERE", type: "keyword", info: "Filters results based on conditions" },
  { label: "CONTAINS", type: "keyword", info: "Specifies containment relationships between RM types" },
  { label: "ORDER BY", type: "keyword", info: "Sorts results by the specified path" },
  { label: "LIMIT", type: "keyword", info: "Restricts the number of returned rows" },
  { label: "OFFSET", type: "keyword", info: "Skips a number of rows before returning results" },
  { label: "AND", type: "keyword", info: "Logical AND operator" },
  { label: "OR", type: "keyword", info: "Logical OR operator" },
  { label: "NOT", type: "keyword", info: "Negates a condition" },
  { label: "AS", type: "keyword", info: "Assigns an alias" },
  { label: "LIKE", type: "keyword", info: "Pattern matching with wildcards" },
  { label: "MATCHES", type: "keyword", info: "Matches a value against a set or constraint" },
  { label: "EXISTS", type: "keyword", info: "Checks whether a path exists" },
  { label: "TOP", type: "keyword", info: "Limits the number of returned rows" },
  { label: "DISTINCT", type: "keyword", info: "Removes duplicate rows" },
  { label: "GROUP BY", type: "keyword", info: "Groups results for aggregation" },
  { label: "HAVING", type: "keyword", info: "Filters grouped results" },
  { label: "UNION", type: "keyword", info: "Combines results of two queries" },
  { label: "COUNT(*)", type: "function", info: "Counts the number of rows" },
  { label: "MAX()", type: "function", info: "Returns the maximum value" },
  { label: "MIN()", type: "function", info: "Returns the minimum value" },
  { label: "SUM()", type: "function", info: "Returns the sum of values" },
  { label: "AVG()", type: "function", info: "Returns the average value" },
];

/**
 * Layer 1: RM Type completions
 */
const RM_TYPE_COMPLETIONS: Completion[] = [
  { label: "EHR", type: "class", info: "The root electronic health record container" },
  { label: "COMPOSITION", type: "class", info: "A clinical document within an EHR" },
  { label: "OBSERVATION", type: "class", info: "Recorded clinical observations" },
  { label: "EVALUATION", type: "class", info: "Clinical assessments and diagnoses" },
  { label: "INSTRUCTION", type: "class", info: "Orders and prescriptions" },
  { label: "ACTION", type: "class", info: "Actions taken in response to instructions" },
  { label: "CLUSTER", type: "class", info: "Reusable data grouping structure" },
  { label: "ELEMENT", type: "class", info: "Leaf data element" },
  { label: "SECTION", type: "class", info: "Organizational heading within a composition" },
  { label: "ADMIN_ENTRY", type: "class", info: "Administrative data entry" },
];

/**
 * Layer 2: Static RM path completions
 */
const EHR_PATH_COMPLETIONS: Completion[] = [
  { label: "ehr_id/value", type: "property", info: "UUID of the EHR" },
  { label: "time_created/value", type: "property", info: "Creation timestamp" },
  { label: "system_id/value", type: "property", info: "Owning CDR system identifier" },
  { label: "ehr_status/subject/external_ref/id/value", type: "property", info: "Patient identifier" },
  { label: "ehr_status/subject/external_ref/namespace", type: "property", info: "Patient ID namespace" },
  { label: "ehr_status/is_queryable", type: "property", info: "Whether EHR appears in AQL" },
  { label: "ehr_status/is_modifiable", type: "property", info: "Whether EHR accepts writes" },
  { label: "ehr_status/uid/value", type: "property", info: "UID of the EHR_STATUS object" },
];

const COMPOSITION_PATH_COMPLETIONS: Completion[] = [
  { label: "uid/value", type: "property", info: "Composition UID" },
  { label: "name/value", type: "property", info: "Composition name (template name)" },
  { label: "archetype_details/template_id/value", type: "property", info: "Template ID" },
  { label: "archetype_details/archetype_id/value", type: "property", info: "Root archetype ID" },
  { label: "archetype_details/rm_version", type: "property", info: "RM version string" },
  { label: "context/start_time/value", type: "property", info: "Composition start time" },
  { label: "context/end_time/value", type: "property", info: "Composition end time" },
  { label: "context/location", type: "property", info: "Clinical location" },
  { label: "context/setting/value", type: "property", info: "Setting code" },
  { label: "language/code_string", type: "property", info: "ISO 639-1 language code" },
  { label: "territory/code_string", type: "property", info: "ISO 3166-1 territory code" },
  { label: "composer/name", type: "property", info: "Author name" },
];

export interface AqlCompletionConfig {
  templatePaths: Map<string, AqlPathEntry[]>;
  allTemplatePaths: AqlPathEntry[];
}

/**
 * Create AQL autocomplete extension with 3-tier completion
 */
export function aqlAutocomplete(config: { value: AqlCompletionConfig }) {
  return autocompletion({
    activateOnTyping: true,
    // Trigger autocomplete when typing "/" for path completions
    override: [
      (context: CompletionContext) => {
        const textBefore = context.state.doc.sliceString(0, context.pos);
        const fullText = context.state.doc.toString();

        // Check if we're after an alias/ pattern (e.g., "e/", "c/", "obs/")
        // This regex captures: alias + "/" + optional path continuation
        const aliasMatch = textBefore.match(/\b([a-zA-Z_]\w*)\/([\w/\[\]@]*)$/);

        if (aliasMatch) {
          const alias = aliasMatch[1];
          const pathPart = aliasMatch[2]; // The part after the slash
          const bindings = parseAliases(fullText);
          const binding = findAlias(bindings, alias);

          if (binding) {
            // Calculate the correct position: start of path part (after the /)
            const slashPos = context.pos - pathPart.length;

            // Layer 3: Template-aware paths for archetype-constrained aliases
            if (binding.archetypeId) {
              const paths = config.value.templatePaths.get(binding.archetypeId);
              if (paths && paths.length > 0) {
                return {
                  from: slashPos,
                  options: paths.map(p => ({
                    label: p.label,
                    type: "property",
                    detail: p.rmType,
                    info: p.aqlPath,
                    apply: p.aqlPath,
                  })),
                };
              }
              // Fallback to all template paths if no specific archetype match
              if (config.value.allTemplatePaths.length > 0) {
                return {
                  from: slashPos,
                  options: config.value.allTemplatePaths.slice(0, 100).map(p => ({
                    label: p.label,
                    type: "property",
                    detail: p.rmType,
                    info: p.aqlPath,
                    apply: p.aqlPath,
                  })),
                };
              }
            }

            // Layer 2: Static RM paths based on RM type
            if (binding.rmType === "EHR") {
              return {
                from: slashPos,
                options: EHR_PATH_COMPLETIONS,
              };
            } else if (binding.rmType === "COMPOSITION") {
              return {
                from: slashPos,
                options: COMPOSITION_PATH_COMPLETIONS,
              };
            }
          }
        }

        // Layer 1: Keywords and RM types (when not in path context)
        const word = context.matchBefore(/\w*/);
        if (!word || (word.from === word.to && !context.explicit)) {
          return null;
        }

        return {
          from: word.from,
          options: [...KEYWORD_COMPLETIONS, ...RM_TYPE_COMPLETIONS],
        };
      },
    ],
  });
}
