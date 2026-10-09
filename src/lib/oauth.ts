import type { AuthMethodInput, AuthMethodPublic } from "../stores/server";

type OAuth2Input = Extract<AuthMethodInput, { type: "oauth2_client_credentials" }>;

/** Token URLs must be https; plain http is only accepted for localhost. */
export function validateTokenUrl(raw: string): string | null {
  const value = raw.trim();
  if (!value) return "Token URL is required";
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    return "Token URL is not a valid URL";
  }
  if (url.protocol === "https:") return null;
  const local = ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname);
  if (url.protocol === "http:" && local) return null;
  return "Token URL must use https (http is only allowed for localhost)";
}

/**
 * Validate the OAuth2 form fields. `hasStoredSecret` is true when editing a
 * profile that already has a secret in the keychain (blank = keep it).
 */
export function validateOAuth2(auth: OAuth2Input, hasStoredSecret: boolean): string | null {
  const urlError = validateTokenUrl(auth.token_url);
  if (urlError) return urlError;
  if (!auth.client_id.trim()) return "Client ID is required";
  if (!auth.client_secret && !hasStoredSecret) return "Client secret is required";
  return null;
}

/** Host of the token endpoint, for display on the profile card. */
export function tokenHost(tokenUrl: string): string | null {
  try {
    return new URL(tokenUrl).host;
  } catch {
    return null;
  }
}

/** Human label for an auth method, e.g. on the profile card. */
export function authLabel(auth: AuthMethodPublic): string {
  switch (auth.type) {
    case "oauth2_client_credentials": {
      const host = tokenHost(auth.token_url);
      return host ? `OAuth2 (client credentials) · ${host}` : "OAuth2 (client credentials)";
    }
    case "basic":
      return "basic";
    case "bearer":
      return "bearer";
    case "none":
      return "none";
  }
}
