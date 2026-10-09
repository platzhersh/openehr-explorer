/** Mirrors `DEFAULT_API_PATH_PREFIX` in src-tauri/src/commands/server.rs. */
export const DEFAULT_API_PATH_PREFIX = "/rest/openehr/v1";

/** Mirrors `CADASTO_API_PATH_PREFIX`: Cadasto serves the API directly under `/openehr/v1`. */
export const CADASTO_API_PATH_PREFIX = "/openehr/v1";

/**
 * The prefix used when a profile doesn't set one, which depends on the server
 * type. Mirrors `ServerType::default_api_path_prefix` in server.rs.
 */
export function defaultApiPathPrefix(serverType?: string | null): string {
  return serverType === "cadasto" ? CADASTO_API_PATH_PREFIX : DEFAULT_API_PATH_PREFIX;
}

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
 * The openEHR REST API root for a profile, mirroring `ServerProfile::api_root`
 * in server.rs: `null`/`undefined` → the server type's default prefix, `""` or
 * `"/"` → the base URL is itself the API root. Display-only; the backend
 * builds real URLs.
 */
export function apiRoot(
  baseUrl: string,
  prefix?: string | null,
  serverType?: string | null,
): string {
  const base = trimSlashes(baseUrl, false, true);
  const trimmed = trimSlashes((prefix ?? defaultApiPathPrefix(serverType)).trim(), true, true);
  return trimmed ? `${base}/${trimmed}` : base;
}

/**
 * Label for a profile's prefix when it differs from its server type's default,
 * else `null`. An explicit empty prefix ("base URL is the API root") is shown
 * as `/`.
 */
export function customApiPrefixLabel(
  prefix?: string | null,
  serverType?: string | null,
): string | null {
  if (prefix == null) return null;
  const trimmed = trimSlashes(prefix.trim(), true, true);
  const normalized = trimmed ? `/${trimmed}` : "/";
  return normalized === defaultApiPathPrefix(serverType) ? null : normalized;
}
