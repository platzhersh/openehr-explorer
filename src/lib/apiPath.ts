/** Mirrors `DEFAULT_API_PATH_PREFIX` in src-tauri/src/commands/server.rs. */
export const DEFAULT_API_PATH_PREFIX = "/rest/openehr/v1";

/**
 * The openEHR REST API root for a profile, mirroring `join_api_root` in
 * server.rs: `null`/`undefined` → default prefix, `""` or `"/"` → the base
 * URL is itself the API root. Display-only; the backend builds real URLs.
 */
export function apiRoot(baseUrl: string, prefix?: string | null): string {
  const base = baseUrl.replace(/\/+$/, "");
  const trimmed = (prefix ?? DEFAULT_API_PATH_PREFIX).trim().replace(/^\/+|\/+$/g, "");
  return trimmed ? `${base}/${trimmed}` : base;
}

/** Path-only form of the API root prefix, e.g. "/rest/openehr/v1" (or "" when none). */
export function apiPathPrefix(prefix?: string | null): string {
  const trimmed = (prefix ?? DEFAULT_API_PATH_PREFIX).trim().replace(/^\/+|\/+$/g, "");
  return trimmed ? `/${trimmed}` : "";
}
