use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::server::{create_client, get_profile_by_id, make_request, ServerType};
use crate::inspector::send_instrumented;

/// The FLAT-format media type to use for a composition, as either a
/// `Content-Type` (POST/PUT) or an `Accept` (GET) header.
///
/// The openEHR REST API spec's current media type for this is
/// `application/openehr.wt.flat+json` — and that's exactly what FerroEHR's
/// own 415/406 error messages name as accepted. EHRBase's actual deployed
/// REST endpoint (verified directly against sandbox.ehrbase.org), however,
/// only recognizes the older/draft `application/openehr.wt.flat.schema+json`
/// variant: sending the spec-current media type gets rejected before it
/// even reaches EHRBase's own validation logic (a generic framework-level
/// 415), while the `.schema` variant is parsed and validated as expected.
/// Better Platform and unspecified/generic servers keep the same `.schema`
/// value this app has always sent them (unconfirmed either way, but
/// unbroken until reported).
fn flat_composition_content_type(server_type: &ServerType) -> &'static str {
    match server_type {
        ServerType::FerroEhr => "application/openehr.wt.flat+json",
        ServerType::Ehrbase | ServerType::BetterPlatform | ServerType::Generic => {
            "application/openehr.wt.flat.schema+json"
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionVersion {
    pub version_id: String,
    pub preceding_version_uid: Option<String>,
    pub lifecycle_state: Option<String>,
    pub commit_audit: Option<CommitAudit>,
    pub time_committed: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitAudit {
    pub change_type: Option<String>,
    pub committer_name: Option<String>,
    pub time_committed: Option<String>,
    pub description: Option<String>,
}

#[tauri::command]
pub async fn get_composition(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
    composition_uid: String,
) -> Result<Value, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = format!(
        "{}/rest/openehr/v1/ehr/{}/composition/{}",
        base, ehr_id, composition_uid
    );

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::GET, &url, &profile.auth_method)
            .header("Accept", "application/json"),
    )
    .await?;

    if !resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    serde_json::from_str(&resp.body).map_err(|e| format!("Failed to parse composition: {}", e))
}

#[tauri::command]
pub async fn get_composition_flat(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
    composition_uid: String,
) -> Result<Value, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = format!(
        "{}/rest/openehr/v1/ehr/{}/composition/{}",
        base, ehr_id, composition_uid
    );

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::GET, &url, &profile.auth_method).header(
            "Accept",
            flat_composition_content_type(&profile.server_type),
        ),
    )
    .await?;

    if !resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    serde_json::from_str(&resp.body).map_err(|e| format!("Failed to parse FLAT composition: {}", e))
}

#[tauri::command]
pub async fn get_composition_versions(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
    versioned_object_uid: String,
) -> Result<Vec<CompositionVersion>, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = format!(
        "{}/rest/openehr/v1/ehr/{}/versioned_composition/{}/revision_history",
        base, ehr_id, versioned_object_uid
    );

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::GET, &url, &profile.auth_method)
            .header("Accept", "application/json"),
    )
    .await?;

    if !resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    let body: Value = serde_json::from_str(&resp.body)
        .map_err(|e| format!("Failed to parse version history: {}", e))?;

    let items = body
        .as_array()
        .or_else(|| body.get("items").and_then(|i| i.as_array()))
        .cloned()
        .unwrap_or_default();

    let versions: Vec<CompositionVersion> = items
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

            CompositionVersion {
                version_id,
                preceding_version_uid: item
                    .get("preceding_version_uid")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                lifecycle_state: None,
                commit_audit,
                time_committed,
            }
        })
        .collect();

    Ok(versions)
}

/// Fetch a single composition VERSION (not just the composition content) and
/// extract the CONTRIBUTION it was committed as part of, if the server
/// includes that linkage.
///
/// Standard openEHR REST API endpoint:
/// `GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version/{version_uid}`,
/// which returns an ORIGINAL_VERSION<COMPOSITION> containing a `contribution`
/// OBJECT_REF. See PRD/OEH-28 — this is how the version history (and the
/// composition Versions tab) links out to the CONTRIBUTION audit trail view.
#[tauri::command]
pub async fn get_composition_version_contribution(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
    versioned_object_uid: String,
    version_uid: String,
) -> Result<Option<String>, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = format!(
        "{}/rest/openehr/v1/ehr/{}/versioned_composition/{}/version/{}",
        base, ehr_id, versioned_object_uid, version_uid
    );

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::GET, &url, &profile.auth_method)
            .header("Accept", "application/json"),
    )
    .await?;

    if !resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    let body: Value =
        serde_json::from_str(&resp.body).map_err(|e| format!("Failed to parse version: {}", e))?;

    Ok(body
        .get("contribution")
        .and_then(|c| c.get("id"))
        .and_then(|i| i.get("value"))
        .and_then(|v| v.as_str())
        .map(String::from))
}

#[tauri::command]
pub async fn create_composition(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
    template_id: String,
    composition_data: Value,
) -> Result<String, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    // EHRBase requires template_id as query parameter for FLAT format
    let url = format!(
        "{}/rest/openehr/v1/ehr/{}/composition?templateId={}",
        base,
        ehr_id,
        urlencoding::encode(&template_id)
    );

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::POST, &url, &profile.auth_method)
            .header(
                "Content-Type",
                flat_composition_content_type(&profile.server_type),
            )
            .header("Accept", "application/json")
            // The openEHR REST spec requires the target template for a
            // Simplified-Format (FLAT) COMPOSITION commit to be named in
            // this header; FerroEHR enforces that and rejects the request
            // with a 422 otherwise. EHRBase accepts it fine alongside the
            // `templateId` query parameter it additionally expects.
            .header("openehr-template-id", &template_id)
            .json(&composition_data),
    )
    .await?;

    if !resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    // EHRBase returns 201/204 with Location header, but may have empty body
    // Try to extract composition UID from Location header first, fall back to response body
    if let Some(location) = resp.headers.get("location") {
        // Extract composition UID from the location path
        // Format: /rest/openehr/v1/ehr/{ehr_id}/composition/{composition_uid}
        let composition_uid = location
            .split('/')
            .next_back()
            .ok_or("Could not extract composition UID from Location header")?
            .to_string();

        return Ok(composition_uid);
    }

    // Fall back to parsing response body if no Location header
    if !resp.body.is_empty() {
        let result: Value = serde_json::from_str(&resp.body)
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Extract composition UID from response
        let composition_uid = result
            .get("uid")
            .and_then(|u| u.get("value"))
            .and_then(|v| v.as_str())
            .ok_or("Composition UID not found in response")?
            .to_string();

        return Ok(composition_uid);
    }

    Err("No Location header or response body found".to_string())
}

#[tauri::command]
pub async fn update_composition(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
    composition_uid: String,
    template_id: String,
    composition_data: Value,
) -> Result<String, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    // composition_uid is the full versioned uid ("<uuid>::<system>::<version>"),
    // matching what If-Match needs — but the PUT path parameter is the plain
    // VERSIONED_OBJECT uid. EHRBase enforces this strictly and 404s ("only
    // UUID-type versionedObjectUids are supported") if the version/system
    // suffix is left on; FerroEHR tolerates the full string here but still
    // needs it, unabridged, in If-Match.
    let versioned_object_uid = composition_uid
        .split("::")
        .next()
        .unwrap_or(&composition_uid);

    let url = format!(
        "{}/rest/openehr/v1/ehr/{}/composition/{}",
        base, ehr_id, versioned_object_uid
    );

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::PUT, &url, &profile.auth_method)
            .header(
                "Content-Type",
                flat_composition_content_type(&profile.server_type),
            )
            .header("Accept", "application/json")
            // See create_composition — same Simplified-Format requirement.
            .header("openehr-template-id", &template_id)
            // Per the openEHR REST API spec, updating a COMPOSITION is an
            // optimistic-concurrency operation: the server requires If-Match
            // to name the preceding version being updated from (quoted, like
            // the ETag it returns on GET), and rejects the request outright
            // without it — confirmed against both EHRBase and FerroEHR.
            .header("If-Match", format!("\"{}\"", composition_uid))
            .json(&composition_data),
    )
    .await?;

    if !resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    // As with create_composition: whether the server returns the updated
    // COMPOSITION as a JSON body (200) or an empty one (204, e.g. FerroEHR's
    // default `Prefer: return=minimal` behavior when no Prefer header is
    // sent) is server-dependent. Try the Location header first, then fall
    // back to the body, instead of assuming a body is always present.
    if let Some(location) = resp.headers.get("location") {
        let new_uid = location
            .split('/')
            .next_back()
            .ok_or("Could not extract composition UID from Location header")?
            .to_string();

        return Ok(new_uid);
    }

    if !resp.body.is_empty() {
        let result: Value = serde_json::from_str(&resp.body)
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let new_uid = result
            .get("uid")
            .and_then(|u| u.get("value"))
            .and_then(|v| v.as_str())
            .ok_or("Composition UID not found in response")?
            .to_string();

        return Ok(new_uid);
    }

    Err("No Location header or response body found".to_string())
}

#[tauri::command]
pub async fn delete_composition(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
    composition_uid: String,
) -> Result<String, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = format!(
        "{}/rest/openehr/v1/ehr/{}/composition/{}",
        base, ehr_id, composition_uid
    );

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::DELETE, &url, &profile.auth_method),
    )
    .await?;

    if !resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    Ok(format!(
        "Composition {} deleted successfully",
        composition_uid
    ))
}
