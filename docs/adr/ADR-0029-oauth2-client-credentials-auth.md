# ADR-0029: OAuth2 Client-Credentials Authentication

**Date:** 2026-10-09
**Status:** Accepted
**Deciders:** Development Team
**Related:** OEH-111, ADR-0011 (Request Inspector), ADR-0015 (Credential storage)

## Context

`AuthMethod` supported `None`, `Basic` and `Bearer`. CDRs that need OAuth2 machine-to-machine auth (Cadasto, Better Platform, Keycloak-fronted EHRbase) could only be used by pasting a token that expires (typically after 1 h). The client-credentials grant issues no refresh token, so the client must re-request.

## Decision

1. New `AuthMethod::OAuth2ClientCredentials { token_url, client_id, client_secret, audience, scope, client_auth }`, server-type agnostic. The secret is stored through the credential manager (`client_secret` / `admin_client_secret`), never in `profiles.json`; public and stored forms carry `has_client_secret`.
2. **Token resolution happens in `send_instrumented`, not at ~60 call sites.** `build_request` is synchronous, so for OAuth2 it stamps an internal marker header (`x-explorer-oauth2`) holding a cache key; `send_instrumented` (async, and the single choke point for every HTTP request) removes the marker, obtains the token and sets `Authorization: Bearer`. No command module changed.
3. **Cache (`oauth.rs`):** in memory only, keyed by a hash of the client configuration (so a separate admin client gets its own entry, and editing a profile never serves a stale token). A per-entry async mutex held across the fetch gives single-flight refresh. Tokens refresh 60 s before expiry (capped at half the lifetime for very short tokens).
4. **401 handling:** on a 401 the rejected token is replaced (concurrent 401s share one re-fetch) and the request is retried exactly once.
5. **Request Inspector:** the token request is its own entry; `Authorization` is redacted as for all requests, `client_secret` in the form body and `access_token` in the response are masked in the logged copy only.
6. **Errors** name the token endpoint step and include the OAuth2 `error` / `error_description`.

## Consequences

- Existing profiles and the serde format are unchanged.
- Cache entries live in memory for the process lifetime (a handful of small structs); tokens are re-fetched after restart.
- Out of scope / follow-ups: discovery (`.well-known/smart-configuration`) button, Cadasto server-type defaults (blocked on OEH-110), admin OAuth2 in the form UI, interactive flows, `private_key_jwt`.
