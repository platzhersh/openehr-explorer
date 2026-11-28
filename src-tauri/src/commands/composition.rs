use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::server::{create_client, get_profile_by_id, make_request, ServerProfile, ServerType};
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
/// Better Platform (verified against cloud.better.care) also rejects `.schema`
/// with a 415 and lists `application/openehr.wt.flat+json` as accepted.
/// Unspecified/generic servers keep the `.schema` value this app has always
/// sent them (unconfirmed either way, but unbroken until reported).
fn flat_composition_content_type(server_type: &ServerType) -> &'static str {
    match server_type {
        ServerType::FerroEhr | ServerType::BetterPlatform => "application/openehr.wt.flat+json",
        ServerType::Ehrbase | ServerType::Generic => "application/openehr.wt.flat.schema+json",
    }
}

/// Name of the header that carries the target template of a FLAT commit.
/// Better's gateway only recognises the spelling `openEHR-TEMPLATE_ID`
/// (underscore); with the spec's `openehr-template-id` it answers 400
/// "Template id is missing". Other servers take the spec spelling.
fn template_id_header(server_type: &ServerType) -> &'static str {
    match server_type {
        ServerType::BetterPlatform => "openEHR-TEMPLATE_ID",
        _ => "openehr-template-id",
    }
}

/// URL of `action` (`meta` or `restore`) on a composition in Better Platform's
/// own API, which sits outside the openEHR API root. `None` for other server
/// types: the openEHR REST API has no restore operation (ADR-0030). Based on
/// the Better CDR 5.x REST specification; not yet verified against a live server.
fn better_composition_url(profile: &ServerProfile, uid: &str, action: &str) -> Option<String> {
    match profile.server_type {
        ServerType::BetterPlatform => Some(format!(
            "{}/rest/v1/composition/{}/{}",
            profile.base_url.trim_end_matches('/'),
            uid,
            action
        )),
        ServerType::Ehrbase | ServerType::FerroEhr | ServerType::Generic => None,
    }
}

/// Where a composition version stands in its history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositionState {
    /// Whether the latest version is a deletion.
    pub deleted: bool,
    /// Whether the requested version is the latest one.
    pub is_latest: bool,
    pub latest_uid: String,
}

/// Reads the `meta` response for the *latest* version of a composition (asked
/// for by its bare versioned-object UID) against the version being looked at.
fn parse_composition_state(requested_uid: &str, body: &str) -> Result<CompositionState, String> {
    let json: Value = serde_json::from_str(body)
        .map_err(|e| format!("Failed to parse composition state: {}", e))?;
    let latest_uid = json
        .get("compositionUid")
        .and_then(Value::as_str)
        .ok_or("Composition state has no compositionUid")?
        .to_string();
    let deleted = json
        .get("deleted")
        .and_then(Value::as_bool)
        .ok_or("Composition state has no deleted flag")?;

    // A bare UID stands for whatever is latest.
    let is_latest = !requested_uid.contains("::") || requested_uid == latest_uid;

    Ok(CompositionState {
        deleted,
        is_latest,
        latest_uid,
    })
}

fn parse_restored_uid(body: &str) -> Result<String, String> {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|json| json.get("compositionUid")?.as_str().map(String::from))
        .ok_or_else(|| "Restored, but the server did not return the new version UID".to_string())
}

/// `None` where the server type can't report it.
#[tauri::command]
pub async fn get_composition_state(
    app: tauri::AppHandle,
    server_id: String,
    composition_uid: String,
) -> Result<Option<CompositionState>, String> {
    let profile = get_profile_by_id(&server_id)?;
    let versioned_object_uid = composition_uid.split("::").next().unwrap_or_default();
    let Some(url) = better_composition_url(&profile, versioned_object_uid, "meta") else {
        return Ok(None);
    };
    let client = create_client(&profile);

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

    parse_composition_state(&composition_uid, &resp.body).map(Some)
}

/// Undoes a delete by committing a new version. `composition_uid` must be the
/// latest (deleted) version. Returns the new version UID.
#[tauri::command]
pub async fn restore_composition(
    app: tauri::AppHandle,
    server_id: String,
    composition_uid: String,
) -> Result<String, String> {
    let profile = get_profile_by_id(&server_id)?;
    let url = better_composition_url(&profile, &composition_uid, "restore")
        .ok_or("Restoring a composition is not supported for this server type")?;
    let client = create_client(&profile);

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::PUT, &url, &profile.auth_method)
            .header("Accept", "application/json"),
    )
    .await?;

    if !resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    parse_restored_uid(&resp.body)
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
    let api_root = profile.api_root();
    let base = api_root.as_str();

    let url = format!("{}/ehr/{}/composition/{}", base, ehr_id, composition_uid);

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

    // The openEHR REST API answers 204, with no body, for a deleted version.
    if resp.status == 204 {
        return Err("This composition version is deleted".to_string());
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
    let api_root = profile.api_root();
    let base = api_root.as_str();

    let url = format!("{}/ehr/{}/composition/{}", base, ehr_id, composition_uid);

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
    let api_root = profile.api_root();
    let base = api_root.as_str();

    let url = format!(
        "{}/ehr/{}/versioned_composition/{}/revision_history",
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
    let api_root = profile.api_root();
    let base = api_root.as_str();

    let url = format!(
        "{}/ehr/{}/versioned_composition/{}/version/{}",
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
    let api_root = profile.api_root();
    let base = api_root.as_str();

    // EHRBase requires template_id as query parameter for FLAT format
    let url = format!(
        "{}/ehr/{}/composition?templateId={}",
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
            .header(template_id_header(&profile.server_type), &template_id)
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
    let api_root = profile.api_root();
    let base = api_root.as_str();

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
        "{}/ehr/{}/composition/{}",
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
            .header(template_id_header(&profile.server_type), &template_id)
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
    let api_root = profile.api_root();
    let base = api_root.as_str();

    let url = format!("{}/ehr/{}/composition/{}", base, ehr_id, composition_uid);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::server::AuthMethod;

    fn profile(server_type: ServerType) -> ServerProfile {
        ServerProfile {
            id: "p".to_string(),
            name: "p".to_string(),
            base_url: "https://cdr.example.com/ehr/".to_string(),
            server_type,
            auth_method: AuthMethod::None,
            admin_auth_method: None,
            terminology_url: None,
            api_path_prefix: Some("/custom/openehr/v1".to_string()),
            is_default: false,
        }
    }

    #[test]
    fn only_better_has_composition_meta_and_restore() {
        assert_eq!(
            better_composition_url(&profile(ServerType::BetterPlatform), "a::sys::2", "restore")
                .as_deref(),
            Some("https://cdr.example.com/ehr/rest/v1/composition/a::sys::2/restore")
        );
        for other in [
            ServerType::Ehrbase,
            ServerType::FerroEhr,
            ServerType::Generic,
        ] {
            assert_eq!(better_composition_url(&profile(other), "a", "meta"), None);
        }
    }

    #[test]
    fn state_compares_the_viewed_version_with_the_latest() {
        let body = r#"{"compositionUid":"a::sys::3","deleted":true}"#;
        let older = parse_composition_state("a::sys::2", body).unwrap();
        assert!(older.deleted && !older.is_latest);
        assert_eq!(older.latest_uid, "a::sys::3");
        assert!(
            parse_composition_state("a::sys::3", body)
                .unwrap()
                .is_latest
        );
        assert!(parse_composition_state("a", body).unwrap().is_latest);
    }

    #[test]
    fn incomplete_state_is_an_error_rather_than_not_deleted() {
        assert!(parse_composition_state("a", r#"{"compositionUid":"a::sys::1"}"#).is_err());
        assert!(parse_composition_state("a", "").is_err());
    }

    #[test]
    fn restored_uid_comes_from_the_body() {
        let body = r#"{"action":"EXECUTE","compositionUid":"a::sys::4"}"#;
        assert_eq!(parse_restored_uid(body).unwrap(), "a::sys::4");
        assert!(parse_restored_uid("").is_err());
    }
}
