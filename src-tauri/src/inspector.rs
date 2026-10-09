use serde::Serialize;
use std::collections::HashMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::Emitter;
use uuid::Uuid;

use crate::oauth;

const MAX_BODY_SIZE: usize = 2 * 1024 * 1024; // 2 MB
const SENSITIVE_HEADERS: &[&str] = &["authorization", "cookie", "set-cookie"];

#[derive(Debug, Clone, Serialize)]
pub struct RequestLogEntry {
    pub id: String,
    pub timestamp_ms: u64,
    pub method: String,
    pub url: String,
    pub request_headers: HashMap<String, String>,
    pub request_body: Option<String>,
    pub status: u16,
    pub response_headers: HashMap<String, String>,
    pub response_body: Option<String>,
    pub duration_ms: u64,
    pub body_truncated: bool,
    /// `"pending"` while in flight, `"complete"` once a response arrived,
    /// `"failed"` when no response was received (see `error`).
    pub state: &'static str,
    pub error: Option<String>,
}

pub struct InstrumentedResponse {
    pub status: u16,
    pub is_success: bool,
    pub headers: HashMap<String, String>,
    pub body: String,
}

fn redact_headers(headers: &HashMap<String, String>) -> HashMap<String, String> {
    headers
        .iter()
        .map(|(k, v)| {
            if SENSITIVE_HEADERS.contains(&k.to_lowercase().as_str()) {
                (k.clone(), "[REDACTED]".to_string())
            } else {
                (k.clone(), v.clone())
            }
        })
        .collect()
}

type Emit<'a> = &'a (dyn Fn(&RequestLogEntry) + Sync);

pub async fn send_instrumented(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    builder: reqwest::RequestBuilder,
) -> Result<InstrumentedResponse, String> {
    send_with_emitter(
        &|entry| {
            let _ = app.emit("cdr-inspector-entry", entry);
        },
        client,
        builder,
    )
    .await
}

/// Send a request through the Request Inspector. Requests stamped with the
/// OAuth2 marker header (see `oauth::MARKER_HEADER`) get a bearer token
/// attached here, and are retried exactly once with a fresh token on a 401.
pub async fn send_with_emitter(
    emit: Emit<'_>,
    client: &reqwest::Client,
    builder: reqwest::RequestBuilder,
) -> Result<InstrumentedResponse, String> {
    let mut request = match builder.build() {
        Ok(r) => r,
        Err(e) => return Err(emit_build_failure(emit, &e)),
    };

    let Some(marker) = request.headers_mut().remove(oauth::MARKER_HEADER) else {
        return execute_logged(emit, client, request, false).await;
    };

    let entry = marker
        .to_str()
        .ok()
        .and_then(oauth::lookup)
        .ok_or_else(|| {
            "OAuth2 configuration for this request is no longer available".to_string()
        })?;

    let token = entry.token(None, |cfg| fetch_token(emit, cfg)).await?;
    let retry = request.try_clone();
    set_bearer(&mut request, &token)?;
    let resp = execute_logged(emit, client, request, false).await?;
    if resp.status != 401 {
        return Ok(resp);
    }

    // Expired or revoked token: replace it once and retry once.
    let Some(mut retry) = retry else {
        return Ok(resp);
    };
    let fresh = entry
        .token(Some(&token), |cfg| fetch_token(emit, cfg))
        .await?;
    set_bearer(&mut retry, &fresh)?;
    execute_logged(emit, client, retry, false).await
}

fn set_bearer(request: &mut reqwest::Request, token: &str) -> Result<(), String> {
    let mut value = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
        .map_err(|_| "OAuth2 access token contains invalid characters".to_string())?;
    value.set_sensitive(true);
    request
        .headers_mut()
        .insert(reqwest::header::AUTHORIZATION, value);
    Ok(())
}

/// The token request is its own Request Inspector entry, with the client
/// secret and the returned access token redacted.
async fn fetch_token(
    emit: Emit<'_>,
    cfg: oauth::OAuthConfig,
) -> Result<oauth::CachedToken, String> {
    // Fail closed: the backend never sends credentials over plain http to a
    // non-loopback host, whatever the profile (UI, IPC or hand-edited) says.
    oauth::validate_token_url(&cfg.token_url)?;
    // Never follow redirects: a 307/308 would replay the POST (and, for
    // `ClientAuthStyle::Body`, the client secret) to another endpoint.
    let client = &reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build token client: {e}"))?;
    let mut req = client
        .post(cfg.token_url.trim())
        .header(reqwest::header::ACCEPT, "application/json")
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(
            oauth::token_form(&cfg)
                .iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect::<Vec<_>>()
                .join("&"),
        );
    if cfg.client_auth == oauth::ClientAuthStyle::Basic {
        req = req.basic_auth(&cfg.client_id, Some(&cfg.client_secret));
    }
    let request = req
        .build()
        .map_err(|e| format!("Invalid OAuth2 token URL '{}': {e}", cfg.token_url))?;
    let resp = execute_logged(emit, client, request, true)
        .await
        .map_err(|e| oauth::unreachable_error(&cfg.token_url, &e))?;
    oauth::parse_token_response(&cfg.token_url, resp.status, &resp.body)
}

fn emit_build_failure(emit: Emit<'_>, e: &reqwest::Error) -> String {
    // Nothing was sent (e.g. an unparseable URL), but still surface the
    // attempt in the inspector. Request details aren't available here.
    let message = format!("Failed to build request: {}", e);
    let entry = RequestLogEntry {
        id: Uuid::new_v4().to_string(),
        timestamp_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        method: "N/A".to_string(),
        url: e
            .url()
            .map(|u| u.to_string())
            .unwrap_or_else(|| "(invalid request)".to_string()),
        request_headers: HashMap::new(),
        request_body: None,
        status: 0,
        response_headers: HashMap::new(),
        response_body: None,
        duration_ms: 0,
        body_truncated: false,
        state: "failed",
        error: Some(message.clone()),
    };
    emit(&entry);
    message
}

/// `redact_bodies` masks `client_secret` / `access_token` in the logged copies
/// (the caller still receives the real body).
async fn execute_logged(
    emit: Emit<'_>,
    client: &reqwest::Client,
    request: reqwest::Request,
    redact_bodies: bool,
) -> Result<InstrumentedResponse, String> {
    // Capture request details before sending
    let method = request.method().to_string();
    let url = request.url().to_string();
    let request_headers: HashMap<String, String> = request
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("[binary]").to_string()))
        .collect();
    let request_body = request
        .body()
        .and_then(|b| b.as_bytes())
        .map(|b| String::from_utf8_lossy(b).to_string())
        .map(|b| {
            if redact_bodies {
                oauth::redact_body(&b)
            } else {
                b
            }
        });

    let id = Uuid::new_v4().to_string();
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let mut entry = RequestLogEntry {
        id,
        timestamp_ms,
        method,
        url,
        request_headers: redact_headers(&request_headers),
        request_body,
        status: 0,
        response_headers: HashMap::new(),
        response_body: None,
        duration_ms: 0,
        body_truncated: false,
        state: "pending",
        error: None,
    };

    // Emit event (best-effort — don't fail the request if emit fails). The
    // pending entry lets the Request Inspector show in-flight requests; the
    // final emit below reuses the same id so the frontend updates it in place.
    emit(&entry);

    let start = Instant::now();

    let fail = |entry: &mut RequestLogEntry, message: String| -> String {
        entry.duration_ms = start.elapsed().as_millis() as u64;
        entry.state = "failed";
        entry.error = Some(message.clone());
        emit(&*entry);
        message
    };

    let response = match client.execute(request).await {
        Ok(r) => r,
        Err(e) => return Err(fail(&mut entry, format!("Request failed: {}", e))),
    };

    let status = response.status().as_u16();
    let is_success = response.status().is_success();

    let response_headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("[binary]").to_string()))
        .collect();

    let body_bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            entry.status = status;
            entry.response_headers = redact_headers(&response_headers);
            return Err(fail(
                &mut entry,
                format!("Failed to read response body: {}", e),
            ));
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    // The full body is what command handlers parse as JSON — truncating it
    // would hand them invalid JSON on large-but-legitimate responses (e.g. a
    // big AQL result set). Truncation only applies to the copy kept for the
    // Request Inspector log, which exists for human inspection.
    let body_truncated = body_bytes.len() > MAX_BODY_SIZE;
    let body = String::from_utf8_lossy(&body_bytes).to_string();
    let log_body = if body_truncated {
        String::from_utf8_lossy(&body_bytes[..MAX_BODY_SIZE]).to_string()
    } else {
        body.clone()
    };

    entry.status = status;
    entry.response_headers = redact_headers(&response_headers);
    entry.response_body = Some(if redact_bodies {
        oauth::redact_body(&log_body)
    } else {
        log_body
    });
    entry.duration_ms = duration_ms;
    entry.body_truncated = body_truncated;
    entry.state = "complete";
    emit(&entry);

    Ok(InstrumentedResponse {
        status,
        is_success,
        headers: response_headers,
        body,
    })
}

#[cfg(test)]
mod oauth_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[derive(Clone, Debug)]
    struct Seen {
        path: String,
        headers: String,
        body: String,
    }

    struct Mock {
        base: String,
        seen: Arc<Mutex<Vec<Seen>>>,
        issued: Arc<AtomicUsize>,
    }

    impl Mock {
        fn api_calls(&self) -> usize {
            self.seen
                .lock()
                .unwrap()
                .iter()
                .filter(|s| s.path == "/api")
                .count()
        }
        fn token_calls(&self) -> Vec<Seen> {
            self.seen
                .lock()
                .unwrap()
                .iter()
                .filter(|s| s.path == "/token")
                .cloned()
                .collect()
        }
    }

    /// Token endpoint issues `tok1`, `tok2`, ...; `/api` accepts only tokens
    /// numbered >= `min_valid` (so tok1 can be "revoked"). `expires_in` and
    /// `token_error` configure `/token`.
    async fn spawn_mock(min_valid: usize, expires_in: u64, token_error: bool) -> Mock {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let issued = Arc::new(AtomicUsize::new(0));
        let (seen2, issued2) = (seen.clone(), issued.clone());
        tokio::spawn(async move {
            loop {
                let (mut sock, _) = listener.accept().await.unwrap();
                let (seen, issued) = (seen2.clone(), issued2.clone());
                tokio::spawn(async move {
                    let mut buf = Vec::new();
                    let mut chunk = [0u8; 4096];
                    let (head_end, content_len) = loop {
                        let n = sock.read(&mut chunk).await.unwrap();
                        buf.extend_from_slice(&chunk[..n]);
                        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                            let head = String::from_utf8_lossy(&buf[..i]).to_lowercase();
                            let len = head
                                .lines()
                                .find_map(|l| l.strip_prefix("content-length:"))
                                .and_then(|v| v.trim().parse::<usize>().ok())
                                .unwrap_or(0);
                            break (i + 4, len);
                        }
                    };
                    while buf.len() < head_end + content_len {
                        let n = sock.read(&mut chunk).await.unwrap();
                        buf.extend_from_slice(&chunk[..n]);
                    }
                    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
                    let body = String::from_utf8_lossy(&buf[head_end..]).to_string();
                    let path = head.split_whitespace().nth(1).unwrap_or("").to_string();
                    seen.lock().unwrap().push(Seen {
                        path: path.clone(),
                        headers: head.to_lowercase(),
                        body,
                    });
                    let (status, payload) = if path == "/redirect" {
                        (307, String::new())
                    } else if path == "/token" {
                        if token_error {
                            (
                                400,
                                r#"{"error":"invalid_scope","error_description":"scope not allowed"}"#.to_string(),
                            )
                        } else {
                            let n = issued.fetch_add(1, Ordering::SeqCst) + 1;
                            (
                                200,
                                format!(
                                    r#"{{"access_token":"tok{n}","token_type":"Bearer","expires_in":{expires_in}}}"#
                                ),
                            )
                        }
                    } else {
                        let ok = head
                            .to_lowercase()
                            .lines()
                            .find_map(|l| l.strip_prefix("authorization: bearer tok"))
                            .and_then(|n| n.trim().parse::<usize>().ok())
                            .is_some_and(|n| n >= min_valid);
                        if ok {
                            (200, "{}".to_string())
                        } else {
                            (401, "{}".to_string())
                        }
                    };
                    let resp = format!(
                        "HTTP/1.1 {status} X\r\nlocation: /token\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{payload}",
                        payload.len()
                    );
                    let _ = sock.write_all(resp.as_bytes()).await;
                });
            }
        });
        Mock { base, seen, issued }
    }

    fn auth(base: &str, style: oauth::ClientAuthStyle) -> crate::commands::server::AuthMethod {
        crate::commands::server::AuthMethod::OAuth2ClientCredentials {
            token_url: format!("{base}/token"),
            client_id: "cid".into(),
            // Unique per test so tests don't share cache entries.
            client_secret: format!("sec-{base}-{style:?}"),
            audience: Some("https://api.test/openehr/v1".into()),
            scope: Some("api.read".into()),
            client_auth: style,
        }
    }

    async fn call(
        mock: &Mock,
        a: &crate::commands::server::AuthMethod,
    ) -> Result<InstrumentedResponse, String> {
        let client = reqwest::Client::new();
        let req = crate::commands::server::make_request(
            &client,
            reqwest::Method::GET,
            &format!("{}/api", mock.base),
            a,
        );
        send_with_emitter(&|_| {}, &client, req).await
    }

    #[tokio::test]
    async fn token_is_fetched_once_and_reused() {
        let mock = spawn_mock(1, 3600, false).await;
        let a = auth(&mock.base, oauth::ClientAuthStyle::Basic);
        for _ in 0..3 {
            assert_eq!(call(&mock, &a).await.unwrap().status, 200);
        }
        assert_eq!(mock.token_calls().len(), 1);
        assert_eq!(mock.api_calls(), 3);
    }

    #[tokio::test]
    async fn revoked_token_triggers_exactly_one_refetch_and_retry() {
        let mock = spawn_mock(2, 3600, false).await; // tok1 is rejected
        let a = auth(&mock.base, oauth::ClientAuthStyle::Basic);
        let resp = call(&mock, &a).await.unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(mock.token_calls().len(), 2);
        assert_eq!(mock.api_calls(), 2);
    }

    #[tokio::test]
    async fn persistent_401_is_retried_only_once() {
        let mock = spawn_mock(99, 3600, false).await; // nothing is ever valid
        let a = auth(&mock.base, oauth::ClientAuthStyle::Basic);
        let resp = call(&mock, &a).await.unwrap();
        assert_eq!(resp.status, 401);
        assert_eq!(mock.token_calls().len(), 2);
        assert_eq!(mock.api_calls(), 2);
    }

    #[tokio::test]
    async fn short_lived_token_is_renewed_before_expiry() {
        let mock = spawn_mock(1, 2, false).await; // margin = 1 s
        let a = auth(&mock.base, oauth::ClientAuthStyle::Basic);
        call(&mock, &a).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(1300)).await;
        assert_eq!(call(&mock, &a).await.unwrap().status, 200);
        assert_eq!(mock.token_calls().len(), 2);
        assert_eq!(mock.issued.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn concurrent_requests_fetch_one_token() {
        let mock = spawn_mock(1, 3600, false).await;
        let a = auth(&mock.base, oauth::ClientAuthStyle::Basic);
        let results = futures_join(&mock, &a).await;
        assert!(results.iter().all(|r| r.as_ref().unwrap().status == 200));
        assert_eq!(mock.token_calls().len(), 1);
    }

    async fn futures_join(
        mock: &Mock,
        a: &crate::commands::server::AuthMethod,
    ) -> Vec<Result<InstrumentedResponse, String>> {
        let (r1, r2, r3, r4) =
            tokio::join!(call(mock, a), call(mock, a), call(mock, a), call(mock, a));
        vec![r1, r2, r3, r4]
    }

    #[tokio::test]
    async fn basic_style_sends_credentials_in_the_header() {
        let mock = spawn_mock(1, 3600, false).await;
        let a = auth(&mock.base, oauth::ClientAuthStyle::Basic);
        call(&mock, &a).await.unwrap();
        let t = &mock.token_calls()[0];
        assert!(t.headers.contains("authorization: basic"));
        assert!(t.body.contains("grant_type=client_credentials"));
        assert!(t
            .body
            .contains("audience=https%3A%2F%2Fapi.test%2Fopenehr%2Fv1"));
        assert!(!t.body.contains("client_secret"));
    }

    #[tokio::test]
    async fn body_style_sends_credentials_in_the_form() {
        let mock = spawn_mock(1, 3600, false).await;
        let a = auth(&mock.base, oauth::ClientAuthStyle::Body);
        call(&mock, &a).await.unwrap();
        let t = &mock.token_calls()[0];
        assert!(!t.headers.contains("authorization:"));
        assert!(t.body.contains("client_id=cid"));
        assert!(t.body.contains("client_secret="));
    }

    #[tokio::test]
    async fn token_error_names_the_token_step_and_skips_the_api() {
        let mock = spawn_mock(1, 3600, true).await;
        let a = auth(&mock.base, oauth::ClientAuthStyle::Basic);
        let err = call(&mock, &a).await.err().unwrap();
        assert!(
            err.contains("/token")
                && err.contains("invalid_scope")
                && err.contains("scope not allowed")
        );
        assert_eq!(mock.api_calls(), 0);
    }

    #[tokio::test]
    async fn unreachable_token_host_is_reported() {
        let a = crate::commands::server::AuthMethod::OAuth2ClientCredentials {
            token_url: "http://127.0.0.1:1/token".into(),
            client_id: "c".into(),
            client_secret: "unreachable-secret".into(),
            audience: None,
            scope: None,
            client_auth: oauth::ClientAuthStyle::Basic,
        };
        let client = reqwest::Client::new();
        let req = crate::commands::server::make_request(
            &client,
            reqwest::Method::GET,
            "http://127.0.0.1:1/api",
            &a,
        );
        let err = send_with_emitter(&|_| {}, &client, req)
            .await
            .err()
            .unwrap();
        assert!(err.contains("unreachable") && err.contains("127.0.0.1:1/token"));
    }

    #[tokio::test]
    async fn inspector_entries_never_contain_the_secret_or_token() {
        let logged = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink = logged.clone();
        let mock = spawn_mock(1, 3600, false).await;
        let a = auth(&mock.base, oauth::ClientAuthStyle::Body);
        let secret = match &a {
            crate::commands::server::AuthMethod::OAuth2ClientCredentials {
                client_secret, ..
            } => client_secret.clone(),
            _ => unreachable!(),
        };
        let client = reqwest::Client::new();
        let req = crate::commands::server::make_request(
            &client,
            reqwest::Method::GET,
            &format!("{}/api", mock.base),
            &a,
        );
        send_with_emitter(
            &|e| sink.lock().unwrap().push(serde_json::to_string(e).unwrap()),
            &client,
            req,
        )
        .await
        .unwrap();
        let all = logged.lock().unwrap().join("\n");
        assert!(all.contains("/token"), "token request is its own entry");
        assert!(!all.contains(&urlencoding::encode(&secret).to_string()));
        assert!(!all.contains("tok1"), "access token must be redacted");
    }

    #[tokio::test]
    async fn plain_http_token_url_on_a_public_host_is_rejected_before_sending() {
        let a = crate::commands::server::AuthMethod::OAuth2ClientCredentials {
            token_url: "http://auth.example.com/token".into(),
            client_id: "c".into(),
            client_secret: "public-http-secret".into(),
            audience: None,
            scope: None,
            client_auth: oauth::ClientAuthStyle::Body,
        };
        let client = reqwest::Client::new();
        let req = crate::commands::server::make_request(
            &client,
            reqwest::Method::GET,
            "http://127.0.0.1:1/api",
            &a,
        );
        let err = send_with_emitter(&|_| {}, &client, req)
            .await
            .err()
            .unwrap();
        assert!(err.contains("https"), "{err}");
    }

    #[tokio::test]
    async fn token_endpoint_redirects_are_not_followed() {
        let mock = spawn_mock(1, 3600, false).await;
        let mut a = auth(&mock.base, oauth::ClientAuthStyle::Body);
        if let crate::commands::server::AuthMethod::OAuth2ClientCredentials { token_url, .. } =
            &mut a
        {
            *token_url = format!("{}/redirect", mock.base);
        }
        let err = call(&mock, &a).await.err().unwrap();
        assert!(err.contains("307"), "{err}");
        assert!(
            mock.token_calls().is_empty(),
            "redirect target must not be hit"
        );
    }
}
