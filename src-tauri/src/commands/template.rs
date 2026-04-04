use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::server::{create_client, get_profile_by_id, make_request};
use crate::inspector::send_instrumented;

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
pub async fn list_templates(
    app: tauri::AppHandle,
    server_id: String,
) -> Result<Vec<TemplateSummary>, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = format!("{}/rest/openehr/v1/definition/template/adl1.4", base);

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
    app: tauri::AppHandle,
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

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::GET, &url, &profile.auth_method)
            .header("Accept", "application/openehr.wt+json"),
    )
    .await?;

    if !resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    serde_json::from_str(&resp.body).map_err(|e| format!("Failed to parse web template: {}", e))
}

#[tauri::command]
pub async fn get_template_opt(
    app: tauri::AppHandle,
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

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::GET, &url, &profile.auth_method)
            .header("Accept", "application/xml"),
    )
    .await?;

    if !resp.is_success {
        return Err(format!(
            "Server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    Ok(resp.body)
}

#[tauri::command]
pub async fn upload_template(
    app: tauri::AppHandle,
    server_id: String,
    opt_xml: String,
) -> Result<String, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = format!("{}/rest/openehr/v1/definition/template/adl1.4", base);

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::POST, &url, &profile.auth_method)
            .header("Content-Type", "application/xml")
            .body(opt_xml),
    )
    .await?;

    if resp.is_success || resp.status == 201 || resp.status == 204 {
        Ok(format!(
            "Template uploaded successfully (HTTP {})",
            resp.status
        ))
    } else {
        Err(format!(
            "Upload failed (HTTP {}): {}",
            resp.status, resp.body
        ))
    }
}
