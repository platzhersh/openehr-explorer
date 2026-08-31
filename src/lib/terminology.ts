import { invoke } from "@tauri-apps/api/core";

/**
 * Look up a terminology code's display name via the configured FHIR terminology server.
 * Returns the preferred term string, or null if resolution is unavailable/fails.
 *
 * Used for passive/lazy resolution (composition/template tree hover) — failures are
 * swallowed rather than surfaced, matching the graceful-degradation contract of the
 * `lookup_code` backend command. For user-triggered lookups, use `describeCode` instead,
 * which propagates errors so the caller can show them.
 */
export async function lookupCode(
  serverId: string,
  system: string,
  code: string,
): Promise<string | null> {
  try {
    return await invoke<string | null>("lookup_code", {
      serverId,
      system,
      code,
    });
  } catch {
    return null; // Graceful degradation
  }
}

export interface TerminologyProperty {
  code: string;
  value: string;
}

export interface CodeDescription {
  system: string;
  code: string;
  display: string | null;
  designations: string[];
  properties: TerminologyProperty[];
}

/**
 * Describe a single code via `CodeSystem/$lookup` — preferred term plus any
 * designations (synonyms/translations) and properties (e.g. `parent`,
 * `inactive`) the terminology server reports. Unlike `lookupCode`, this is a
 * user-triggered "run this query" action, so it rejects on failure (no
 * terminology server configured, network error, 404, …) rather than
 * resolving to `null` — the caller is expected to show the error.
 */
export async function describeCode(
  serverId: string,
  system: string,
  code: string,
): Promise<CodeDescription> {
  return invoke<CodeDescription>("describe_code", { serverId, system, code });
}

export interface TerminologyConcept {
  system: string | null;
  code: string | null;
  display: string | null;
}

export interface ValueSetExpansion {
  total: number | null;
  concepts: TerminologyConcept[];
}

/**
 * Expand a value set via `ValueSet/$expand`. `valueSet` is a canonical URL
 * (or FHIR ValueSet id); `filter` narrows by display text for large value
 * sets; `count` caps how many concepts come back.
 */
export async function expandValueSet(
  serverId: string,
  valueSet: string,
  filter?: string,
  count?: number,
): Promise<ValueSetExpansion> {
  return invoke<ValueSetExpansion>("expand_valueset", {
    serverId,
    valueSet,
    filter: filter || null,
    count: count ?? null,
  });
}

export interface CodeValidation {
  result: boolean;
  message: string | null;
  display: string | null;
}

/**
 * Test whether a code is a member of a value set (`ValueSet/$validate-code`
 * when `valueSet` is given) or simply valid in a code system
 * (`CodeSystem/$validate-code` otherwise).
 */
export async function validateCode(
  serverId: string,
  system: string,
  code: string,
  valueSet?: string,
): Promise<CodeValidation> {
  return invoke<CodeValidation>("validate_code", {
    serverId,
    system,
    code,
    valueSet: valueSet || null,
  });
}

export interface SubsumptionResult {
  outcome: string;
}

/**
 * Test the subsumption relationship between two codes in the same code
 * system via `CodeSystem/$subsumes` — one of `equivalent`, `subsumes`,
 * `subsumed-by`, or `not-subsumed`.
 */
export async function testSubsumption(
  serverId: string,
  system: string,
  codeA: string,
  codeB: string,
): Promise<SubsumptionResult> {
  return invoke<SubsumptionResult>("test_subsumption", {
    serverId,
    system,
    codeA,
    codeB,
  });
}
