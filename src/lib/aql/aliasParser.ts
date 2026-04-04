export interface AliasBinding {
  alias: string;
  rmType: string;
  archetypeId?: string;
}

/**
 * Parse AQL query text to extract alias-to-RM-type bindings.
 *
 * Handles patterns like:
 *   FROM EHR e
 *   FROM EHR e[ehr_id/value='...']
 *   CONTAINS COMPOSITION c
 *   CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.encounter.v1]
 *   CONTAINS OBSERVATION obs[openEHR-EHR-OBSERVATION.blood_pressure.v2]
 */
export function parseAliases(queryText: string): AliasBinding[] {
  const bindings: AliasBinding[] = [];
  const seen = new Set<string>();

  // Match: RM_TYPE alias or RM_TYPE alias[archetype_id_or_filter]
  // The RM type must be a known openEHR type keyword
  const rmTypes =
    "EHR|COMPOSITION|OBSERVATION|EVALUATION|INSTRUCTION|ACTION|ADMIN_ENTRY|CLUSTER|ELEMENT|SECTION";

  const pattern = new RegExp(`\\b(${rmTypes})\\s+([a-zA-Z_]\\w*)(?:\\s*\\[([^\\]]+)\\])?`, "gi");

  let match: RegExpExecArray | null;
  while ((match = pattern.exec(queryText)) !== null) {
    const rmType = match[1].toUpperCase();
    const alias = match[2];
    const bracketContent = match[3];

    // Skip if the "alias" is actually another RM type keyword (e.g., "CONTAINS COMPOSITION")
    if (new RegExp(`^(${rmTypes})$`, "i").test(alias)) {
      continue;
    }

    // Skip duplicates (first wins)
    if (seen.has(alias.toLowerCase())) {
      continue;
    }
    seen.add(alias.toLowerCase());

    let archetypeId: string | undefined;
    if (bracketContent) {
      // Extract archetype ID if it looks like one (openEHR-EHR-...)
      const archetypeMatch = bracketContent.match(/(openEHR-[\w.-]+)/);
      if (archetypeMatch) {
        archetypeId = archetypeMatch[1];
      }
    }

    bindings.push({ alias, rmType, archetypeId });
  }

  return bindings;
}

/**
 * Find the alias binding for a given alias name (case-insensitive).
 */
export function findAlias(bindings: AliasBinding[], alias: string): AliasBinding | undefined {
  return bindings.find((b) => b.alias.toLowerCase() === alias.toLowerCase());
}
