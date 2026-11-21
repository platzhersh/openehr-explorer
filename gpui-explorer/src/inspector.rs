//! Request Inspector model: a log of every HTTP request the client sends, ported from the Tauri app
//! (`src-tauri/src/inspector.rs` + `src/stores/inspector.ts`, ADR-0011). Pure data + helpers; the UI lives in main.rs.
//! `net::Profile` reports each request through [`emit`]; the UI drains the channel installed by [`install`].

use serde_json::Value;
use std::sync::OnceLock;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub const MAX_BODY_SIZE: usize = 2 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 500;
const SENSITIVE_HEADERS: &[&str] = &["authorization", "cookie", "set-cookie"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Pending,
    Complete,
    Failed,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub id: u64,
    pub timestamp_ms: u64,
    pub method: String,
    pub url: String,
    /// Sorted by name for stable display. Sensitive values are already redacted.
    pub request_headers: Vec<(String, String)>,
    pub request_body: Option<String>,
    pub status: u16,
    pub response_headers: Vec<(String, String)>,
    pub response_body: Option<String>,
    pub duration_ms: u64,
    pub body_truncated: bool,
    pub state: State,
    pub error: Option<String>,
}

impl LogEntry {
    /// A copy without the (possibly megabytes large) bodies, for rendering metadata.
    pub fn without_bodies(&self) -> LogEntry {
        LogEntry { request_body: None, response_body: None, ..self.clone() }
    }
}

pub fn redact_headers(headers: impl IntoIterator<Item = (String, String)>) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = headers
        .into_iter()
        .map(|(k, val)| if SENSITIVE_HEADERS.contains(&k.to_lowercase().as_str()) { (k, "[REDACTED]".to_string()) } else { (k, val) })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

/// The copy of a response body kept for the log: capped at 2 MB (on a char boundary), flag says whether it was cut.
pub fn log_body(body: &str) -> (String, bool) {
    if body.len() <= MAX_BODY_SIZE {
        return (body.to_string(), false);
    }
    let mut end = MAX_BODY_SIZE;
    while !body.is_char_boundary(end) {
        end -= 1;
    }
    (body[..end].to_string(), true)
}

// ---- global sink -----------------------------------------------------------------------------------

static SINK: OnceLock<UnboundedSender<LogEntry>> = OnceLock::new();

/// Install the sink (once) and return the receiving end for the UI. Later calls return `None`.
pub fn install() -> Option<UnboundedReceiver<LogEntry>> {
    let (tx, rx) = unbounded_channel();
    SINK.set(tx).ok().map(|_| rx)
}

pub fn emit(entry: &LogEntry) {
    if let Some(tx) = SINK.get() {
        let _ = tx.send(entry.clone());
    }
}

pub fn next_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    N.fetch_add(1, Ordering::Relaxed)
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

// ---- the log ---------------------------------------------------------------------------------------

#[derive(Default)]
pub struct Filter {
    pub text: String,
    pub methods: Vec<String>,
    /// "2xx", "3xx", "4xx", "5xx", "pending", "failed"
    pub classes: Vec<String>,
}

#[derive(Default)]
pub struct Log {
    /// Newest first.
    pub entries: Vec<LogEntry>,
    pub selected: Option<u64>,
}

impl Log {
    /// The backend reports the same id twice (pending, then final): update in place; a late "pending"
    /// never overwrites a final state. New entries go on top and are auto-selected.
    pub fn upsert(&mut self, e: LogEntry) {
        if let Some(i) = self.entries.iter().position(|x| x.id == e.id) {
            if e.state == State::Pending && self.entries[i].state != State::Pending {
                return;
            }
            self.entries[i] = e;
            return;
        }
        self.selected = Some(e.id);
        self.entries.insert(0, e);
        self.entries.truncate(MAX_ENTRIES);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.selected = None;
    }

    pub fn selected_entry(&self) -> Option<&LogEntry> {
        let id = self.selected?;
        self.entries.iter().find(|e| e.id == id)
    }

    /// True when the newest request failed or returned >= 400 (drives the red dot on the collapsed header).
    pub fn has_errors(&self) -> bool {
        self.entries.first().is_some_and(|e| e.state == State::Failed || e.status >= 400)
    }

    pub fn filtered(&self, f: &Filter) -> Vec<usize> {
        let needle = f.text.to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                (f.methods.is_empty() || f.methods.contains(&e.method))
                    && (f.classes.is_empty() || f.classes.contains(&filter_class(e)))
                    && (needle.is_empty() || extract_path(&e.url).to_lowercase().contains(&needle))
            })
            .map(|(i, _)| i)
            .collect()
    }
}

pub fn filter_class(e: &LogEntry) -> String {
    match e.state {
        State::Pending => "pending".to_string(),
        State::Failed => "failed".to_string(),
        State::Complete => format!("{}xx", e.status / 100),
    }
}

// ---- formatting helpers ----------------------------------------------------------------------------

/// Path part of a URL (no scheme/host/query); falls back to the input.
pub fn extract_path(url: &str) -> String {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    match rest.find('/') {
        Some(i) => rest[i..].split(['?', '#']).next().unwrap_or("").to_string(),
        None => url.to_string(),
    }
}

pub fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        409 => "Conflict",
        412 => "Precondition Failed",
        415 => "Unsupported Media Type",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "",
    }
}

pub fn status_label(e: &LogEntry) -> String {
    match e.state {
        State::Pending => "Pending".to_string(),
        State::Failed => "Failed".to_string(),
        State::Complete => format!("{} {}", e.status, status_text(e.status)).trim().to_string(),
    }
}

/// `curl` command reproducing the request (note: the Authorization header is the redacted placeholder, as in Tauri).
pub fn generate_curl(e: &LogEntry) -> String {
    let mut parts = vec![format!("curl -X {}", e.method), format!("'{}'", e.url)];
    for (k, v) in &e.request_headers {
        parts.push(format!("-H '{k}: {v}'"));
    }
    if let Some(b) = e.request_body.as_deref().filter(|b| !b.is_empty()) {
        parts.push(format!("-d '{}'", b.replace('\'', "'\\''")));
    }
    parts.join(" \\\n  ")
}

/// `2026-08-20T09:15:00.123Z` from epoch milliseconds (civil-from-days, no time crate needed).
pub fn iso8601(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z", rem / 3600, rem % 3600 / 60, rem % 60, ms % 1000)
}

pub fn format_size(bytes: usize) -> String {
    match bytes {
        b if b < 1024 => format!("{b} B"),
        b if b < 1024 * 1024 => format!("{:.1} KB", b as f64 / 1024.0),
        b => format!("{:.1} MB", b as f64 / 1024.0 / 1024.0),
    }
}

pub fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
}

/// Body as display lines: pretty-printed when it parses as JSON (and `pretty`), otherwise the raw text.
pub fn body_lines(body: &str, pretty: bool) -> Vec<String> {
    if pretty {
        if let Ok(v) = serde_json::from_str::<Value>(body) {
            if let Ok(s) = serde_json::to_string_pretty(&v) {
                return s.lines().map(str::to_string).collect();
            }
        }
    }
    body.lines().map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u64, method: &str, url: &str, status: u16, state: State) -> LogEntry {
        LogEntry { id, timestamp_ms: 0, method: method.into(), url: url.into(), request_headers: vec![], request_body: None, status, response_headers: vec![], response_body: None, duration_ms: 0, body_truncated: false, state, error: None }
    }

    #[test]
    fn redacts_sensitive_headers_case_insensitively() {
        let h = redact_headers([("Authorization".to_string(), "Basic abc".to_string()), ("accept".to_string(), "json".to_string()), ("Set-Cookie".to_string(), "x".to_string())]);
        assert_eq!(h, vec![("Authorization".into(), "[REDACTED]".into()), ("Set-Cookie".into(), "[REDACTED]".into()), ("accept".into(), "json".into())]);
    }

    #[test]
    fn body_cap_respects_char_boundaries() {
        let body = "é".repeat(MAX_BODY_SIZE); // 2 bytes each: cut must not split a char
        let (kept, truncated) = log_body(&body);
        assert!(truncated);
        assert!(kept.len() <= MAX_BODY_SIZE);
        assert!(kept.chars().all(|c| c == 'é'));
        assert_eq!(log_body("small"), ("small".to_string(), false));
    }

    #[test]
    fn upsert_updates_in_place_and_ignores_late_pending() {
        let mut log = Log::default();
        log.upsert(entry(1, "GET", "http://h/a", 0, State::Pending));
        log.upsert(entry(2, "GET", "http://h/b", 0, State::Pending));
        assert_eq!(log.entries.len(), 2);
        assert_eq!(log.selected, Some(2));
        log.upsert(entry(1, "GET", "http://h/a", 200, State::Complete));
        assert_eq!(log.entries.len(), 2);
        assert_eq!(log.entries[1].status, 200);
        log.upsert(entry(1, "GET", "http://h/a", 0, State::Pending));
        assert_eq!(log.entries[1].state, State::Complete);
        assert_eq!(log.selected, Some(2), "updates must not steal the selection");
    }

    #[test]
    fn log_is_capped_newest_first() {
        let mut log = Log::default();
        for i in 0..(MAX_ENTRIES as u64 + 20) {
            log.upsert(entry(i, "GET", "http://h/x", 200, State::Complete));
        }
        assert_eq!(log.entries.len(), MAX_ENTRIES);
        assert_eq!(log.entries[0].id, MAX_ENTRIES as u64 + 19);
    }

    #[test]
    fn filters_by_text_method_and_class() {
        let mut log = Log::default();
        log.upsert(entry(1, "GET", "http://h/rest/openehr/v1/ehr/abc?x=1", 200, State::Complete));
        log.upsert(entry(2, "POST", "http://h/rest/openehr/v1/query/aql", 400, State::Complete));
        log.upsert(entry(3, "GET", "http://h/rest/status", 0, State::Failed));
        let f = |text: &str, m: &[&str], c: &[&str]| Filter { text: text.into(), methods: m.iter().map(|s| s.to_string()).collect(), classes: c.iter().map(|s| s.to_string()).collect() };
        assert_eq!(log.filtered(&f("", &[], &[])).len(), 3);
        assert_eq!(log.filtered(&f("AQL", &[], &[])), vec![1]);
        assert_eq!(log.filtered(&f("", &["GET"], &[])), vec![0, 2]);
        assert_eq!(log.filtered(&f("", &[], &["4xx", "failed"])), vec![0, 1]);
        assert_eq!(log.filtered(&f("abc", &[], &[])).len(), 1, "query string is not searched but path is");
        assert!(log.has_errors());
    }

    #[test]
    fn path_extraction() {
        assert_eq!(extract_path("https://h:8080/rest/x?y=1#z"), "/rest/x");
        assert_eq!(extract_path("http://h"), "http://h");
        assert_eq!(extract_path("not a url"), "not a url");
    }

    #[test]
    fn curl_escapes_single_quotes() {
        let mut e = entry(1, "POST", "http://h/q", 0, State::Pending);
        e.request_headers = vec![("accept".into(), "application/json".into())];
        e.request_body = Some("{\"q\":\"it's\"}".into());
        assert_eq!(generate_curl(&e), "curl -X POST \\\n  'http://h/q' \\\n  -H 'accept: application/json' \\\n  -d '{\"q\":\"it'\\''s\"}'");
    }

    #[test]
    fn iso_timestamps() {
        assert_eq!(iso8601(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso8601(1_787_217_300_123), "2026-08-20T09:15:00.123Z");
        assert_eq!(iso8601(951_782_400_000), "2000-02-29T00:00:00.000Z");
    }

    #[test]
    fn labels_and_sizes() {
        assert_eq!(status_label(&entry(1, "GET", "u", 404, State::Complete)), "404 Not Found");
        assert_eq!(status_label(&entry(1, "GET", "u", 299, State::Complete)), "299");
        assert_eq!(status_label(&entry(1, "GET", "u", 0, State::Failed)), "Failed");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1536), "1.5 KB");
    }

    #[test]
    fn body_pretty_or_raw() {
        assert_eq!(body_lines("{\"a\":1}", true), vec!["{", "  \"a\": 1", "}"]);
        assert_eq!(body_lines("{\"a\":1}", false), vec!["{\"a\":1}"]);
        assert_eq!(body_lines("<xml>\nx</xml>", true), vec!["<xml>", "x</xml>"]);
    }
}
