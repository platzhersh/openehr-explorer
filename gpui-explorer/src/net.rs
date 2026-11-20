//! REST client for openEHR CDRs. Mirrors what `src-tauri/src/commands/{ehr,template,query}.rs` send:
//! EHR listing via AQL, templates via `/definition/template/adl1.4`, Web Templates, and AQL execution.

use reqwest::Method;
use serde_json::{json, Value};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Shared Tokio runtime. GPUI has its own executor; futures from here can be awaited from GPUI tasks
/// because `JoinHandle` polling needs no runtime context.
pub fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("oeh-net")
            .enable_all()
            .build()
            .expect("tokio runtime")
    })
}

#[derive(Clone)]
pub enum Auth {
    None,
    Basic { user: String, password: String },
    Bearer(String),
}

#[derive(Clone)]
pub struct Profile {
    pub base_url: String,
    server_type: crate::profiles::ServerType,
    api_prefix: String,
    auth: Auth,
    client: reqwest::Client,
}

#[derive(Clone, Debug)]
pub struct EhrRow {
    pub id: String,
    pub created: String,
    pub system_id: String,
}

pub struct EhrPage {
    pub rows: Vec<EhrRow>,
    pub has_more: bool,
    pub sort_applied: bool,
}

#[derive(Clone, Debug)]
pub struct CompositionRow {
    pub uid: String,
    pub name: String,
    pub template: String,
    pub composer: String,
    pub committed: String,
}

#[derive(Clone, Debug)]
pub struct TemplateRow {
    pub id: String,
    pub concept: String,
    pub archetype: String,
}

#[derive(Clone, Debug)]
pub struct TreeRow {
    pub depth: u8,
    pub label: String,
    pub rm_type: String,
    pub path: String,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Counts {
    pub ehrs: u64,
    pub compositions: u64,
    pub templates: u64,
}

pub struct AqlOut {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub server_ms: Option<f64>,
}

impl Profile {
    pub fn new(base_url: &str, api_prefix: Option<String>, auth: Auth) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .user_agent("openEHR-Explorer-gpui-poc")
            .build()
            .expect("http client");
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            // `--server` has no profile; Generic and EHRBase share the same FLAT media type.
            server_type: crate::profiles::ServerType::Generic,
            api_prefix: api_prefix.unwrap_or_else(|| "/rest/openehr/v1".to_string()),
            auth,
            client,
        }
    }

    /// The serialized server type name (shown in the dashboard's server card).
    pub fn type_name(&self) -> &'static str {
        self.server_type.as_str()
    }

    /// Build a request profile from a stored/resolved server profile (secrets already resolved).
    pub fn from_server(p: &crate::profiles::ServerProfile) -> Self {
        use crate::profiles::AuthMethod;
        let auth = match &p.auth_method {
            AuthMethod::None => Auth::None,
            AuthMethod::Basic { username, password } => Auth::Basic { user: username.clone(), password: password.clone() },
            AuthMethod::Bearer { token } => Auth::Bearer(token.clone()),
        };
        let mut profile = Self::new(&p.base_url, p.api_path_prefix.clone(), auth);
        profile.server_type = p.server_type;
        profile
    }

    /// The same check as the Tauri app's "Test Connection": an authenticated GET of the template list; any 2xx passes.
    pub async fn test_http(&self) -> Result<String, String> {
        let url = format!("{}/definition/template/adl1.4", self.api_root());
        let resp = self.req(Method::GET, &url).header("Accept", "application/json").send().await.map_err(|e| format!("Request failed: {e}"))?;
        let status = resp.status();
        if status.is_success() {
            Ok(format!("Connected successfully (HTTP {})", status.as_u16()))
        } else {
            Err(format!("Server returned HTTP {}", status.as_u16()))
        }
    }

    /// Same rule as `ServerProfile::api_root()`: the prefix is appended to `base_url`; "" means base_url is the root.
    fn api_root(&self) -> String {
        format!("{}{}", self.base_url, self.api_prefix.trim_end_matches('/'))
    }

    fn req(&self, method: Method, url: &str) -> reqwest::RequestBuilder {
        let r = self.client.request(method, url);
        match &self.auth {
            Auth::None => r,
            Auth::Basic { user, password } => r.basic_auth(user, Some(password)),
            Auth::Bearer(t) => r.bearer_auth(t),
        }
    }

    async fn send_json(&self, rb: reqwest::RequestBuilder) -> Result<Value, String> {
        let resp = rb.send().await.map_err(|e| format!("Request failed: {e}"))?;
        let status = resp.status();
        let body = resp.text().await.map_err(|e| format!("Failed to read response: {e}"))?;
        if !status.is_success() {
            let snippet: String = body.chars().take(300).collect();
            return Err(format!("Server returned HTTP {}: {}", status.as_u16(), snippet));
        }
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse response: {e}"))
    }

    /// Returns a short human-readable server description.
    pub async fn test_connection(&self) -> Result<String, String> {
        // EHRBase exposes /rest/status outside the openEHR root; other CDRs may not.
        let status_url = format!("{}/rest/status", self.base_url);
        // EHRBase answers /rest/status in XML unless JSON is requested explicitly.
        if let Ok(v) = self.send_json(self.req(Method::GET, &status_url).header("Accept", "application/json")).await {
            if let Some(ver) = v.get("ehrbase_version").and_then(|x| x.as_str()) {
                return Ok(format!("EHRBase {ver}"));
            }
        }
        // Generic fallback: any authenticated, successful template listing proves reachability + credentials.
        let url = format!("{}/definition/template/adl1.4", self.api_root());
        self.send_json(self.req(Method::GET, &url).header("Accept", "application/json")).await?;
        Ok("openEHR CDR".to_string())
    }

    pub async fn aql(&self, query: &str) -> Result<AqlOut, String> {
        let url = format!("{}/query/aql", self.api_root());
        let t = Instant::now();
        let body = self
            .send_json(self.req(Method::POST, &url).header("Content-Type", "application/json").header("Accept", "application/json").json(&json!({ "q": query })))
            .await?;
        let columns = body
            .get("columns")
            .and_then(|c| c.as_array())
            .map(|a| {
                a.iter()
                    .map(|c| {
                        c.get("name")
                            .and_then(|n| n.as_str())
                            .or_else(|| c.get("path").and_then(|n| n.as_str()))
                            .unwrap_or("?")
                            .to_string()
                    })
                    .collect()
            })
            .unwrap_or_default();
        let rows = body
            .get("rows")
            .and_then(|r| r.as_array())
            .map(|a| a.iter().map(|row| row.as_array().map(|cells| cells.iter().map(cell_to_string).collect()).unwrap_or_default()).collect())
            .unwrap_or_default();
        Ok(AqlOut { columns, rows, server_ms: Some(t.elapsed().as_secs_f64() * 1000.0) })
    }

    /// One page of EHRs. Requests `limit + 1` rows to learn whether another page exists, and retries without
    /// ORDER BY if the CDR rejects it (EHRBase does), exactly like the real backend.
    pub async fn list_ehrs(&self, offset: usize, limit: usize) -> Result<EhrPage, String> {
        let fetch = limit + 1;
        let sorted = format!(
            "SELECT e/ehr_id/value, e/time_created/value, e/system_id/value FROM EHR e \
             ORDER BY e/time_created/value DESC, e/ehr_id/value ASC LIMIT {fetch} OFFSET {offset}"
        );
        let (out, sort_applied) = match self.aql(&sorted).await {
            Ok(o) => (o, true),
            Err(e) if e.to_ascii_lowercase().contains("order_by") || e.to_ascii_lowercase().contains("order by") => {
                let unsorted = format!("SELECT e/ehr_id/value, e/time_created/value, e/system_id/value FROM EHR e LIMIT {fetch} OFFSET {offset}");
                (self.aql(&unsorted).await?, false)
            }
            Err(e) => return Err(e),
        };
        let mut rows: Vec<EhrRow> = out
            .rows
            .into_iter()
            .filter(|r| r.len() >= 3)
            .map(|r| EhrRow { id: r[0].clone(), created: r[1].clone(), system_id: r[2].clone() })
            .collect();
        let has_more = rows.len() > limit;
        rows.truncate(limit);
        Ok(EhrPage { rows, has_more, sort_applied })
    }

    pub async fn ehr_compositions(&self, ehr_id: &str) -> Result<Vec<CompositionRow>, String> {
        // The id is interpolated into AQL, so only accept UUID characters.
        if ehr_id.is_empty() || !ehr_id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
            return Err("Refusing to query: EHR id is not a UUID".to_string());
        }
        let q = format!(
            "SELECT c/uid/value, c/name/value, c/archetype_details/template_id/value, c/composer/name, c/context/start_time/value \
             FROM EHR e[ehr_id/value='{ehr_id}'] CONTAINS COMPOSITION c ORDER BY c/context/start_time/value DESC LIMIT 100"
        );
        let out = self.aql(&q).await?;
        Ok(out
            .rows
            .into_iter()
            .filter(|r| r.len() >= 5)
            .map(|r| CompositionRow { uid: r[0].clone(), name: r[1].clone(), template: r[2].clone(), composer: r[3].clone(), committed: r[4].clone() })
            .collect())
    }

    /// Same three numbers as the Tauri dashboard: COUNT on concrete leaf paths (EHRBase rejects COUNT on whole
    /// RM objects) plus the template list length.
    pub async fn dashboard_counts(&self) -> Result<Counts, String> {
        let ehrs = self.aql_count("SELECT COUNT(e/ehr_id/value) FROM EHR e").await?;
        let compositions = self.aql_count("SELECT COUNT(c/uid/value) FROM EHR e CONTAINS COMPOSITION c").await?;
        let templates = self.list_templates().await?.len() as u64;
        Ok(Counts { ehrs, compositions, templates })
    }

    async fn aql_count(&self, q: &str) -> Result<u64, String> {
        let out = self.aql(q).await?;
        let cell = out.rows.first().and_then(|r| r.first()).ok_or_else(|| format!("COUNT query returned no rows: {q}"))?;
        parse_count(cell).ok_or_else(|| format!("COUNT query returned a non-numeric value: {cell}"))
    }

    /// Canonical JSON composition.
    pub async fn composition(&self, ehr_id: &str, uid: &str) -> Result<Value, String> {
        let url = self.composition_url(ehr_id, uid)?;
        self.send_json(self.req(Method::GET, &url).header("Accept", "application/json")).await
    }

    /// FLAT composition; the media type depends on the server (same mapping as the Tauri backend).
    pub async fn composition_flat(&self, ehr_id: &str, uid: &str) -> Result<Value, String> {
        use crate::profiles::ServerType;
        let accept = match self.server_type {
            ServerType::FerroEhr | ServerType::BetterPlatform => "application/openehr.wt.flat+json",
            ServerType::Ehrbase | ServerType::Generic => "application/openehr.wt.flat.schema+json",
        };
        let url = self.composition_url(ehr_id, uid)?;
        self.send_json(self.req(Method::GET, &url).header("Accept", accept)).await
    }

    /// Ids end up in the URL path, so only accept what composition UIDs/EHR ids can contain.
    fn composition_url(&self, ehr_id: &str, uid: &str) -> Result<String, String> {
        let ok = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'));
        if !ok(ehr_id) || !ok(uid) {
            return Err("Refusing to request: EHR id or composition uid has unexpected characters".to_string());
        }
        Ok(format!("{}/ehr/{}/composition/{}", self.api_root(), ehr_id, uid))
    }

    pub async fn list_templates(&self) -> Result<Vec<TemplateRow>, String> {
        let url = format!("{}/definition/template/adl1.4", self.api_root());
        let v = self.send_json(self.req(Method::GET, &url).header("Accept", "application/json")).await?;
        let s = |item: &Value, k: &str| item.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        Ok(v.as_array()
            .map(|a| a.iter().map(|i| TemplateRow { id: s(i, "template_id"), concept: s(i, "concept"), archetype: s(i, "archetype_id") }).collect())
            .unwrap_or_default())
    }

    /// Flattened Web Template tree (depth-first), capped to keep the PoC responsive on huge templates.
    pub async fn web_template(&self, template_id: &str) -> Result<Vec<TreeRow>, String> {
        let url = format!("{}/definition/template/adl1.4/{}", self.api_root(), urlencode(template_id));
        let v = self.send_json(self.req(Method::GET, &url).header("Accept", "application/openehr.wt+json")).await?;
        let root = v.get("tree").unwrap_or(&v);
        let mut out = Vec::new();
        flatten(root, 0, &mut out);
        Ok(out)
    }
}

fn flatten(node: &Value, depth: u8, out: &mut Vec<TreeRow>) {
    const MAX_ROWS: usize = 5000;
    if out.len() >= MAX_ROWS || depth > 12 {
        return;
    }
    let s = |k: &str| node.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let name = if s("name").is_empty() { s("id") } else { s("name") };
    out.push(TreeRow { depth, label: name, rm_type: s("rmType"), path: s("aqlPath") });
    if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
        for c in children {
            flatten(c, depth + 1, out);
        }
    }
}

/// COUNT() comes back as an integer, a float (3.0) or a numeric string depending on the CDR.
fn parse_count(v: &str) -> Option<u64> {
    v.parse::<u64>().ok().or_else(|| v.parse::<f64>().ok().filter(|f| *f >= 0.0).map(|f| f as u64))
}

fn cell_to_string(v: &Value) -> String {
    match v {
        Value::Null => "—".to_string(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        other => {
            let s = other.to_string();
            if s.chars().count() > 60 {
                format!("{}…", s.chars().take(60).collect::<String>())
            } else {
                s
            }
        }
    }
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_parsing_matches_tauri() {
        assert_eq!(parse_count("42"), Some(42));
        assert_eq!(parse_count("3.0"), Some(3));
        assert_eq!(parse_count("17"), Some(17));
        assert_eq!(parse_count("not a number"), None);
        assert_eq!(parse_count("—"), None);
    }

    #[test]
    fn composition_url_rejects_path_tricks() {
        let p = Profile::new("http://h", None, Auth::None);
        assert!(p.composition_url("9c2c5c5e-1c1b", "c1a2::local.ehrbase.org::1").is_ok());
        assert!(p.composition_url("../x", "u").is_err());
        assert!(p.composition_url("e", "u?x=1").is_err());
        assert!(p.composition_url("e", "a/b").is_err());
    }
}
