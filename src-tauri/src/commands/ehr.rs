use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::server::{create_client, get_profile_by_id, make_request};

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
}

#[tauri::command]
pub async fn list_ehrs(
    server_id: String,
    offset: usize,
    limit: usize,
) -> Result<EhrListResponse, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    // Use AQL to list EHRs since the REST API list endpoint varies by implementation
    let aql = format!(
        "SELECT e/ehr_id/value, e/time_created/value, e/system_id/value FROM EHR e LIMIT {} OFFSET {}",
        limit, offset
    );

    let url = format!("{}/rest/openehr/v1/query/aql", base);
    let response = make_request(&client, reqwest::Method::POST, &url, &profile.auth_method)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "q": aql }))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch EHRs: {}", e))?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Server returned HTTP {}: {}", status, body));
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

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
    })
}

#[tauri::command]
pub async fn get_ehr_detail(server_id: String, ehr_id: String) -> Result<EhrDetail, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    // Fetch EHR status
    let status_url = format!("{}/rest/openehr/v1/ehr/{}", base, ehr_id);
    let ehr_response = make_request(
        &client,
        reqwest::Method::GET,
        &status_url,
        &profile.auth_method,
    )
    .header("Accept", "application/json")
    .send()
    .await
    .map_err(|e| format!("Failed to fetch EHR: {}", e))?;

    let ehr_json: Value = if ehr_response.status().is_success() {
        ehr_response
            .json()
            .await
            .map_err(|e| format!("Failed to parse EHR: {}", e))?
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
    let comp_response = make_request(
        &client,
        reqwest::Method::POST,
        &query_url,
        &profile.auth_method,
    )
    .header("Content-Type", "application/json")
    .json(&serde_json::json!({ "q": aql }))
    .send()
    .await
    .map_err(|e| format!("Failed to fetch compositions: {}", e))?;

    let compositions = if comp_response.status().is_success() {
        let body: Value = comp_response
            .json()
            .await
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

#[tauri::command]
pub async fn create_ehr(
    server_id: String,
    request: CreateEhrRequest,
) -> Result<CreateEhrResponse, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    // Build EHR request body
    let mut ehr_body = serde_json::json!({});

    if let Some(ehr_id) = &request.ehr_id {
        ehr_body["ehr_id"] = serde_json::json!({
            "_type": "HIER_OBJECT_ID",
            "value": ehr_id
        });
    }

    // Build EHR status
    let mut ehr_status = serde_json::json!({
        "_type": "EHR_STATUS",
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
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
    if request.subject_namespace.is_some() && request.subject_id.is_some() {
        ehr_status["subject"] = serde_json::json!({
            "_type": "PARTY_SELF",
            "external_ref": {
                "_type": "PARTY_REF",
                "id": {
                    "_type": "GENERIC_ID",
                    "value": request.subject_id.unwrap(),
                    "scheme": request.subject_namespace.unwrap()
                },
                "namespace": "external",
                "type": "PERSON"
            }
        });
    }

    // For EHR creation, EHRBase expects the EHR_STATUS object directly, not wrapped
    let request_body = if request.ehr_id.is_some() {
        // If custom EHR ID is provided, we need to wrap it
        ehr_body["ehr_status"] = ehr_status.clone();
        ehr_body
    } else {
        // Otherwise, send EHR_STATUS directly as the root object
        ehr_status
    };

    // Debug: print the JSON being sent
    eprintln!("Creating EHR with JSON:\n{}", serde_json::to_string_pretty(&request_body).unwrap_or_default());

    let url = format!("{}/rest/openehr/v1/ehr", base);
    let response = make_request(&client, reqwest::Method::POST, &url, &profile.auth_method)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Failed to create EHR: {}", e))?;

    let status = response.status();

    if !status.is_success() {
        let response_text = response.text().await.unwrap_or_default();
        return Err(format!("Server returned HTTP {}: {}", status.as_u16(), response_text));
    }

    // EHRBase returns 201 Created with Location header, but empty body
    // Extract EHR ID from Location header: /rest/openehr/v1/ehr/{ehr_id}
    let location = response
        .headers()
        .get("Location")
        .and_then(|v| v.to_str().ok())
        .ok_or("Location header not found in response")?;

    eprintln!("EHR created at: {}", location);

    // Extract EHR ID from the location path
    let ehr_id = location
        .split('/')
        .last()
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
    server_id: String,
    ehr_id: String,
    request: UpdateEhrStatusRequest,
) -> Result<String, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    // First, fetch current EHR status to get the version UID
    let get_url = format!("{}/rest/openehr/v1/ehr/{}/ehr_status", base, ehr_id);
    let get_response = make_request(&client, reqwest::Method::GET, &get_url, &profile.auth_method)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch current EHR status: {}", e))?;

    if !get_response.status().is_success() {
        let status = get_response.status().as_u16();
        let body = get_response.text().await.unwrap_or_default();
        return Err(format!("Server returned HTTP {}: {}", status, body));
    }

    let current_status: Value = get_response
        .json()
        .await
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
    if request.subject_namespace.is_some() && request.subject_id.is_some() {
        updated_status["subject"] = serde_json::json!({
            "_type": "PARTY_SELF",
            "external_ref": {
                "id": {
                    "_type": "GENERIC_ID",
                    "value": request.subject_id.unwrap(),
                    "scheme": request.subject_namespace.unwrap()
                },
                "namespace": "external"
            }
        });
    }

    // PUT request with If-Match header
    let put_url = format!("{}/rest/openehr/v1/ehr/{}/ehr_status", base, ehr_id);
    let put_response = make_request(&client, reqwest::Method::PUT, &put_url, &profile.auth_method)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("If-Match", version_uid)
        .json(&updated_status)
        .send()
        .await
        .map_err(|e| format!("Failed to update EHR status: {}", e))?;

    if !put_response.status().is_success() {
        let status = put_response.status().as_u16();
        let body = put_response.text().await.unwrap_or_default();
        return Err(format!("Server returned HTTP {}: {}", status, body));
    }

    Ok("EHR status updated successfully".to_string())
}

#[tauri::command]
pub async fn delete_ehr(server_id: String, ehr_id: String) -> Result<String, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = format!("{}/rest/openehr/v1/ehr/{}", base, ehr_id);
    let response = make_request(&client, reqwest::Method::DELETE, &url, &profile.auth_method)
        .send()
        .await
        .map_err(|e| format!("Failed to delete EHR: {}", e))?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Server returned HTTP {}: {}", status, body));
    }

    Ok(format!("EHR {} deleted successfully", ehr_id))
}
