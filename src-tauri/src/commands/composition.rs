use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::server::{create_client, get_profile_by_id, make_request};
use crate::inspector::send_instrumented;

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
        make_request(&client, reqwest::Method::GET, &url, &profile.auth_method)
            .header("Accept", "application/openehr.wt.flat+json"),
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

#[tauri::command]
pub async fn create_composition(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
    composition_data: Value,
) -> Result<String, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = format!("{}/rest/openehr/v1/ehr/{}/composition", base, ehr_id);

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::POST, &url, &profile.auth_method)
            .header("Content-Type", "application/openehr.wt.flat.schema+json")
            .header("Accept", "application/json")
            .json(&composition_data),
    )
    .await?;

    if !resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    let result: Value =
        serde_json::from_str(&resp.body).map_err(|e| format!("Failed to parse response: {}", e))?;

    // Extract composition UID from response
    let composition_uid = result
        .get("uid")
        .and_then(|u| u.get("value"))
        .and_then(|v| v.as_str())
        .ok_or("Composition UID not found in response")?
        .to_string();

    Ok(composition_uid)
}

#[tauri::command]
pub async fn update_composition(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
    composition_uid: String,
    composition_data: Value,
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
        make_request(&client, reqwest::Method::PUT, &url, &profile.auth_method)
            .header("Content-Type", "application/openehr.wt.flat.schema+json")
            .header("Accept", "application/json")
            .json(&composition_data),
    )
    .await?;

    if !resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    let result: Value =
        serde_json::from_str(&resp.body).map_err(|e| format!("Failed to parse response: {}", e))?;

    // Extract new version UID
    let new_uid = result
        .get("uid")
        .and_then(|u| u.get("value"))
        .and_then(|v| v.as_str())
        .ok_or("Composition UID not found in response")?
        .to_string();

    Ok(new_uid)
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
