use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Semaphore;

use super::composition::CommitAudit;
use super::server::{create_client, get_profile_by_id, make_request, AuthMethod, ServerType};
use crate::inspector::{send_instrumented, InstrumentedResponse};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EhrSummary {
    pub ehr_id: String,
    pub system_id: Option<String>,
    pub time_created: Option<String>,
    pub subject_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EhrDetail {
    pub ehr_id: String,
    pub system_id: Option<String>,
    pub time_created: Option<String>,
    pub is_modifiable: Option<bool>,
    pub is_queryable: Option<bool>,
    pub subject_id: Option<String>,
    pub subject_namespace: Option<String>,
    pub compositions: Vec<CompositionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionSummary {
    pub uid: String,
    pub template_id: Option<String>,
    pub name: Option<String>,
    pub composer: Option<String>,
    pub time_committed: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EhrListResponse {
    pub ehrs: Vec<EhrSummary>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    /// False when the requested sort couldn't actually be applied — the CDR
    /// rejected `ORDER BY` on the sort field and `list_ehrs` fell back to an
    /// unsorted query (see `is_order_by_unsupported`). The frontend uses
    /// this to tell the user sorting isn't supported rather than implying
    /// the shown order matches what they asked for.
    pub sort_applied: bool,
}

/// Whitelisted EHR-list sort fields, mapped to their AQL path expressions.
/// A whitelist (rather than interpolating the caller-supplied field name
/// straight into the query) is what makes it safe to build the `ORDER BY`
/// clause via string formatting in `build_ehr_list_aql` below.
fn sort_field_path(field: &str) -> Option<&'static str> {
    match field {
        "time_created" => Some("e/time_created/value"),
        "ehr_id" => Some("e/ehr_id/value"),
        "system_id" => Some("e/system_id/value"),
        _ => None,
    }
}

/// Builds the AQL query used by `list_ehrs`, including an `ORDER BY` clause
/// for the given sort field/direction. Both default to `time_created DESC`
/// (newest first) when omitted, matching the app's historical default
/// ordering (see PRD-0001).
///
/// Always appends `e/ehr_id/value ASC` as a secondary sort key (unless
/// `ehr_id` is itself the primary field) so ties on the primary field — e.g.
/// several EHRs created in the same instant — get a total, stable order.
/// Without one, a CDR is free to return tied rows in a different relative
/// order across separate paginated queries, which could duplicate or skip
/// rows across page boundaries.
fn build_ehr_list_aql(
    offset: usize,
    limit: usize,
    sort_by: Option<&str>,
    sort_dir: Option<&str>,
) -> Result<String, String> {
    let field = sort_by.unwrap_or("time_created");
    let path =
        sort_field_path(field).ok_or_else(|| format!("Unsupported sort field: {}", field))?;

    let dir = match sort_dir.unwrap_or("desc").to_ascii_lowercase().as_str() {
        "asc" => "ASC",
        "desc" => "DESC",
        other => return Err(format!("Unsupported sort direction: {}", other)),
    };

    let tiebreaker = if field == "ehr_id" {
        String::new()
    } else {
        ", e/ehr_id/value ASC".to_string()
    };

    Ok(format!(
        "SELECT e/ehr_id/value, e/time_created/value, e/system_id/value FROM EHR e \
         ORDER BY {} {}{} LIMIT {} OFFSET {}",
        path, dir, tiebreaker, limit, offset
    ))
}

/// Same `SELECT`/`FROM`/`LIMIT`/`OFFSET` as `build_ehr_list_aql`, but with no
/// `ORDER BY` at all — the fallback used when the CDR rejects sorting on
/// EHR-level attributes (see `list_ehrs`).
fn build_ehr_list_aql_unsorted(offset: usize, limit: usize) -> String {
    format!(
        "SELECT e/ehr_id/value, e/time_created/value, e/system_id/value FROM EHR e LIMIT {} OFFSET {}",
        limit, offset
    )
}

/// Detects the specific "`ORDER BY` on an EHR-level attribute isn't
/// implemented" error some CDRs return for this query shape — confirmed
/// against a real EHRBase instance, which rejects `ORDER BY e/time_created/value`
/// the same way it's documented (in `build_ehr_search_aql` above) to reject
/// `WHERE` on EHR-level paths, even though the AQL spec allows both. Kept
/// narrow (requires `order_by` plus a not-implemented/not-supported phrase)
/// so it doesn't swallow unrelated 400s from `list_ehrs`'s query — those
/// still surface as a normal error.
fn is_order_by_unsupported(response_body: &str) -> bool {
    let lower = response_body.to_ascii_lowercase();
    lower.contains("order_by")
        && (lower.contains("not implemented") || lower.contains("not supported"))
}

#[tauri::command]
pub async fn list_ehrs(
    app: tauri::AppHandle,
    server_id: String,
    offset: usize,
    limit: usize,
    sort_by: Option<String>,
    sort_dir: Option<String>,
) -> Result<EhrListResponse, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');
    let url = format!("{}/rest/openehr/v1/query/aql", base);

    // Use AQL to list EHRs since the REST API list endpoint varies by implementation
    let aql = build_ehr_list_aql(offset, limit, sort_by.as_deref(), sort_dir.as_deref())?;
    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::POST, &url, &profile.auth_method)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "q": aql })),
    )
    .await?;

    // Some CDRs (confirmed: EHRBase) don't implement `ORDER BY` on EHR-level
    // attributes. Rather than surfacing a hard error and leaving the EHR
    // Browser unusable on those servers, retry once without the ORDER BY
    // clause; `sort_applied: false` in the response tells the frontend the
    // requested order wasn't actually honored, so it can say so instead of
    // silently showing unsorted results as if they were sorted.
    let (resp, sort_applied) = if !resp.is_success && is_order_by_unsupported(&resp.body) {
        let fallback_aql = build_ehr_list_aql_unsorted(offset, limit);
        let fallback_resp = send_instrumented(
            &app,
            &client,
            make_request(&client, reqwest::Method::POST, &url, &profile.auth_method)
                .header("Content-Type", "application/json")
                .json(&serde_json::json!({ "q": fallback_aql })),
        )
        .await?;
        (fallback_resp, false)
    } else {
        (resp, true)
    };

    if !resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    let body: Value =
        serde_json::from_str(&resp.body).map_err(|e| format!("Failed to parse response: {}", e))?;

    let rows = body
        .get("rows")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();

    let ehrs: Vec<EhrSummary> = rows
        .iter()
        .filter_map(|row| {
            let arr = row.as_array()?;
            Some(EhrSummary {
                ehr_id: arr.first()?.as_str()?.to_string(),
                time_created: arr.get(1).and_then(|v| v.as_str()).map(String::from),
                system_id: arr.get(2).and_then(|v| v.as_str()).map(String::from),
                subject_id: None,
            })
        })
        .collect();

    let total = ehrs.len() + offset; // Approximate — exact total requires separate count query

    Ok(EhrListResponse {
        total,
        offset,
        limit,
        ehrs,
        sort_applied,
    })
}

#[tauri::command]
pub async fn get_ehr_detail(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
) -> Result<EhrDetail, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    // Fetch EHR status
    let status_url = format!("{}/rest/openehr/v1/ehr/{}", base, ehr_id);
    let ehr_resp = send_instrumented(
        &app,
        &client,
        make_request(
            &client,
            reqwest::Method::GET,
            &status_url,
            &profile.auth_method,
        )
        .header("Accept", "application/json"),
    )
    .await?;

    let ehr_json: Value = if ehr_resp.is_success {
        serde_json::from_str(&ehr_resp.body).map_err(|e| format!("Failed to parse EHR: {}", e))?
    } else {
        Value::Null
    };

    let system_id = ehr_json
        .get("system_id")
        .and_then(|s| s.get("value"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let time_created = ehr_json
        .get("time_created")
        .and_then(|s| s.get("value"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let is_modifiable = ehr_json
        .get("ehr_status")
        .and_then(|s| s.get("is_modifiable"))
        .and_then(|v| v.as_bool());

    let is_queryable = ehr_json
        .get("ehr_status")
        .and_then(|s| s.get("is_queryable"))
        .and_then(|v| v.as_bool());

    // Extract subject identity from EHR status
    let subject_id = ehr_json
        .get("ehr_status")
        .and_then(|s| s.get("subject"))
        .and_then(|s| s.get("external_ref"))
        .and_then(|r| r.get("id"))
        .and_then(|id| id.get("value"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let subject_namespace = ehr_json
        .get("ehr_status")
        .and_then(|s| s.get("subject"))
        .and_then(|s| s.get("external_ref"))
        .and_then(|r| r.get("id"))
        .and_then(|id| id.get("scheme"))
        .and_then(|v| v.as_str())
        .map(String::from);

    // Fetch compositions via AQL
    let aql = format!(
        "SELECT c/uid/value, c/archetype_details/template_id/value, c/name/value, c/composer/name, c/context/start_time/value \
         FROM EHR e CONTAINS COMPOSITION c WHERE e/ehr_id/value = '{}' ORDER BY c/context/start_time/value DESC",
        ehr_id
    );

    let query_url = format!("{}/rest/openehr/v1/query/aql", base);
    let comp_resp = send_instrumented(
        &app,
        &client,
        make_request(
            &client,
            reqwest::Method::POST,
            &query_url,
            &profile.auth_method,
        )
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "q": aql })),
    )
    .await?;

    let compositions = if comp_resp.is_success {
        let body: Value = serde_json::from_str(&comp_resp.body)
            .map_err(|e| format!("Failed to parse compositions: {}", e))?;

        body.get("rows")
            .and_then(|r| r.as_array())
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| {
                        let arr = row.as_array()?;
                        Some(CompositionSummary {
                            uid: arr.first()?.as_str()?.to_string(),
                            template_id: arr.get(1).and_then(|v| v.as_str()).map(String::from),
                            name: arr.get(2).and_then(|v| v.as_str()).map(String::from),
                            composer: arr.get(3).and_then(|v| v.as_str()).map(String::from),
                            time_committed: arr.get(4).and_then(|v| v.as_str()).map(String::from),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    Ok(EhrDetail {
        ehr_id,
        system_id,
        time_created,
        is_modifiable,
        is_queryable,
        subject_id,
        subject_namespace,
        compositions,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEhrRequest {
    pub subject_namespace: Option<String>,
    pub subject_id: Option<String>,
    pub is_queryable: Option<bool>,
    pub is_modifiable: Option<bool>,
    pub ehr_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEhrResponse {
    pub ehr_id: String,
    pub system_id: Option<String>,
    pub time_created: Option<String>,
}

/// Builds the EHR_STATUS JSON object for an EHR creation request.
///
/// `archetype_details` is technically mandatory on any LOCATABLE that is an
/// archetype root (per the RM's Archetyped_valid invariant): EHR_STATUS sets
/// `archetype_node_id`, so it's a root and needs `archetype_details` with a
/// matching `archetype_id`. EHRBase fills this in server-side if it's
/// missing, but stricter CDRs (e.g. FerroEHR) reject the request with a 422
/// if it's absent, so we always supply it here.
fn build_ehr_status_json(request: &CreateEhrRequest) -> serde_json::Value {
    let mut ehr_status = serde_json::json!({
        "_type": "EHR_STATUS",
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": {
                "_type": "ARCHETYPE_ID",
                "value": "openEHR-EHR-EHR_STATUS.generic.v1"
            },
            "rm_version": "1.0.4"
        },
        "name": {
            "_type": "DV_TEXT",
            "value": "EHR Status"
        },
        "subject": {
            "_type": "PARTY_SELF"
        },
        "is_queryable": request.is_queryable.unwrap_or(true),
        "is_modifiable": request.is_modifiable.unwrap_or(true)
    });

    // Override subject if external identity is provided
    if let (Some(subject_id), Some(subject_namespace)) =
        (&request.subject_id, &request.subject_namespace)
    {
        ehr_status["subject"] = serde_json::json!({
            "_type": "PARTY_SELF",
            "external_ref": {
                "_type": "PARTY_REF",
                "id": {
                    "_type": "GENERIC_ID",
                    "value": subject_id,
                    "scheme": subject_namespace
                },
                "namespace": "external",
                "type": "PERSON"
            }
        });
    }

    ehr_status
}

#[tauri::command]
pub async fn create_ehr(
    app: tauri::AppHandle,
    server_id: String,
    request: CreateEhrRequest,
) -> Result<CreateEhrResponse, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let ehr_status = build_ehr_status_json(&request);

    // For EHR creation, EHRBase expects the EHR_STATUS object directly, not wrapped
    let request_body = if let Some(ehr_id) = &request.ehr_id {
        // If custom EHR ID is provided, we need to wrap it alongside ehr_status
        serde_json::json!({
            "ehr_id": {
                "_type": "HIER_OBJECT_ID",
                "value": ehr_id
            },
            "ehr_status": ehr_status
        })
    } else {
        // Otherwise, send EHR_STATUS directly as the root object
        ehr_status
    };

    let url = format!("{}/rest/openehr/v1/ehr", base);
    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::POST, &url, &profile.auth_method)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&request_body),
    )
    .await?;

    if !resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    // EHRBase returns 201 Created with Location header, but empty body
    // Extract EHR ID from Location header: /rest/openehr/v1/ehr/{ehr_id}
    let location = resp
        .headers
        .get("location")
        .ok_or("Location header not found in response")?;

    // Extract EHR ID from the location path
    let ehr_id = location
        .split('/')
        .next_back()
        .ok_or("Could not extract EHR ID from Location header")?
        .to_string();

    Ok(CreateEhrResponse {
        ehr_id,
        system_id: None,
        time_created: None,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEhrStatusRequest {
    pub is_queryable: bool,
    pub is_modifiable: bool,
    pub subject_namespace: Option<String>,
    pub subject_id: Option<String>,
}

#[tauri::command]
pub async fn update_ehr_status(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
    request: UpdateEhrStatusRequest,
) -> Result<String, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    // First, fetch current EHR status to get the version UID
    let get_url = format!("{}/rest/openehr/v1/ehr/{}/ehr_status", base, ehr_id);
    let get_resp = send_instrumented(
        &app,
        &client,
        make_request(
            &client,
            reqwest::Method::GET,
            &get_url,
            &profile.auth_method,
        )
        .header("Accept", "application/json"),
    )
    .await?;

    if !get_resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            get_resp.status, get_resp.body
        ));
    }

    let current_status: Value = serde_json::from_str(&get_resp.body)
        .map_err(|e| format!("Failed to parse current status: {}", e))?;

    let version_uid = current_status
        .get("uid")
        .and_then(|u| u.get("value"))
        .and_then(|v| v.as_str())
        .ok_or("Version UID not found in current status")?;

    // Build updated EHR status
    let mut updated_status = current_status.clone();
    updated_status["is_queryable"] = serde_json::json!(request.is_queryable);
    updated_status["is_modifiable"] = serde_json::json!(request.is_modifiable);

    // Update subject if provided
    if let (Some(subject_id), Some(subject_namespace)) =
        (request.subject_id, request.subject_namespace)
    {
        updated_status["subject"] = serde_json::json!({
            "_type": "PARTY_SELF",
            "external_ref": {
                "id": {
                    "_type": "GENERIC_ID",
                    "value": subject_id,
                    "scheme": subject_namespace
                },
                "namespace": "external"
            }
        });
    }

    // PUT request with If-Match header
    let put_url = format!("{}/rest/openehr/v1/ehr/{}/ehr_status", base, ehr_id);
    let put_resp = send_instrumented(
        &app,
        &client,
        make_request(
            &client,
            reqwest::Method::PUT,
            &put_url,
            &profile.auth_method,
        )
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("If-Match", version_uid)
        .json(&updated_status),
    )
    .await?;

    if !put_resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            put_resp.status, put_resp.body
        ));
    }

    Ok("EHR status updated successfully".to_string())
}

#[tauri::command]
pub async fn delete_ehr(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
) -> Result<String, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    // The standard openEHR REST API does not support DELETE on EHR.
    // EHRBase provides an admin API for EHR deletion.
    let (url, auth) = match profile.server_type {
        ServerType::Ehrbase => {
            let admin_auth = profile
                .admin_auth_method
                .as_ref()
                .unwrap_or(&profile.auth_method);
            (
                format!("{}/rest/admin/ehr/{}", base, ehr_id),
                admin_auth.clone(),
            )
        }
        // FerroEHR's admin API is nested under the openEHR REST API base
        // path, unlike EHRbase's sibling `/rest/admin/...` mount.
        ServerType::FerroEhr => {
            let admin_auth = profile
                .admin_auth_method
                .as_ref()
                .unwrap_or(&profile.auth_method);
            (
                format!("{}/rest/openehr/v1/admin/ehr/{}", base, ehr_id),
                admin_auth.clone(),
            )
        }
        _ => (
            format!("{}/rest/openehr/v1/ehr/{}", base, ehr_id),
            profile.auth_method.clone(),
        ),
    };
    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::DELETE, &url, &auth),
    )
    .await?;

    if !resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    Ok(format!("EHR {} deleted successfully", ehr_id))
}

// --- DIRECTORY (OEH-27) ---

/// Builds the URL for the openEHR `DIRECTORY` resource.
///
/// - `version_uid: None` → the latest version: `GET /ehr/{ehr_id}/directory`
///   (optionally combined with a `version_at_time` query param).
/// - `version_uid: Some(uid)` → a specific historical version:
///   `GET /ehr/{ehr_id}/directory/{version_uid}`.
fn build_directory_url(
    base: &str,
    ehr_id: &str,
    version_uid: Option<&str>,
    version_at_time: Option<&str>,
) -> String {
    let path = match version_uid {
        Some(uid) => format!("{}/rest/openehr/v1/ehr/{}/directory/{}", base, ehr_id, uid),
        None => format!("{}/rest/openehr/v1/ehr/{}/directory", base, ehr_id),
    };

    match version_at_time {
        Some(time) => match reqwest::Url::parse(&path) {
            Ok(mut url) => {
                url.query_pairs_mut().append_pair("version_at_time", time);
                url.to_string()
            }
            // Malformed base URL — fall back to the un-parameterized path;
            // the request will fail downstream with a clearer connection error.
            Err(_) => path,
        },
        None => path,
    }
}

/// Formats a non-2xx response as the `Err(String)` surfaced to the frontend.
fn http_error(resp: &InstrumentedResponse) -> String {
    format!("Server returned HTTP {}: {}", resp.status, resp.body)
}

/// Shared GET + response handling for both DIRECTORY commands below (they
/// differ only in how `url` is built) — sends the request, treats a 404 as
/// "no directory" (`Ok(None)`, since that's an expected, common state rather
/// than a failure), and parses the body into JSON on any other success.
///
/// The DIRECTORY's FOLDER/OBJECT_REF nesting is arbitrary-depth and
/// data-driven, so — like `composition::get_composition` — the response is
/// passed through as raw JSON rather than modeled with a Rust struct.
async fn fetch_directory(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    url: &str,
    auth: &AuthMethod,
) -> Result<Option<Value>, String> {
    let resp = send_instrumented(
        app,
        client,
        make_request(client, reqwest::Method::GET, url, auth).header("Accept", "application/json"),
    )
    .await?;

    if resp.status == 404 {
        return Ok(None);
    }

    if !resp.is_success {
        return Err(http_error(&resp));
    }

    serde_json::from_str(&resp.body)
        .map(Some)
        .map_err(|e| format!("Failed to parse directory: {}", e))
}

/// Fetches the latest (or, if `version_at_time` is given, the version in
/// effect at that instant) DIRECTORY folder hierarchy for an EHR.
#[tauri::command]
pub async fn get_directory(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
    version_at_time: Option<String>,
) -> Result<Option<Value>, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = build_directory_url(base, &ehr_id, None, version_at_time.as_deref());
    fetch_directory(&app, &client, &url, &profile.auth_method).await
}

/// Fetches a specific historical version of the DIRECTORY, identified by its
/// version UID (as returned in the `ETag`/`Location` of a prior DIRECTORY
/// write, or from `get_directory`'s response `uid`).
#[tauri::command]
pub async fn get_directory_version(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
    version_uid: String,
) -> Result<Option<Value>, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = build_directory_url(base, &ehr_id, Some(&version_uid), None);
    fetch_directory(&app, &client, &url, &profile.auth_method).await
}

/// One entry in the DIRECTORY's revision history — same shape as
/// `composition::CompositionVersion`, since both come from the same kind of
/// `VERSIONED_OBJECT` revision-history response (a list of ORIGINAL_VERSION
/// summaries, each with an `audits` array whose first entry is the commit
/// audit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryRevision {
    pub version_id: String,
    pub preceding_version_uid: Option<String>,
    pub commit_audit: Option<CommitAudit>,
    pub time_committed: Option<String>,
}

fn build_versioned_directory_revision_history_url(base: &str, ehr_id: &str) -> String {
    format!(
        "{}/rest/openehr/v1/ehr/{}/versioned_directory/revision_history",
        base, ehr_id
    )
}

/// Parses a `versioned_directory/revision_history` response body into
/// `DirectoryRevision`s — factored out of `get_directory_revision_history` so
/// it can be exercised directly with fixture JSON, without a live server.
fn parse_directory_revision_history(body: &Value) -> Vec<DirectoryRevision> {
    let items = body
        .as_array()
        .or_else(|| body.get("items").and_then(|i| i.as_array()))
        .cloned()
        .unwrap_or_default();

    items
        .iter()
        .map(|item| {
            let version_id = item
                .get("version_id")
                .and_then(|v| v.get("value"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let audits = item.get("audits").and_then(|a| a.as_array());
            let first_audit = audits.and_then(|a| a.first());

            let commit_audit = first_audit.map(|audit| CommitAudit {
                change_type: audit
                    .get("change_type")
                    .and_then(|c| c.get("value"))
                    .and_then(|v| v.as_str())
                    .map(String::from),
                committer_name: audit
                    .get("committer")
                    .and_then(|c| c.get("name"))
                    .and_then(|v| v.as_str())
                    .map(String::from),
                time_committed: audit
                    .get("time_committed")
                    .and_then(|t| t.get("value"))
                    .and_then(|v| v.as_str())
                    .map(String::from),
                description: audit
                    .get("description")
                    .and_then(|d| d.get("value"))
                    .and_then(|v| v.as_str())
                    .map(String::from),
            });

            let time_committed = commit_audit.as_ref().and_then(|a| a.time_committed.clone());

            DirectoryRevision {
                version_id,
                preceding_version_uid: item
                    .get("preceding_version_uid")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                commit_audit,
                time_committed,
            }
        })
        .collect()
}

/// Fetches the full revision history of the EHR's DIRECTORY — every version
/// ever committed, not just the current one — via the standard openEHR
/// `VERSIONED_DIRECTORY` resource:
/// `GET /ehr/{ehr_id}/versioned_directory/revision_history`.
///
/// Mirrors `composition::get_composition_versions`, which does the same
/// thing for `VERSIONED_COMPOSITION`. Each entry's `version_id` can be passed
/// straight to `get_directory_version` to fetch that version's content.
#[tauri::command]
pub async fn get_directory_revision_history(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
) -> Result<Vec<DirectoryRevision>, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = build_versioned_directory_revision_history_url(base, &ehr_id);

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::GET, &url, &profile.auth_method)
            .header("Accept", "application/json"),
    )
    .await?;

    if resp.status == 404 {
        // No DIRECTORY has ever been created for this EHR — an empty
        // history, not an error (matches fetch_directory's own treatment of
        // a 404 as "no directory" rather than a failure).
        return Ok(Vec::new());
    }

    if !resp.is_success {
        return Err(http_error(&resp));
    }

    let body: Value = serde_json::from_str(&resp.body)
        .map_err(|e| format!("Failed to parse directory revision history: {}", e))?;

    Ok(parse_directory_revision_history(&body))
}

/// Creates the DIRECTORY for an EHR that doesn't have one yet.
///
/// POSTs `folder` (a full FOLDER structure: `_type`, `name`, and optional
/// `items`/`folders`) to the DIRECTORY resource, then re-fetches it via GET
/// rather than trusting the POST response body — servers vary in whether
/// that body is present at all (some return 201 with an empty body and only
/// a `Location`/`ETag`), so re-fetching is the one path that always yields
/// the server's canonical stored representation (assigned `uid`, any
/// server-side normalization).
#[tauri::command]
pub async fn create_directory(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
    folder: Value,
) -> Result<Value, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');
    let url = build_directory_url(base, &ehr_id, None, None);

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::POST, &url, &profile.auth_method)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&folder),
    )
    .await?;

    if !resp.is_success {
        return Err(http_error(&resp));
    }

    fetch_directory(&app, &client, &url, &profile.auth_method)
        .await?
        .ok_or_else(|| "Directory was created but could not be re-fetched".to_string())
}

/// Replaces the DIRECTORY's FOLDER hierarchy in place.
///
/// `preceding_version_uid` must be the `uid.value` of the directory version
/// currently on screen (from a prior `get_directory`/`create_directory`
/// call) — sent as `If-Match` so the server rejects the write with a 409/412
/// if the DIRECTORY changed underneath the client since it was loaded,
/// rather than silently clobbering that change. See `update_ehr_status` for
/// the same optimistic-concurrency pattern.
#[tauri::command]
pub async fn update_directory(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
    folder: Value,
    preceding_version_uid: String,
) -> Result<Value, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');
    let url = build_directory_url(base, &ehr_id, None, None);

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::PUT, &url, &profile.auth_method)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("If-Match", preceding_version_uid)
            .json(&folder),
    )
    .await?;

    if !resp.is_success {
        return Err(http_error(&resp));
    }

    fetch_directory(&app, &client, &url, &profile.auth_method)
        .await?
        .ok_or_else(|| "Directory was updated but could not be re-fetched".to_string())
}

/// Builds the URL for deleting the DIRECTORY: the base DIRECTORY resource
/// URL plus a `version_uid` query parameter identifying the version being
/// deleted (the openEHR REST API's optimistic-concurrency guard for this
/// endpoint — unlike composition/EHR deletion, DIRECTORY has no per-version
/// path segment to address instead).
fn build_directory_delete_url(base: &str, ehr_id: &str, preceding_version_uid: &str) -> String {
    let directory_url = build_directory_url(base, ehr_id, None, None);

    match reqwest::Url::parse(&directory_url) {
        Ok(mut u) => {
            u.query_pairs_mut()
                .append_pair("version_uid", preceding_version_uid);
            u.to_string()
        }
        // Malformed base URL — fall back to manual interpolation; the
        // request will fail downstream with a clearer connection error.
        Err(_) => format!(
            "{}?version_uid={}",
            directory_url,
            urlencoding::encode(preceding_version_uid)
        ),
    }
}

/// Deletes the DIRECTORY entirely.
///
/// `preceding_version_uid` is the same optimistic-concurrency guard as
/// `update_directory`, but unlike composition/EHR deletion (which address
/// the resource by path segment) the openEHR REST API takes it as a
/// `version_uid` query parameter on DELETE — the DIRECTORY resource itself
/// has no per-version path.
#[tauri::command]
pub async fn delete_directory(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
    preceding_version_uid: String,
) -> Result<String, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');
    let url = build_directory_delete_url(base, &ehr_id, &preceding_version_uid);

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::DELETE, &url, &profile.auth_method),
    )
    .await?;

    if !resp.is_success {
        return Err(http_error(&resp));
    }

    Ok(format!("Directory for EHR {} deleted successfully", ehr_id))
}

// --- AQL-backed EHR search (PRD-0013) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EhrSearchCriteria {
    pub ehr_id_prefix: Option<String>,
    pub subject_id: Option<String>,
    pub subject_namespace: Option<String>,
    pub system_id: Option<String>,
    pub modifiable: Option<bool>,
    pub has_compositions: Option<bool>,
    pub created_on: Option<String>,     // YYYY-MM-DD
    pub created_before: Option<String>, // YYYY-MM-DD
    pub created_after: Option<String>,  // YYYY-MM-DD
    /// Whether the EHR has a DIRECTORY (FOLDER structure) set. Unlike the
    /// other criteria, this can't be expressed as an AQL predicate — the
    /// DIRECTORY is only reachable via its own REST resource — so it's
    /// applied as a post-filter in `search_ehrs` after the AQL results come
    /// back (see `filter_by_directory_presence`). Both `true` and `false`
    /// are supported, since the post-filter just checks presence either way.
    pub has_directory: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EhrSearchResult {
    pub ehr_id: String,
    pub time_created: Option<String>,
    pub subject_id: Option<String>,
    pub subject_namespace: Option<String>,
    pub is_modifiable: Option<bool>,
    pub is_queryable: Option<bool>,
    pub system_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EhrSearchResponse {
    pub results: Vec<EhrSearchResult>,
    pub total: usize,
    pub limit_reached: bool,
}

/// Escape a string value for safe interpolation into AQL single-quoted literals.
/// Doubles any embedded single quotes (' -> '').
fn escape_aql_string(s: &str) -> String {
    s.replace('\'', "''")
}

/// Build an AQL query string from the given search criteria.
/// Returns an error if no criteria are provided (to prevent full-table scans).
pub fn build_ehr_search_aql(criteria: &EhrSearchCriteria) -> Result<String, String> {
    // Determine if we need to include COMPOSITION in the FROM clause
    let has_compositions_filter = criteria.has_compositions == Some(true);

    let base = if has_compositions_filter {
        // When filtering for EHRs with compositions, we query compositions
        // and join with EHR_STATUS data. Use DISTINCT to avoid duplicates.
        // This uses the standard pattern: FROM EHR e [archetype_id] CONTAINS COMPOSITION c [archetype_id]
        "\
SELECT DISTINCT \
e/ehr_id/value, \
e/time_created/value, \
e/ehr_status/subject/external_ref/id/value AS subject_id, \
e/ehr_status/subject/external_ref/namespace AS subject_namespace, \
e/ehr_status/is_modifiable AS modifiable, \
e/ehr_status/is_queryable AS queryable, \
e/system_id/value AS system_id \
FROM EHR e CONTAINS COMPOSITION c"
    } else {
        // Standard query without composition requirement
        "\
SELECT \
e/ehr_id/value, \
e/time_created/value, \
s/subject/external_ref/id/value AS subject_id, \
s/subject/external_ref/namespace AS subject_namespace, \
s/is_modifiable AS modifiable, \
s/is_queryable AS queryable, \
e/system_id/value AS system_id \
FROM EHR e CONTAINS EHR_STATUS s"
    };

    let mut predicates: Vec<String> = Vec::new();

    // Determine the correct path prefix for EHR_STATUS fields
    // When filtering by compositions, we access ehr_status via the EHR object path
    let status_prefix = if has_compositions_filter {
        "e/ehr_status"
    } else {
        "s"
    };

    if let Some(ref prefix) = criteria.ehr_id_prefix {
        predicates.push(format!(
            "e/ehr_id/value LIKE '{}%'",
            escape_aql_string(prefix)
        ));
    }

    if let Some(ref subject_id) = criteria.subject_id {
        predicates.push(format!(
            "{}/subject/external_ref/id/value LIKE '%{}%'",
            status_prefix,
            escape_aql_string(subject_id)
        ));
    }

    if let Some(ref ns) = criteria.subject_namespace {
        predicates.push(format!(
            "{}/subject/external_ref/namespace = '{}'",
            status_prefix,
            escape_aql_string(ns)
        ));
    }

    if let Some(ref sys) = criteria.system_id {
        predicates.push(format!("e/system_id/value = '{}'", escape_aql_string(sys)));
    }

    if let Some(modifiable) = criteria.modifiable {
        predicates.push(format!("{}/is_modifiable = {}", status_prefix, modifiable));
    }

    if let Some(has_comp) = criteria.has_compositions {
        if !has_comp {
            // For has_compositions:false, AQL doesn't provide a clean way to express "no compositions"
            // The best approach is to use a workaround: count compositions and filter where count = 0
            // However, this requires a complex subquery that may not be supported by all CDRs
            // For now, we'll document this as unsupported and suggest using the inverse search
            return Err(
                "hasCompositions:false is not currently supported due to AQL limitations. \
                 To find EHRs with compositions, use hasCompositions:true instead."
                    .to_string(),
            );
        }
        // For has_compositions:true, the FROM clause already includes COMPOSITION c,
        // so we don't need an additional predicate
    }

    // Date handling: created_on takes precedence over created_before/created_after
    // NOTE: EHRBase does not support filtering on e/time_created in WHERE clauses
    // This is a known AQL limitation - EHR-level attributes cannot be used in predicates
    if criteria.created_on.is_some()
        || criteria.created_before.is_some()
        || criteria.created_after.is_some()
    {
        return Err(
            "Date filters (created-on, created-before, created-after) are not currently supported \
             due to EHRBase limitations. EHR-level attributes like time_created cannot be used in \
             WHERE clauses. Use the paginated list view and sort by creation date instead."
                .to_string(),
        );
    }

    // Check if any criteria was provided (either predicates, has_compositions:true,
    // or has_directory — the latter never adds a predicate since it's applied as a
    // post-filter in `search_ehrs`, but it's still a real criterion the user asked for).
    if predicates.is_empty() && !has_compositions_filter && criteria.has_directory.is_none() {
        return Err("At least one search criterion must be provided".to_string());
    }

    // Build the final query
    if predicates.is_empty() {
        // Only has_compositions:true, no WHERE clause needed
        Ok(format!("{} LIMIT 200", base))
    } else {
        let where_clause = predicates.join(" AND ");
        Ok(format!("{} WHERE {} LIMIT 200", base, where_clause))
    }
}

/// Filters `results` down to only the EHRs whose DIRECTORY presence matches
/// `want` (`true` keeps EHRs that have one, `false` keeps EHRs that don't).
///
/// AQL has no path for this — the DIRECTORY is only reachable via its own
/// `GET /ehr/{id}/directory` resource, not a queryable attribute — so this
/// issues one directory fetch per candidate EHR instead, bounded to a modest
/// concurrency so a 200-row result set doesn't fire 200 requests at once.
/// Original ordering is preserved even though the requests complete out of
/// order, so results don't visibly reshuffle between identical searches.
///
/// A fetch failure (auth, transport, a non-404 HTTP error, bad JSON, ...) is
/// *not* treated as "no directory" — that would silently misclassify an EHR
/// we simply couldn't check. Only a confirmed 404 (which `fetch_directory`
/// itself maps to `Ok(None)`) counts as absence; anything else drops that
/// EHR from the result entirely, matching this function's existing
/// best-effort handling of a panicked task below.
async fn filter_by_directory_presence(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    base: &str,
    auth: &AuthMethod,
    results: Vec<EhrSearchResult>,
    want: bool,
) -> Vec<EhrSearchResult> {
    const CONCURRENCY: usize = 8;
    let semaphore = Arc::new(Semaphore::new(CONCURRENCY));
    let mut set = tokio::task::JoinSet::new();

    for (index, result) in results.into_iter().enumerate() {
        let semaphore = semaphore.clone();
        let app = app.clone();
        let client = client.clone();
        let auth = auth.clone();
        let url = build_directory_url(base, &result.ehr_id, None, None);
        set.spawn(async move {
            // Permit is held for the duration of the request; dropped (and the
            // slot freed) when this task completes.
            let _permit = semaphore.acquire_owned().await;
            let directory_result = fetch_directory(&app, &client, &url, &auth).await;
            (index, result, directory_result)
        });
    }

    let mut kept: Vec<(usize, EhrSearchResult)> = Vec::new();
    while let Some(joined) = set.join_next().await {
        // Only a *confirmed* answer counts: Ok(Some(_)) means the EHR has a
        // directory, Ok(None) means fetch_directory saw a real 404 (its own
        // documented way of reporting "no directory set" — not an error).
        // An Err (auth failure, transport error, a non-404 HTTP error, bad
        // JSON, ...) means we genuinely don't know, so that EHR is left out
        // of the filtered list entirely rather than being silently
        // miscounted as "no directory" either way. Likewise, a panicked
        // task is dropped rather than surfaced — its EHR is simply left out
        // of the (already best-effort) filtered list.
        if let Ok((index, result, Ok(directory))) = joined {
            if directory.is_some() == want {
                kept.push((index, result));
            }
        }
    }
    kept.sort_by_key(|(index, _)| *index);
    kept.into_iter().map(|(_, result)| result).collect()
}

#[tauri::command]
pub async fn search_ehrs(
    app: tauri::AppHandle,
    server_id: String,
    criteria: EhrSearchCriteria,
) -> Result<EhrSearchResponse, String> {
    let aql = build_ehr_search_aql(&criteria)?;

    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = format!("{}/rest/openehr/v1/query/aql", base);
    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::POST, &url, &profile.auth_method)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "q": aql })),
    )
    .await?;

    if !resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    let body: Value =
        serde_json::from_str(&resp.body).map_err(|e| format!("Failed to parse response: {}", e))?;

    let rows = body
        .get("rows")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();

    let results: Vec<EhrSearchResult> = rows
        .iter()
        .filter_map(|row| {
            let arr = row.as_array()?;
            Some(EhrSearchResult {
                ehr_id: arr.first()?.as_str()?.to_string(),
                time_created: arr.get(1).and_then(|v| v.as_str()).map(String::from),
                subject_id: arr.get(2).and_then(|v| v.as_str()).map(String::from),
                subject_namespace: arr.get(3).and_then(|v| v.as_str()).map(String::from),
                is_modifiable: arr.get(4).and_then(|v| v.as_bool()),
                is_queryable: arr.get(5).and_then(|v| v.as_bool()),
                system_id: arr.get(6).and_then(|v| v.as_str()).map(String::from),
            })
        })
        .collect();

    // `limit_reached` reflects the raw AQL result set (capped at 200 rows by
    // the query's own LIMIT) — computed before the directory post-filter so
    // "showing first 200, refine your search" still means what it says even
    // when has_directory then narrows the displayed count further.
    let limit_reached = results.len() >= 200;

    let results = if let Some(want_directory) = criteria.has_directory {
        filter_by_directory_presence(
            &app,
            &client,
            base,
            &profile.auth_method,
            results,
            want_directory,
        )
        .await
    } else {
        results
    };

    let total = results.len();

    Ok(EhrSearchResponse {
        results,
        total,
        limit_reached,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_ehr_list_aql_defaults_to_time_created_desc() {
        let aql = build_ehr_list_aql(0, 20, None, None).unwrap();
        assert!(aql.contains("ORDER BY e/time_created/value DESC"));
        assert!(aql.contains("LIMIT 20 OFFSET 0"));
    }

    #[test]
    fn test_build_ehr_list_aql_asc() {
        let aql = build_ehr_list_aql(40, 20, Some("time_created"), Some("asc")).unwrap();
        assert!(aql.contains("ORDER BY e/time_created/value ASC"));
        assert!(aql.contains("LIMIT 20 OFFSET 40"));
    }

    #[test]
    fn test_build_ehr_list_aql_direction_is_case_insensitive() {
        let aql = build_ehr_list_aql(0, 20, Some("ehr_id"), Some("ASC")).unwrap();
        assert!(aql.contains("ORDER BY e/ehr_id/value ASC"));
    }

    #[test]
    fn test_build_ehr_list_aql_sort_by_ehr_id() {
        let aql = build_ehr_list_aql(0, 20, Some("ehr_id"), Some("desc")).unwrap();
        assert!(aql.contains("ORDER BY e/ehr_id/value DESC"));
    }

    #[test]
    fn test_build_ehr_list_aql_sort_by_system_id() {
        let aql = build_ehr_list_aql(0, 20, Some("system_id"), Some("asc")).unwrap();
        assert!(aql.contains("ORDER BY e/system_id/value ASC"));
    }

    #[test]
    fn test_build_ehr_list_aql_appends_ehr_id_tiebreaker_for_other_fields() {
        // A secondary sort key on the unique ehr_id gives ties on the
        // primary field (e.g. several EHRs created in the same instant) a
        // stable, total order across separate paginated queries.
        let aql = build_ehr_list_aql(0, 20, Some("time_created"), Some("desc")).unwrap();
        assert!(aql.contains("ORDER BY e/time_created/value DESC, e/ehr_id/value ASC LIMIT"));

        let aql = build_ehr_list_aql(0, 20, Some("system_id"), Some("asc")).unwrap();
        assert!(aql.contains("ORDER BY e/system_id/value ASC, e/ehr_id/value ASC LIMIT"));
    }

    #[test]
    fn test_build_ehr_list_aql_no_duplicate_tiebreaker_when_sorting_by_ehr_id() {
        // ehr_id is already the (unique) primary key here, so appending it
        // again as a tiebreaker would be redundant.
        let aql = build_ehr_list_aql(0, 20, Some("ehr_id"), Some("asc")).unwrap();
        assert!(aql.contains("ORDER BY e/ehr_id/value ASC LIMIT"));
        assert_eq!(aql.matches("e/ehr_id/value").count(), 2); // SELECT column + ORDER BY, no third occurrence
    }

    #[test]
    fn test_build_ehr_list_aql_unsorted_has_no_order_by() {
        let aql = build_ehr_list_aql_unsorted(40, 20);
        assert!(!aql.to_ascii_uppercase().contains("ORDER BY"));
        assert!(aql.contains("LIMIT 20 OFFSET 40"));
        assert!(aql.contains("SELECT e/ehr_id/value, e/time_created/value, e/system_id/value"));
    }

    #[test]
    fn test_is_order_by_unsupported_matches_confirmed_ehrbase_error() {
        // Actual error body observed from a real EHRBase instance.
        let body = r#"{"error":"Bad Request","message":"Not implemented: ORDER_BY: identified path 'time_created/value' for type EHR not supported"}"#;
        assert!(is_order_by_unsupported(body));
    }

    #[test]
    fn test_is_order_by_unsupported_ignores_unrelated_errors() {
        assert!(!is_order_by_unsupported(
            r#"{"error":"Unauthorized","message":"Invalid credentials"}"#
        ));
        assert!(!is_order_by_unsupported(
            r#"{"error":"Bad Request","message":"malformed AQL query"}"#
        ));
    }

    #[test]
    fn test_build_ehr_list_aql_rejects_unknown_field() {
        let result = build_ehr_list_aql(0, 20, Some("subject_id"), Some("asc"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported sort field"));
    }

    #[test]
    fn test_build_ehr_list_aql_rejects_unknown_direction() {
        let result = build_ehr_list_aql(0, 20, Some("time_created"), Some("sideways"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported sort direction"));
    }

    fn empty_criteria() -> EhrSearchCriteria {
        EhrSearchCriteria {
            ehr_id_prefix: None,
            subject_id: None,
            subject_namespace: None,
            system_id: None,
            modifiable: None,
            has_compositions: None,
            created_on: None,
            created_before: None,
            created_after: None,
            has_directory: None,
        }
    }

    #[test]
    fn test_empty_criteria_returns_error() {
        let result = build_ehr_search_aql(&empty_criteria());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("At least one search criterion"));
    }

    #[test]
    fn test_ehr_id_prefix() {
        let mut c = empty_criteria();
        c.ehr_id_prefix = Some("fde80e0e".to_string());
        let aql = build_ehr_search_aql(&c).unwrap();
        assert!(aql.contains("e/ehr_id/value LIKE 'fde80e0e%'"));
        assert!(aql.contains("LIMIT 200"));
    }

    #[test]
    fn test_subject_id_contains() {
        let mut c = empty_criteria();
        c.subject_id = Some("6f4b5848".to_string());
        let aql = build_ehr_search_aql(&c).unwrap();
        assert!(aql.contains("s/subject/external_ref/id/value LIKE '%6f4b5848%'"));
    }

    #[test]
    fn test_subject_namespace_exact() {
        let mut c = empty_criteria();
        c.subject_namespace = Some("patnr".to_string());
        let aql = build_ehr_search_aql(&c).unwrap();
        assert!(aql.contains("s/subject/external_ref/namespace = 'patnr'"));
    }

    #[test]
    fn test_system_id_exact() {
        let mut c = empty_criteria();
        c.system_id = Some("dev.cistec.io".to_string());
        let aql = build_ehr_search_aql(&c).unwrap();
        assert!(aql.contains("e/system_id/value = 'dev.cistec.io'"));
    }

    #[test]
    fn test_modifiable_true() {
        let mut c = empty_criteria();
        c.modifiable = Some(true);
        let aql = build_ehr_search_aql(&c).unwrap();
        assert!(aql.contains("s/is_modifiable = true"));
    }

    #[test]
    fn test_modifiable_false() {
        let mut c = empty_criteria();
        c.modifiable = Some(false);
        let aql = build_ehr_search_aql(&c).unwrap();
        assert!(aql.contains("s/is_modifiable = false"));
    }

    #[test]
    fn test_has_compositions_true() {
        let mut c = empty_criteria();
        c.has_compositions = Some(true);
        let aql = build_ehr_search_aql(&c).unwrap();
        // When searching for EHRs with compositions, COMPOSITION is included in FROM clause
        // EHR_STATUS is accessed via path notation (e/ehr_status/...)
        assert!(aql.contains("FROM EHR e CONTAINS COMPOSITION c"));
        assert!(aql.contains("SELECT DISTINCT"));
        assert!(aql.contains("e/ehr_status/"));
    }

    #[test]
    fn test_has_compositions_false() {
        let mut c = empty_criteria();
        c.has_compositions = Some(false);
        let result = build_ehr_search_aql(&c);
        // hasCompositions:false is not currently supported due to AQL limitations
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not currently supported"));
    }

    #[test]
    fn test_created_on() {
        let mut c = empty_criteria();
        c.created_on = Some("2026-03-12".to_string());
        let result = build_ehr_search_aql(&c);
        // Date filters are not supported due to EHRBase limitations
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not currently supported"));
    }

    #[test]
    fn test_created_before() {
        let mut c = empty_criteria();
        c.created_before = Some("2026-03-12".to_string());
        let result = build_ehr_search_aql(&c);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not currently supported"));
    }

    #[test]
    fn test_created_after() {
        let mut c = empty_criteria();
        c.created_after = Some("2026-03-12".to_string());
        let result = build_ehr_search_aql(&c);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not currently supported"));
    }

    #[test]
    fn test_created_before_and_after_range() {
        let mut c = empty_criteria();
        c.created_after = Some("2026-03-01".to_string());
        c.created_before = Some("2026-03-31".to_string());
        let result = build_ehr_search_aql(&c);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not currently supported"));
    }

    #[test]
    fn test_created_on_overrides_before_after() {
        let mut c = empty_criteria();
        c.created_on = Some("2026-03-15".to_string());
        c.created_before = Some("2026-03-31".to_string());
        c.created_after = Some("2026-03-01".to_string());
        let result = build_ehr_search_aql(&c);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not currently supported"));
    }

    #[test]
    fn test_has_directory_true_alone_is_a_valid_criterion() {
        // has_directory can't become an AQL predicate (it's applied as a
        // post-filter in search_ehrs), but it must still count as "a
        // criterion was provided" so the query isn't rejected.
        let mut c = empty_criteria();
        c.has_directory = Some(true);
        let aql = build_ehr_search_aql(&c).unwrap();
        assert!(aql.contains("FROM EHR e CONTAINS EHR_STATUS s"));
        assert!(!aql.contains("WHERE"));
        assert!(aql.contains("LIMIT 200"));
    }

    #[test]
    fn test_has_directory_false_alone_is_also_valid() {
        // Unlike has_compositions:false, has_directory:false is supported —
        // the post-filter checks presence either way — so it shouldn't error.
        let mut c = empty_criteria();
        c.has_directory = Some(false);
        let aql = build_ehr_search_aql(&c).unwrap();
        assert!(!aql.contains("WHERE"));
    }

    #[test]
    fn test_has_directory_combines_with_other_predicates() {
        let mut c = empty_criteria();
        c.has_directory = Some(true);
        c.subject_namespace = Some("patnr".to_string());
        let aql = build_ehr_search_aql(&c).unwrap();
        assert!(aql.contains("s/subject/external_ref/namespace = 'patnr'"));
        // has_directory itself never appears as a predicate.
        assert!(!aql.to_lowercase().contains("directory"));
    }

    #[test]
    fn test_has_directory_combines_with_has_compositions() {
        let mut c = empty_criteria();
        c.has_directory = Some(true);
        c.has_compositions = Some(true);
        let aql = build_ehr_search_aql(&c).unwrap();
        assert!(aql.contains("FROM EHR e CONTAINS COMPOSITION c"));
    }

    #[test]
    fn test_multi_criteria_and() {
        let mut c = empty_criteria();
        c.subject_namespace = Some("patnr".to_string());
        c.modifiable = Some(true);
        let aql = build_ehr_search_aql(&c).unwrap();
        assert!(aql.contains("s/subject/external_ref/namespace = 'patnr'"));
        assert!(aql.contains("s/is_modifiable = true"));
        assert!(aql.contains(" AND "));
    }

    #[test]
    fn test_escape_single_quotes() {
        let mut c = empty_criteria();
        c.subject_id = Some("O'Brien".to_string());
        let aql = build_ehr_search_aql(&c).unwrap();
        assert!(aql.contains("O''Brien"));
        assert!(!aql.contains("O'B")); // unescaped single quote should not appear
    }

    #[test]
    fn test_invalid_date_format() {
        let mut c = empty_criteria();
        c.created_on = Some("not-a-date".to_string());
        let result = build_ehr_search_aql(&c);
        // Date filters are not supported, so we get that error instead of validation error
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not currently supported"));
    }

    #[test]
    fn test_build_directory_url_latest() {
        let url = build_directory_url("https://cdr.example.com", "ehr-123", None, None);
        assert_eq!(
            url,
            "https://cdr.example.com/rest/openehr/v1/ehr/ehr-123/directory"
        );
    }

    #[test]
    fn test_build_directory_url_specific_version() {
        let url = build_directory_url(
            "https://cdr.example.com",
            "ehr-123",
            Some("uid::system::1"),
            None,
        );
        assert_eq!(
            url,
            "https://cdr.example.com/rest/openehr/v1/ehr/ehr-123/directory/uid::system::1"
        );
    }

    #[test]
    fn test_build_directory_delete_url() {
        let url =
            build_directory_delete_url("https://cdr.example.com", "ehr-123", "uid::system::1");
        assert_eq!(
            url,
            "https://cdr.example.com/rest/openehr/v1/ehr/ehr-123/directory?version_uid=uid%3A%3Asystem%3A%3A1"
        );
    }

    #[test]
    fn test_build_directory_url_at_time() {
        let url = build_directory_url(
            "https://cdr.example.com",
            "ehr-123",
            None,
            Some("2026-08-25T12:00:00Z"),
        );
        assert_eq!(
            url,
            "https://cdr.example.com/rest/openehr/v1/ehr/ehr-123/directory?version_at_time=2026-08-25T12%3A00%3A00Z"
        );
    }

    #[test]
    fn test_aql_has_correct_select_columns() {
        let mut c = empty_criteria();
        c.ehr_id_prefix = Some("abc".to_string());
        let aql = build_ehr_search_aql(&c).unwrap();
        assert!(aql.contains("e/ehr_id/value"));
        assert!(aql.contains("e/time_created/value"));
        assert!(aql.contains("s/subject/external_ref/id/value AS subject_id"));
        assert!(aql.contains("s/subject/external_ref/namespace AS subject_namespace"));
        assert!(aql.contains("s/is_modifiable AS modifiable"));
        assert!(aql.contains("s/is_queryable AS queryable"));
        assert!(aql.contains("e/system_id/value AS system_id"));
        assert!(aql.contains("FROM EHR e CONTAINS EHR_STATUS s"));
    }

    fn empty_create_ehr_request() -> CreateEhrRequest {
        CreateEhrRequest {
            subject_namespace: None,
            subject_id: None,
            is_queryable: None,
            is_modifiable: None,
            ehr_id: None,
        }
    }

    #[test]
    fn test_ehr_status_includes_archetype_details() {
        // Stricter CDRs (e.g. FerroEHR) reject an EHR_STATUS root without
        // archetype_details, so it must always be present.
        let status = build_ehr_status_json(&empty_create_ehr_request());
        assert_eq!(
            status["archetype_details"]["archetype_id"]["value"],
            "openEHR-EHR-EHR_STATUS.generic.v1"
        );
        assert_eq!(status["archetype_details"]["_type"], "ARCHETYPED");
        assert!(status["archetype_details"]["rm_version"].is_string());
    }

    #[test]
    fn test_ehr_status_defaults_queryable_and_modifiable_true() {
        let status = build_ehr_status_json(&empty_create_ehr_request());
        assert_eq!(status["is_queryable"], true);
        assert_eq!(status["is_modifiable"], true);
        assert_eq!(status["subject"]["_type"], "PARTY_SELF");
        assert!(status["subject"].get("external_ref").is_none());
    }

    #[test]
    fn test_ehr_status_respects_explicit_flags() {
        let mut req = empty_create_ehr_request();
        req.is_queryable = Some(false);
        req.is_modifiable = Some(false);
        let status = build_ehr_status_json(&req);
        assert_eq!(status["is_queryable"], false);
        assert_eq!(status["is_modifiable"], false);
    }

    #[test]
    fn test_ehr_status_with_subject_identity() {
        let mut req = empty_create_ehr_request();
        req.subject_id = Some("Testtest".to_string());
        req.subject_namespace = Some("ch.ahv".to_string());
        let status = build_ehr_status_json(&req);
        let external_ref = &status["subject"]["external_ref"];
        assert_eq!(external_ref["id"]["value"], "Testtest");
        assert_eq!(external_ref["id"]["scheme"], "ch.ahv");
    }

    #[test]
    fn test_build_versioned_directory_revision_history_url() {
        let url =
            build_versioned_directory_revision_history_url("https://cdr.example.com", "ehr-123");
        assert_eq!(
            url,
            "https://cdr.example.com/rest/openehr/v1/ehr/ehr-123/versioned_directory/revision_history"
        );
    }

    #[test]
    fn test_parse_directory_revision_history_extracts_commit_audit() {
        let body = serde_json::json!([
            {
                "version_id": { "value": "01a058ec-c19e-7823-b3f8-9fbb5e0e47f9::ferroehr.local::2" },
                "preceding_version_uid": "01a058ec-c19a-7bc2-9988-9ad84f965f3b::ferroehr.local::1",
                "audits": [
                    {
                        "change_type": { "value": "modification" },
                        "committer": { "name": "ferroehr" },
                        "time_committed": { "value": "2026-08-31T17:45:47.628264Z" },
                        "description": { "value": "renamed folder" }
                    }
                ]
            }
        ]);
        let revisions = parse_directory_revision_history(&body);
        assert_eq!(revisions.len(), 1);
        let rev = &revisions[0];
        assert_eq!(
            rev.version_id,
            "01a058ec-c19e-7823-b3f8-9fbb5e0e47f9::ferroehr.local::2"
        );
        assert_eq!(
            rev.preceding_version_uid.as_deref(),
            Some("01a058ec-c19a-7bc2-9988-9ad84f965f3b::ferroehr.local::1")
        );
        let audit = rev.commit_audit.as_ref().expect("commit_audit");
        assert_eq!(audit.change_type.as_deref(), Some("modification"));
        assert_eq!(audit.committer_name.as_deref(), Some("ferroehr"));
        assert_eq!(
            audit.time_committed.as_deref(),
            Some("2026-08-31T17:45:47.628264Z")
        );
        assert_eq!(
            rev.time_committed.as_deref(),
            Some("2026-08-31T17:45:47.628264Z")
        );
    }

    #[test]
    fn test_parse_directory_revision_history_handles_items_wrapper_and_missing_audits() {
        let body = serde_json::json!({
            "items": [
                { "version_id": { "value": "uid::system::1" } }
            ]
        });
        let revisions = parse_directory_revision_history(&body);
        assert_eq!(revisions.len(), 1);
        assert_eq!(revisions[0].version_id, "uid::system::1");
        assert!(revisions[0].commit_audit.is_none());
        assert!(revisions[0].time_committed.is_none());
    }

    #[test]
    fn test_parse_directory_revision_history_empty_array() {
        let revisions = parse_directory_revision_history(&serde_json::json!([]));
        assert!(revisions.is_empty());
    }
}
