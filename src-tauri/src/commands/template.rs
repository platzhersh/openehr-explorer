use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::server::{create_client, get_profile_by_id, make_request};
use crate::inspector::send_instrumented;

#[tauri::command]
pub async fn get_template_example(
    app: tauri::AppHandle,
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

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::GET, &url, &profile.auth_method)
            .header("Accept", "application/json"),
    )
    .await?;

    if !resp.is_success {
        return Err(format!("Server returned HTTP {}", resp.status));
    }

    serde_json::from_str(&resp.body).map_err(|e| format!("Failed to parse template example: {}", e))
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermBinding {
    pub terminology: String,
    pub code: String,
    pub node_id: String,
}

/// Parse term_bindings from OPT XML.
/// Returns a list of TermBinding structs extracted from <term_bindings> elements.
fn parse_term_bindings(opt_xml: &str) -> Vec<TermBinding> {
    let mut bindings = Vec::new();
    let mut reader = Reader::from_str(opt_xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut current_terminology = String::new();
    let mut in_term_bindings = false;
    let mut in_items = false;
    let mut current_code = String::new();
    let mut current_node_id = String::new();
    let mut current_tag = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                current_tag = tag.clone();

                if tag == "term_bindings" {
                    in_term_bindings = true;
                    // Extract terminology attribute
                    for attr in e.attributes().flatten() {
                        if String::from_utf8_lossy(attr.key.as_ref()) == "terminology" {
                            current_terminology = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                } else if in_term_bindings && tag == "items" {
                    in_items = true;
                    current_code.clear();
                    current_node_id.clear();
                    // Extract code attribute if present
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        if key == "code" {
                            current_node_id = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if in_items {
                    let text = e.unescape().unwrap_or_default().to_string();
                    match current_tag.as_str() {
                        "code_string" => current_code = text,
                        "terminology_id" | "value" => {
                            // Inside terminology_id/value or code items
                            if current_tag == "value" && current_code.is_empty() {
                                // Could be terminology_id value — skip
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "term_bindings" {
                    in_term_bindings = false;
                    current_terminology.clear();
                } else if tag == "items" && in_items {
                    if !current_code.is_empty() {
                        bindings.push(TermBinding {
                            terminology: current_terminology.clone(),
                            code: current_code.clone(),
                            node_id: current_node_id.clone(),
                        });
                    }
                    in_items = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    bindings
}

#[tauri::command]
pub async fn get_term_bindings(
    app: tauri::AppHandle,
    server_id: String,
    template_id: String,
) -> Result<Vec<TermBinding>, String> {
    // Fetch OPT XML first
    let opt_xml = get_template_opt(app, server_id, template_id).await?;
    Ok(parse_term_bindings(&opt_xml))
}
