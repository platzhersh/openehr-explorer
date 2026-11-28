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

/// URL of the endpoint that validates a FLAT composition without storing it,
/// or `None` where the server type has none.
///
/// The openEHR REST API defines no such operation. Better Platform offers its
/// own `POST <base_url>/rest/v1/composition/validate` (per the Better CDR 5.x
/// REST API specification; not yet verified against a live server), outside
/// the openEHR API root. See ADR-0030.
fn composition_validate_url(profile: &ServerProfile, template_id: &str) -> Option<String> {
    match profile.server_type {
        ServerType::BetterPlatform => Some(format!(
            "{}/rest/v1/composition/validate?templateId={}&format=FLAT",
            profile.base_url.trim_end_matches('/'),
            urlencoding::encode(template_id)
        )),
        ServerType::Ehrbase | ServerType::FerroEhr | ServerType::Generic => None,
    }
}

/// Outcome of validating a composition. A rejected composition is a successful
/// validation run, so it is reported here rather than as a command error.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CompositionValidation {
    pub valid: bool,
    /// Vendor error code, e.g. `COMP-1081`.
    pub code: Option<String>,
    /// Summary of what is wrong.
    pub message: Option<String>,
    /// Specifics, such as the path of the offending value.
    pub details: Vec<String>,
}

/// The fields of Better's error body (`code`, `userMessage`,
/// `developerMessage`, `exceptionMessage`). A body that isn't in that shape is
/// passed through as the message, so nothing the server said gets lost.
fn parse_validation_error(body: &str) -> CompositionValidation {
    let json: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let field = |name: &str| {
        json.get(name)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(String::from)
    };

    let message = field("userMessage")
        .or_else(|| field("developerMessage"))
        .or_else(|| Some(body.trim().to_string()).filter(|b| !b.is_empty()));

    let mut details: Vec<String> = Vec::new();
    for detail in [field("developerMessage"), field("exceptionMessage")]
        .into_iter()
        .flatten()
    {
        if Some(&detail) != message.as_ref() && !details.contains(&detail) {
            details.push(detail);
        }
    }

    CompositionValidation {
        valid: false,
        code: field("code"),
        message,
        details,
    }
}

/// Maps the validate response: 2xx (204) is valid, 400 is a rejected
/// composition, and anything else means validation did not run.
fn interpret_validation_response(status: u16, body: &str) -> Result<CompositionValidation, String> {
    match status {
        200..=299 => Ok(CompositionValidation {
            valid: true,
            ..Default::default()
        }),
        400 => Ok(parse_validation_error(body)),
        _ => {
            let error = parse_validation_error(body);
            let reason = [error.code, error.message]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(": ");
            Err(format!(
                "Could not validate (HTTP {}){}{}",
                status,
                if reason.is_empty() { "" } else { ": " },
                reason
            ))
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

/// Checks a FLAT composition against its template without storing it.
#[tauri::command]
pub async fn validate_composition(
    app: tauri::AppHandle,
    server_id: String,
    template_id: String,
    composition_data: Value,
) -> Result<CompositionValidation, String> {
    let profile = get_profile_by_id(&server_id)?;
    let url = composition_validate_url(&profile, &template_id)
        .ok_or("Validating without saving is not supported for this server type")?;
    let client = create_client(&profile);

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::POST, &url, &profile.auth_method)
            .header("Accept", "application/json")
            // `.json` sets `Content-Type: application/json`, the only JSON
            // media type Better's spec lists for this endpoint.
            .json(&composition_data),
    )
    .await?;

    interpret_validation_response(resp.status, &resp.body)
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
    fn better_validates_outside_the_openehr_api_root() {
        assert_eq!(
            composition_validate_url(&profile(ServerType::BetterPlatform), "Vital Signs")
                .as_deref(),
            Some(
                "https://cdr.example.com/ehr/rest/v1/composition/validate\
                 ?templateId=Vital%20Signs&format=FLAT"
            )
        );
    }

    #[test]
    fn other_server_types_have_no_validate_endpoint() {
        for server_type in [
            ServerType::Ehrbase,
            ServerType::FerroEhr,
            ServerType::Generic,
        ] {
            assert_eq!(composition_validate_url(&profile(server_type), "t"), None);
        }
    }

    #[test]
    fn no_content_means_valid() {
        let result = interpret_validation_response(204, "").unwrap();
        assert!(result.valid);
        assert_eq!(result.message, None);
    }

    /// The COMP-1081 example from Better's REST error documentation.
    #[test]
    fn bad_request_reports_why_the_composition_was_rejected() {
        let body = r#"{
            "code": "COMP-1081",
            "developerMessage": "Composition validation failed (there are missing or invalid values).",
            "exceptionMessage": "Missing DvCodedText.value at vitals/symptoms:0",
            "status": 400,
            "userMessage": "Composition validation failed (there are missing or invalid values)."
        }"#;
        let result = interpret_validation_response(400, body).unwrap();
        assert_eq!(
            result,
            CompositionValidation {
                valid: false,
                code: Some("COMP-1081".to_string()),
                message: Some(
                    "Composition validation failed (there are missing or invalid values)."
                        .to_string()
                ),
                // The developer message repeats the user message, so only once.
                details: vec!["Missing DvCodedText.value at vitals/symptoms:0".to_string()],
            }
        );
    }

    #[test]
    fn unrecognised_error_body_is_kept_verbatim() {
        let result = interpret_validation_response(400, " <h1>Bad</h1> ").unwrap();
        assert!(!result.valid);
        assert_eq!(result.message.as_deref(), Some("<h1>Bad</h1>"));
        assert!(result.details.is_empty());
    }

    #[test]
    fn other_statuses_mean_validation_did_not_run() {
        let body = r#"{"code":"TMPL-3021","userMessage":"Template does not exist."}"#;
        assert_eq!(
            interpret_validation_response(404, body).unwrap_err(),
            "Could not validate (HTTP 404): TMPL-3021: Template does not exist."
        );
        assert_eq!(
            interpret_validation_response(500, "").unwrap_err(),
            "Could not validate (HTTP 500)"
        );
    }
}
