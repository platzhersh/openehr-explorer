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
        compositions,
    })
}
