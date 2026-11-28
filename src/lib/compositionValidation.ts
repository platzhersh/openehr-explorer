import type { ServerProfile } from "../stores/server";

/** Result of the `validate_composition` command. */
export interface CompositionValidation {
  valid: boolean;
  code: string | null;
  message: string | null;
  details: string[];
}

/**
 * Whether a composition can be checked without being saved. The openEHR REST API
 * has no such operation; only Better Platform's own endpoint is implemented
 * (ADR-0030). Mirrors `composition_validate_url` in
 * src-tauri/src/commands/composition.rs.
 */
export function supportsCompositionValidation(
  serverType: ServerProfile["server_type"] | undefined,
): boolean {
  return serverType === "better_platform";
}
