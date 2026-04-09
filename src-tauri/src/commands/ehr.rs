use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::server::{create_client, get_profile_by_id, make_request, ServerType};
use crate::inspector::send_instrumented;

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
    app: tauri::AppHandle,
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

#[tauri::command]
pub async fn create_ehr(
    app: tauri::AppHandle,
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
    if let (Some(subject_id), Some(subject_namespace)) =
        (request.subject_id, request.subject_namespace)
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

    // For EHR creation, EHRBase expects the EHR_STATUS object directly, not wrapped
    let request_body = if request.ehr_id.is_some() {
        // If custom EHR ID is provided, we need to wrap it
        ehr_body["ehr_status"] = ehr_status.clone();
        ehr_body
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

    // Check if any criteria was provided (either predicates or has_compositions:true)
    if predicates.is_empty() && !has_compositions_filter {
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

    let total = results.len();
    let limit_reached = total >= 200;

    Ok(EhrSearchResponse {
        results,
        total,
        limit_reached,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
