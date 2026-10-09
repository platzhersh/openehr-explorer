//! OAuth2 client-credentials support: token cache with single-flight refresh,
//! token-request construction and error mapping. See OEH-111 / ADR-0030.
//!
//! This module is transport-agnostic: the actual HTTP call (and its Request
//! Inspector logging) is injected by the caller, which keeps the cache logic
//! unit-testable without a Tauri `AppHandle`.

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Internal marker header. `build_request` (sync) stamps it on requests of an
/// OAuth2 profile; `send_instrumented` (async) swaps it for the real
/// `Authorization: Bearer` header. It never leaves the process.
pub const MARKER_HEADER: &str = "x-explorer-oauth2";

/// Refresh this long before the token actually expires.
const REFRESH_MARGIN: Duration = Duration::from_secs(60);
const DEFAULT_EXPIRES_IN_SECS: u64 = 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClientAuthStyle {
    /// Client id/secret in an HTTP Basic `Authorization` header (preferred).
    #[default]
    Basic,
    /// Client id/secret as `client_id` / `client_secret` form fields.
    Body,
}

#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub token_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub audience: Option<String>,
    pub scope: Option<String>,
    pub client_auth: ClientAuthStyle,
}

#[derive(Debug, Clone)]
pub struct CachedToken {
    pub access_token: String,
    pub expires_at: Instant,
    /// How long before `expires_at` the token counts as stale.
    margin: Duration,
}

impl CachedToken {
    pub fn new(access_token: String, expires_in_secs: u64) -> Self {
        let lifetime = Duration::from_secs(expires_in_secs);
        Self {
            access_token,
            expires_at: Instant::now() + lifetime,
            // Very short-lived tokens would otherwise be stale on arrival.
            margin: REFRESH_MARGIN.min(lifetime / 2),
        }
    }

    fn is_fresh(&self) -> bool {
        Instant::now() + self.margin < self.expires_at
    }
}

/// One cache slot per distinct client configuration. The async mutex is held
/// across the token fetch, which is what makes refresh single-flight.
pub struct Entry {
    pub config: OAuthConfig,
    token: tokio::sync::Mutex<Option<CachedToken>>,
}

impl Entry {
    /// Return a valid access token, fetching one when the cache is empty, near
    /// expiry, or holds `stale` (a token the server just rejected with 401).
    pub async fn token<F, Fut>(&self, stale: Option<&str>, fetch: F) -> Result<String, String>
    where
        F: FnOnce(OAuthConfig) -> Fut,
        Fut: Future<Output = Result<CachedToken, String>>,
    {
        let mut slot = self.token.lock().await;
        if let Some(t) = slot.as_ref() {
            if t.is_fresh() && Some(t.access_token.as_str()) != stale {
                return Ok(t.access_token.clone());
            }
        }
        *slot = None;
        let fresh = fetch(self.config.clone()).await?;
        let value = fresh.access_token.clone();
        *slot = Some(fresh);
        Ok(value)
    }
}

type Registry = Mutex<HashMap<String, Arc<Entry>>>;

fn registry() -> &'static Registry {
    static REG: OnceLock<Registry> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Cache key: identical client configurations share a token; editing any
/// field (or using a separate admin client) gets its own entry.
fn config_key(cfg: &OAuthConfig) -> String {
    let mut h = DefaultHasher::new();
    cfg.token_url.hash(&mut h);
    cfg.client_id.hash(&mut h);
    cfg.client_secret.hash(&mut h);
    cfg.audience.hash(&mut h);
    cfg.scope.hash(&mut h);
    (cfg.client_auth as u8).hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Make `cfg` known to the cache and return the key to stamp on the request.
pub fn register(cfg: &OAuthConfig) -> String {
    let key = config_key(cfg);
    let mut reg = registry().lock().unwrap_or_else(|e| e.into_inner());
    reg.entry(key.clone()).or_insert_with(|| {
        Arc::new(Entry {
            config: cfg.clone(),
            token: tokio::sync::Mutex::new(None),
        })
    });
    key
}

pub fn lookup(key: &str) -> Option<Arc<Entry>> {
    registry()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(key)
        .cloned()
}

/// Form fields for the token request (client credentials added for `Body` style).
pub fn token_form(cfg: &OAuthConfig) -> Vec<(&'static str, String)> {
    let mut form = vec![("grant_type", "client_credentials".to_string())];
    if let Some(a) = cfg.audience.as_ref().filter(|a| !a.trim().is_empty()) {
        form.push(("audience", a.trim().to_string()));
    }
    if let Some(s) = cfg.scope.as_ref().filter(|s| !s.trim().is_empty()) {
        form.push(("scope", s.trim().to_string()));
    }
    if cfg.client_auth == ClientAuthStyle::Body {
        form.push(("client_id", cfg.client_id.clone()));
        form.push(("client_secret", cfg.client_secret.clone()));
    }
    form
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    expires_in: Option<serde_json::Value>,
    error: Option<String>,
    error_description: Option<String>,
}

/// Turn a token-endpoint response into a token or a readable error that names
/// the token step.
pub fn parse_token_response(
    token_url: &str,
    status: u16,
    body: &str,
) -> Result<CachedToken, String> {
    let parsed: Option<TokenResponse> = serde_json::from_str(body).ok();
    if (200..300).contains(&status) {
        let p = parsed.ok_or_else(|| {
            format!("OAuth2 token request to {token_url} returned a response that is not JSON")
        })?;
        let token = p.access_token.filter(|t| !t.is_empty()).ok_or_else(|| {
            format!("OAuth2 token request to {token_url} succeeded but returned no access_token")
        })?;
        let expires_in = match p.expires_in {
            Some(serde_json::Value::Number(n)) => n.as_u64(),
            Some(serde_json::Value::String(s)) => s.parse().ok(),
            _ => None,
        }
        .unwrap_or(DEFAULT_EXPIRES_IN_SECS);
        return Ok(CachedToken::new(token, expires_in));
    }

    let mut msg = format!("OAuth2 token request to {token_url} failed (HTTP {status})");
    if let Some(p) = parsed {
        if let Some(code) = p.error.as_deref().filter(|c| !c.is_empty()) {
            msg.push_str(&format!(": {code}"));
            if let Some(hint) = error_hint(code) {
                msg.push_str(&format!(" ({hint})"));
            }
        }
        if let Some(desc) = p.error_description.as_deref().filter(|d| !d.is_empty()) {
            msg.push_str(&format!(" - {desc}"));
            if desc.to_lowercase().contains("audience") {
                msg.push_str(" (this server requires a valid audience)");
            }
        }
    }
    Err(msg)
}

fn error_hint(code: &str) -> Option<&'static str> {
    match code {
        "invalid_client" => Some("client id or secret rejected"),
        "invalid_scope" => Some("scope not allowed for this client"),
        "unauthorized_client" => Some("client is not allowed to use client credentials"),
        _ => None,
    }
}

/// Error for a token request that never got an HTTP response.
pub fn unreachable_error(token_url: &str, cause: &str) -> String {
    format!("OAuth2 token endpoint {token_url} is unreachable: {cause}")
}

/// Redact secrets from a request/response body before it reaches the Request
/// Inspector: the `client_secret` form field and any `access_token` JSON value.
pub fn redact_body(body: &str) -> String {
    let mut out = body.to_string();
    // application/x-www-form-urlencoded: client_secret=...&...
    if let Some(start) = out.find("client_secret=") {
        let value_start = start + "client_secret=".len();
        let end = out[value_start..]
            .find('&')
            .map(|i| value_start + i)
            .unwrap_or(out.len());
        out.replace_range(value_start..end, "***");
    }
    // JSON: "access_token": "..."
    if let Ok(serde_json::Value::Object(mut map)) = serde_json::from_str::<serde_json::Value>(&out)
    {
        if map.contains_key("access_token") {
            map.insert(
                "access_token".into(),
                serde_json::Value::String("***".into()),
            );
            return serde_json::Value::Object(map).to_string();
        }
    }
    out
}

/// Validate a user-entered token URL: https only, except localhost.
pub fn validate_token_url(url: &str) -> Result<(), String> {
    let u = reqwest::Url::parse(url.trim()).map_err(|e| format!("Invalid token URL: {e}"))?;
    let host = u.host_str().unwrap_or_default();
    let local = host == "localhost" || host == "127.0.0.1" || host == "[::1]" || host == "::1";
    match u.scheme() {
        "https" => Ok(()),
        "http" if local => Ok(()),
        _ => Err("Token URL must use https (http is only allowed for localhost)".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn cfg(style: ClientAuthStyle) -> OAuthConfig {
        OAuthConfig {
            token_url: "https://auth.example.com/oauth/token".into(),
            client_id: "id".into(),
            client_secret: "s3cret".into(),
            audience: Some("https://api.example.com/openehr/v1".into()),
            scope: Some("api.read api.write".into()),
            client_auth: style,
        }
    }

    fn entry() -> Entry {
        Entry {
            config: cfg(ClientAuthStyle::Basic),
            token: tokio::sync::Mutex::new(None),
        }
    }

    #[tokio::test]
    async fn cache_hit_does_not_refetch() {
        let e = entry();
        let calls = AtomicUsize::new(0);
        for _ in 0..3 {
            let t = e
                .token(None, |_| async {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(CachedToken::new("tok1".into(), 3600))
                })
                .await
                .unwrap();
            assert_eq!(t, "tok1");
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn near_expiry_token_is_refreshed() {
        let e = entry();
        // 30 s left: inside the 60 s margin, but the margin is capped at half
        // the remaining lifetime for tiny tokens, so use an already-expired one.
        let t1 = e
            .token(None, |_| async {
                let mut t = CachedToken::new("old".into(), 3600);
                t.expires_at = Instant::now() - Duration::from_secs(1);
                Ok(t)
            })
            .await
            .unwrap();
        assert_eq!(t1, "old");
        let t2 = e
            .token(None, |_| async { Ok(CachedToken::new("new".into(), 3600)) })
            .await
            .unwrap();
        assert_eq!(t2, "new");
    }

    #[tokio::test]
    async fn token_within_margin_is_refreshed() {
        let e = entry();
        e.token(None, |_| async { Ok(CachedToken::new("a".into(), 3600)) })
            .await
            .unwrap();
        // Force the cached token to 10 s from expiry.
        {
            let mut slot = e.token.lock().await;
            slot.as_mut().unwrap().expires_at = Instant::now() + Duration::from_secs(10);
        }
        let t = e
            .token(None, |_| async { Ok(CachedToken::new("b".into(), 3600)) })
            .await
            .unwrap();
        assert_eq!(t, "b");
    }

    #[tokio::test]
    async fn stale_token_is_replaced() {
        let e = entry();
        e.token(None, |_| async { Ok(CachedToken::new("a".into(), 3600)) })
            .await
            .unwrap();
        let t = e
            .token(Some("a"), |_| async {
                Ok(CachedToken::new("b".into(), 3600))
            })
            .await
            .unwrap();
        assert_eq!(t, "b");
    }

    #[tokio::test]
    async fn concurrent_requests_share_one_fetch() {
        let e = Arc::new(entry());
        let calls = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();
        for _ in 0..10 {
            let e = e.clone();
            let calls = calls.clone();
            handles.push(tokio::spawn(async move {
                e.token(None, |_| async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Ok(CachedToken::new("shared".into(), 3600))
                })
                .await
                .unwrap()
            }));
        }
        for h in handles {
            assert_eq!(h.await.unwrap(), "shared");
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn concurrent_401s_share_one_refetch() {
        let e = Arc::new(entry());
        e.token(None, |_| async { Ok(CachedToken::new("old".into(), 3600)) })
            .await
            .unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();
        for _ in 0..5 {
            let e = e.clone();
            let calls = calls.clone();
            handles.push(tokio::spawn(async move {
                e.token(Some("old"), |_| async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(CachedToken::new("new".into(), 3600))
                })
                .await
                .unwrap()
            }));
        }
        for h in handles {
            assert_eq!(h.await.unwrap(), "new");
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn fetch_errors_are_not_cached() {
        let e = entry();
        assert!(e
            .token(None, |_| async { Err("boom".to_string()) })
            .await
            .is_err());
        let t = e
            .token(None, |_| async { Ok(CachedToken::new("ok".into(), 3600)) })
            .await
            .unwrap();
        assert_eq!(t, "ok");
    }

    #[test]
    fn basic_style_keeps_credentials_out_of_the_form() {
        let form = token_form(&cfg(ClientAuthStyle::Basic));
        assert!(form
            .iter()
            .any(|(k, v)| *k == "grant_type" && v == "client_credentials"));
        assert!(form.iter().any(|(k, _)| *k == "audience"));
        assert!(form
            .iter()
            .any(|(k, v)| *k == "scope" && v == "api.read api.write"));
        assert!(!form
            .iter()
            .any(|(k, _)| *k == "client_secret" || *k == "client_id"));
    }

    #[test]
    fn body_style_puts_credentials_in_the_form() {
        let form = token_form(&cfg(ClientAuthStyle::Body));
        assert!(form.iter().any(|(k, v)| *k == "client_id" && v == "id"));
        assert!(form
            .iter()
            .any(|(k, v)| *k == "client_secret" && v == "s3cret"));
    }

    #[test]
    fn empty_audience_and_scope_are_omitted() {
        let mut c = cfg(ClientAuthStyle::Basic);
        c.audience = Some("  ".into());
        c.scope = None;
        let form = token_form(&c);
        assert!(!form.iter().any(|(k, _)| *k == "audience" || *k == "scope"));
    }

    #[test]
    fn parses_a_successful_response() {
        let t = parse_token_response(
            "https://a/t",
            200,
            r#"{"access_token":"abc","token_type":"Bearer","expires_in":3600}"#,
        )
        .unwrap();
        assert_eq!(t.access_token, "abc");
        assert!(t.expires_at > Instant::now() + Duration::from_secs(3000));
    }

    #[test]
    fn missing_expires_in_defaults_to_an_hour() {
        let t = parse_token_response("https://a/t", 200, r#"{"access_token":"abc"}"#).unwrap();
        assert!(t.expires_at > Instant::now() + Duration::from_secs(3000));
    }

    #[test]
    fn success_without_access_token_is_an_error() {
        let err = parse_token_response("https://a/t", 200, "{}").unwrap_err();
        assert!(err.contains("no access_token") && err.contains("https://a/t"));
    }

    #[test]
    fn maps_invalid_client() {
        let err = parse_token_response(
            "https://a/t",
            401,
            r#"{"error":"invalid_client","error_description":"Client authentication failed"}"#,
        )
        .unwrap_err();
        assert!(err.contains("https://a/t"));
        assert!(err.contains("invalid_client"));
        assert!(err.contains("Client authentication failed"));
    }

    #[test]
    fn maps_invalid_scope() {
        let err = parse_token_response(
            "https://a/t",
            400,
            r#"{"error":"invalid_scope","error_description":"nope"}"#,
        )
        .unwrap_err();
        assert!(err.contains("invalid_scope") && err.contains("nope"));
    }

    #[test]
    fn maps_missing_audience() {
        let err = parse_token_response(
            "https://a/t",
            400,
            r#"{"error":"invalid_request","error_description":"Missing required parameter: audience"}"#,
        )
        .unwrap_err();
        assert!(err.contains("audience"));
        assert!(err.contains("requires a valid audience"));
    }

    #[test]
    fn non_json_error_body_still_names_the_step() {
        let err = parse_token_response("https://a/t", 502, "<html>bad gateway</html>").unwrap_err();
        assert!(err.contains("https://a/t") && err.contains("502"));
    }

    #[test]
    fn redacts_secrets_for_the_inspector() {
        let req =
            redact_body("grant_type=client_credentials&client_id=id&client_secret=s3cret&scope=a");
        assert!(!req.contains("s3cret"));
        assert!(req.contains("client_secret=***&scope=a"));
        let resp = redact_body(r#"{"access_token":"abc","expires_in":3600}"#);
        assert!(!resp.contains("abc"));
        assert!(resp.contains("***") && resp.contains("3600"));
    }

    #[test]
    fn token_url_validation() {
        assert!(validate_token_url("https://t.auth.prod.cadasto.io/oauth/token").is_ok());
        assert!(validate_token_url("http://localhost:8080/token").is_ok());
        assert!(validate_token_url("http://127.0.0.1:8080/token").is_ok());
        assert!(validate_token_url("http://auth.example.com/token").is_err());
        assert!(validate_token_url("not a url").is_err());
    }

    #[test]
    fn config_key_separates_distinct_clients() {
        let a = cfg(ClientAuthStyle::Basic);
        let mut b = a.clone();
        b.client_id = "admin".into();
        assert_ne!(config_key(&a), config_key(&b));
        assert_eq!(config_key(&a), config_key(&a.clone()));
    }
}
