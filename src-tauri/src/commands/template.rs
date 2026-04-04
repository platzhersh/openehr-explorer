use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::server::{create_client, get_profile_by_id, make_request};

#[tauri::command]
pub async fn get_template_example(
    server_id: String,
    template_id: String,
) -> Result<Value, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = format!(
        "{}/rest/openehr/v1/definition/template/adl1.4/{}/example?format=FLAT",
        base,
        urlencoding::encode(&template_id)
    );

    let response = make_request(&client, reqwest::Method::GET, &url, &profile.auth_method)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch template example: {}", e))?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Server returned HTTP {}: {}", status, body));
    }

    response
        .json::<Value>()
        .await
        .map_err(|e| format!("Failed to parse template example: {}", e))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSummary {
    pub template_id: String,
    pub concept: Option<String>,
    pub archetype_id: Option<String>,
    pub created_timestamp: Option<String>,
}

#[tauri::command]
pub async fn list_templates(server_id: String) -> Result<Vec<TemplateSummary>, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = format!("{}/rest/openehr/v1/definition/template/adl1.4", base);

    let response = make_request(&client, reqwest::Method::GET, &url, &profile.auth_method)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch templates: {}", e))?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Server returned HTTP {}: {}", status, body));
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse templates: {}", e))?;

    let templates = match body {
        Value::Array(arr) => arr
            .iter()
            .map(|item| TemplateSummary {
                template_id: item
                    .get("template_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                concept: item
                    .get("concept")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                archetype_id: item
                    .get("archetype_id")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                created_timestamp: item
                    .get("created_timestamp")
                    .and_then(|v| v.as_str())
                    .map(String::from),
            })
            .collect(),
        _ => Vec::new(),
    };

    Ok(templates)
}

#[tauri::command]
pub async fn get_web_template(
    server_id: String,
    template_id: String,
) -> Result<Value, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = format!(
        "{}/rest/openehr/v1/definition/template/adl1.4/{}",
        base,
        urlencoding::encode(&template_id)
    );

    let response = make_request(&client, reqwest::Method::GET, &url, &profile.auth_method)
        .header("Accept", "application/openehr.wt+json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch web template: {}", e))?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Server returned HTTP {}: {}", status, body));
    }

    response
        .json::<Value>()
        .await
        .map_err(|e| format!("Failed to parse web template: {}", e))
}

#[tauri::command]
pub async fn get_template_opt(
    server_id: String,
    template_id: String,
) -> Result<String, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = format!(
        "{}/rest/openehr/v1/definition/template/adl1.4/{}",
        base,
        urlencoding::encode(&template_id)
    );

    let response = make_request(&client, reqwest::Method::GET, &url, &profile.auth_method)
        .header("Accept", "application/xml")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch OPT: {}", e))?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Server returned HTTP {}: {}", status, body));
    }

    response
        .text()
        .await
        .map_err(|e| format!("Failed to read OPT: {}", e))
}

#[tauri::command]
pub async fn upload_template(
    server_id: String,
    opt_xml: String,
) -> Result<String, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = format!("{}/rest/openehr/v1/definition/template/adl1.4", base);

    let response = make_request(&client, reqwest::Method::POST, &url, &profile.auth_method)
        .header("Content-Type", "application/xml")
        .body(opt_xml)
        .send()
        .await
        .map_err(|e| format!("Failed to upload template: {}", e))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if status.is_success() || status.as_u16() == 201 || status.as_u16() == 204 {
        Ok(format!("Template uploaded successfully (HTTP {})", status.as_u16()))
    } else {
        Err(format!("Upload failed (HTTP {}): {}", status.as_u16(), body))
    }
}
