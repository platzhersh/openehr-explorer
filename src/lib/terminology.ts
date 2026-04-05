import { invoke } from "@tauri-apps/api/core";

/**
 * Look up a terminology code's display name via the configured FHIR terminology server.
 * Returns the preferred term string, or null if resolution is unavailable/fails.
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
