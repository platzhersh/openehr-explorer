/** Mirrors `DEFAULT_API_PATH_PREFIX` in src-tauri/src/commands/server.rs. */
export const DEFAULT_API_PATH_PREFIX = "/rest/openehr/v1";

// Plain index scans rather than `/^\/+|\/+$/g`: those regexes backtrack
// super-linearly on long runs of slashes (SonarCloud S5852).
function trimSlashes(value: string, leading: boolean, trailing: boolean): string {
  let start = 0;
  let end = value.length;
  if (leading) {
    while (start < end && value[start] === "/") start++;
  }
  if (trailing) {
    while (end > start && value[end - 1] === "/") end--;
  }
  return value.slice(start, end);
}

/**
 * The openEHR REST API root for a profile, mirroring `join_api_root` in
 * server.rs: `null`/`undefined` → default prefix, `""` or `"/"` → the base
 * URL is itself the API root. Display-only; the backend builds real URLs.
 */
export function apiRoot(baseUrl: string, prefix?: string | null): string {
  const base = trimSlashes(baseUrl, false, true);
  const trimmed = trimSlashes((prefix ?? DEFAULT_API_PATH_PREFIX).trim(), true, true);
  return trimmed ? `${base}/${trimmed}` : base;
}
