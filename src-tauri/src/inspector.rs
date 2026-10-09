use serde::Serialize;
use std::collections::HashMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::Emitter;
use uuid::Uuid;

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

pub async fn send_instrumented(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    builder: reqwest::RequestBuilder,
) -> Result<InstrumentedResponse, String> {
    let request = match builder.build() {
        Ok(r) => r,
        Err(e) => {
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
            let _ = app.emit("cdr-inspector-entry", &entry);
            return Err(message);
        }
    };

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
        .map(|b| String::from_utf8_lossy(b).to_string());

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
    let _ = app.emit("cdr-inspector-entry", &entry);

    let start = Instant::now();

    let fail = |entry: &mut RequestLogEntry, message: String| -> String {
        entry.duration_ms = start.elapsed().as_millis() as u64;
        entry.state = "failed";
        entry.error = Some(message.clone());
        let _ = app.emit("cdr-inspector-entry", &*entry);
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
    entry.response_body = Some(log_body);
    entry.duration_ms = duration_ms;
    entry.body_truncated = body_truncated;
    entry.state = "complete";
    let _ = app.emit("cdr-inspector-entry", &entry);

    Ok(InstrumentedResponse {
        status,
        is_success,
        headers: response_headers,
        body,
    })
}
