use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

use super::server::{create_client, get_profile_by_id, make_request};
use crate::inspector::send_instrumented;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AqlResult {
    pub columns: Vec<AqlColumn>,
    pub rows: Vec<Vec<Value>>,
    pub total_count: usize,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AqlColumn {
    pub name: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredQuerySummary {
    pub qualified_query_name: String,
    pub version: Option<String>,
    #[serde(rename = "type")]
    pub query_type: Option<String>,
    pub saved_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredQueryDefinition {
    pub qualified_query_name: String,
    pub version: Option<String>,
    #[serde(rename = "type")]
    pub query_type: Option<String>,
    pub q: Option<String>,
    pub saved_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedQuery {
    pub id: String,
    pub name: String,
    pub query: String,
    pub server_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryStore {
    queries: Vec<SavedQuery>,
}

fn get_queries_path() -> PathBuf {
    let config_dir = if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(dir)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        PathBuf::from(".config")
    };
    let dir = config_dir.join("openehr-explorer");
    fs::create_dir_all(&dir).ok();
    dir.join("saved_queries.json")
}

fn load_queries() -> Vec<SavedQuery> {
    let path = get_queries_path();
    if path.exists() {
        let data = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str::<QueryStore>(&data)
            .map(|s| s.queries)
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn save_queries_to_file(queries: &[SavedQuery]) -> Result<(), String> {
    let path = get_queries_path();
    let store = QueryStore {
        queries: queries.to_vec(),
    };
    let data = serde_json::to_string_pretty(&store).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn execute_aql(
    app: tauri::AppHandle,
    server_id: String,
    query: String,
) -> Result<AqlResult, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');
    let url = format!("{}/rest/openehr/v1/query/aql", base);

    let start = std::time::Instant::now();

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::POST, &url, &profile.auth_method)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&serde_json::json!({ "q": query })),
    )
    .await?;

    let elapsed = start.elapsed().as_millis() as u64;

    if !resp.is_success {
        return Err(format!("AQL error (HTTP {}): {}", resp.status, resp.body));
    }

    parse_aql_response(&resp.body, elapsed)
}

/// Shared parser for the openEHR Query API result envelope (`{ columns, rows }`),
/// used by both ad-hoc AQL execution and stored query execution.
fn parse_aql_response(body_str: &str, elapsed_ms: u64) -> Result<AqlResult, String> {
    let body: Value = serde_json::from_str(body_str)
        .map_err(|e| format!("Failed to parse query response: {}", e))?;

    let columns: Vec<AqlColumn> = body
        .get("columns")
        .and_then(|c| c.as_array())
        .map(|cols| {
            cols.iter()
                .map(|col| AqlColumn {
                    name: col
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("?")
                        .to_string(),
                    path: col.get("path").and_then(|p| p.as_str()).map(String::from),
                })
                .collect()
        })
        .unwrap_or_default();

    let rows: Vec<Vec<Value>> = body
        .get("rows")
        .and_then(|r| r.as_array())
        .map(|rs| {
            rs.iter()
                .map(|row| row.as_array().cloned().unwrap_or_else(|| vec![row.clone()]))
                .collect()
        })
        .unwrap_or_default();

    let total_count = rows.len();

    Ok(AqlResult {
        columns,
        rows,
        total_count,
        execution_time_ms: elapsed_ms,
    })
}

fn extract_query_name(item: &Value, fallback: &str) -> String {
    item.get("query_name")
        .or_else(|| item.get("name"))
        .or_else(|| item.get("qualified_query_name"))
        .and_then(|v| v.as_str())
        .unwrap_or(fallback)
        .to_string()
}

/// The `version` / `type` / `saved_time` fields are shared by both the
/// stored-query list and detail responses — extracted once here so the
/// field names aren't repeated at every call site.
struct StoredQueryFields {
    version: Option<String>,
    query_type: Option<String>,
    saved_time: Option<String>,
}

fn extract_stored_query_fields(item: &Value) -> StoredQueryFields {
    StoredQueryFields {
        version: item
            .get("version")
            .and_then(|v| v.as_str())
            .map(String::from),
        query_type: item.get("type").and_then(|v| v.as_str()).map(String::from),
        saved_time: item
            .get("saved_time")
            .and_then(|v| v.as_str())
            .map(String::from),
    }
}

fn server_http_error(status: u16, body: &str) -> String {
    format!("Server returned HTTP {}: {}", status, body)
}

/// List the STORED_QUERY definitions registered on the connected CDR
/// (`GET /definition/query`). Distinct from `list_saved_queries`, which are
/// queries persisted locally per server profile.
#[tauri::command]
pub async fn list_stored_queries(
    app: tauri::AppHandle,
    server_id: String,
) -> Result<Vec<StoredQuerySummary>, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');
    let url = format!("{}/rest/openehr/v1/definition/query", base);

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::GET, &url, &profile.auth_method)
            .header("Accept", "application/json"),
    )
    .await?;

    if !resp.is_success {
        return Err(server_http_error(resp.status, &resp.body));
    }

    let body: Value = serde_json::from_str(&resp.body)
        .map_err(|e| format!("Failed to parse stored query list: {}", e))?;

    let items: Vec<Value> = match &body {
        Value::Array(arr) => arr.clone(),
        Value::Object(obj) => obj
            .get("queries")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    };

    let queries = items
        .iter()
        .map(|item| {
            let fields = extract_stored_query_fields(item);
            StoredQuerySummary {
                qualified_query_name: extract_query_name(item, "?"),
                version: fields.version,
                query_type: fields.query_type,
                saved_time: fields.saved_time,
            }
        })
        .collect();

    Ok(queries)
}

/// Fetch a single stored query's definition (AQL text, type, version) via
/// `GET /definition/query/{qualified_query_name}[/{version}]`.
#[tauri::command]
pub async fn get_stored_query_definition(
    app: tauri::AppHandle,
    server_id: String,
    qualified_query_name: String,
    version: Option<String>,
) -> Result<StoredQueryDefinition, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let mut url = format!(
        "{}/rest/openehr/v1/definition/query/{}",
        base,
        urlencoding::encode(&qualified_query_name)
    );
    if let Some(v) = version.as_deref().filter(|v| !v.is_empty()) {
        url.push('/');
        url.push_str(&urlencoding::encode(v));
    }

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::GET, &url, &profile.auth_method)
            .header("Accept", "application/json"),
    )
    .await?;

    if !resp.is_success {
        return Err(server_http_error(resp.status, &resp.body));
    }

    let body: Value = serde_json::from_str(&resp.body)
        .map_err(|e| format!("Failed to parse stored query definition: {}", e))?;

    // Some servers return a single definition object; others return a list of
    // versions — take the first (typically the latest) entry in that case.
    let item = match &body {
        Value::Array(arr) => arr.first().cloned().unwrap_or(Value::Null),
        other => other.clone(),
    };

    let fields = extract_stored_query_fields(&item);

    Ok(StoredQueryDefinition {
        qualified_query_name: extract_query_name(&item, &qualified_query_name),
        version: fields.version.or(version),
        query_type: fields.query_type,
        q: item.get("q").and_then(|v| v.as_str()).map(String::from),
        saved_time: fields.saved_time,
    })
}

/// Execute a server-side STORED_QUERY with optional parameters
/// (`POST /query/{qualified_query_name}[/{version}]`). Returns the same
/// `AqlResult` shape as `execute_aql` so the frontend can reuse the existing
/// results table / CSV export UI.
#[tauri::command]
pub async fn execute_stored_query(
    app: tauri::AppHandle,
    server_id: String,
    qualified_query_name: String,
    version: Option<String>,
    parameters: Option<std::collections::HashMap<String, Value>>,
) -> Result<AqlResult, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let mut url = format!(
        "{}/rest/openehr/v1/query/{}",
        base,
        urlencoding::encode(&qualified_query_name)
    );
    if let Some(v) = version.as_deref().filter(|v| !v.is_empty()) {
        url.push('/');
        url.push_str(&urlencoding::encode(v));
    }

    let start = std::time::Instant::now();

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::POST, &url, &profile.auth_method)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&serde_json::json!({
                "query_parameters": parameters.unwrap_or_default(),
            })),
    )
    .await?;

    let elapsed = start.elapsed().as_millis() as u64;

    if !resp.is_success {
        return Err(format!(
            "Stored query error (HTTP {}): {}",
            resp.status, resp.body
        ));
    }

    parse_aql_response(&resp.body, elapsed)
}

#[tauri::command]
pub async fn list_saved_queries(server_id: Option<String>) -> Result<Vec<SavedQuery>, String> {
    let queries = load_queries();
    if let Some(sid) = server_id {
        Ok(queries
            .into_iter()
            .filter(|q| q.server_id.as_deref() == Some(&sid) || q.server_id.is_none())
            .collect())
    } else {
        Ok(queries)
    }
}

#[tauri::command]
pub async fn save_query(query: SavedQuery) -> Result<Vec<SavedQuery>, String> {
    let mut queries = load_queries();
    if let Some(existing) = queries.iter_mut().find(|q| q.id == query.id) {
        *existing = query;
    } else {
        queries.push(query);
    }
    save_queries_to_file(&queries)?;
    Ok(queries)
}

#[tauri::command]
pub async fn delete_saved_query(id: String) -> Result<Vec<SavedQuery>, String> {
    let mut queries = load_queries();
    queries.retain(|q| q.id != id);
    save_queries_to_file(&queries)?;
    Ok(queries)
}
