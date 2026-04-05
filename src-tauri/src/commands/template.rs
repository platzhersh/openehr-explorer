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

/// Normalise term binding codes from OPT format to clean (terminology_id, code) pairs.
///
/// Handles:
/// - `[SNOMED-CT(2003)::364090009]` → `("SNOMED-CT", "364090009")`
/// - `[SNOMED-CT::364090009]` → `("SNOMED-CT", "364090009")`
/// - `364090009` → `("", "364090009")` (bare code, no terminology ID)
fn normalise_term_code(raw: &str) -> (String, String) {
    let trimmed = raw.trim();
    let stripped = trimmed.trim_start_matches('[').trim_end_matches(']');

    if let Some((terminology_part, code)) = stripped.split_once("::") {
        // Remove version qualifier: SNOMED-CT(2003) → SNOMED-CT
        let terminology_id = terminology_part
            .split('(')
            .next()
            .unwrap_or(terminology_part)
            .to_string();
        (terminology_id, code.to_string())
    } else {
        // No delimiter → bare code
        (String::new(), stripped.to_string())
    }
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
                        // Normalise the code before storing
                        let (normalised_terminology, normalised_code) =
                            normalise_term_code(&current_code);

                        // Use the normalised terminology if available, otherwise use the current_terminology from the attribute
                        let final_terminology = if !normalised_terminology.is_empty() {
                            normalised_terminology
                        } else {
                            current_terminology.clone()
                        };

                        bindings.push(TermBinding {
                            terminology: final_terminology,
                            code: normalised_code,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalise_term_code_bare_code() {
        let (terminology, code) = normalise_term_code("364090009");
        assert_eq!(terminology, "");
        assert_eq!(code, "364090009");
    }

    #[test]
    fn test_normalise_term_code_snomed_unversioned() {
        let (terminology, code) = normalise_term_code("[SNOMED-CT::364090009]");
        assert_eq!(terminology, "SNOMED-CT");
        assert_eq!(code, "364090009");
    }

    #[test]
    fn test_normalise_term_code_snomed_versioned() {
        let (terminology, code) = normalise_term_code("[SNOMED-CT(2003)::364090009]");
        assert_eq!(terminology, "SNOMED-CT");
        assert_eq!(code, "364090009");
    }

    #[test]
    fn test_normalise_term_code_loinc() {
        let (terminology, code) = normalise_term_code("[LOINC::8867-4]");
        assert_eq!(terminology, "LOINC");
        assert_eq!(code, "8867-4");
    }

    #[test]
    fn test_normalise_term_code_icd10() {
        let (terminology, code) = normalise_term_code("[ICD-10::A41.9]");
        assert_eq!(terminology, "ICD-10");
        assert_eq!(code, "A41.9");
    }

    #[test]
    fn test_normalise_term_code_with_whitespace() {
        let (terminology, code) = normalise_term_code("  [SNOMED-CT::364090009]  ");
        assert_eq!(terminology, "SNOMED-CT");
        assert_eq!(code, "364090009");
    }

    #[test]
    fn test_parse_term_bindings_with_bracketed_codes() {
        let opt_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<template xmlns="http://schemas.openehr.org/v1">
    <term_bindings terminology="SNOMED-CT">
        <items code="at0000">
            <code_string>[SNOMED-CT(2003)::364090009]</code_string>
        </items>
        <items code="at0001">
            <code_string>[SNOMED-CT::91302008]</code_string>
        </items>
    </term_bindings>
    <term_bindings terminology="LOINC">
        <items code="at0002">
            <code_string>[LOINC::8867-4]</code_string>
        </items>
    </term_bindings>
    <term_bindings terminology="ICD-10">
        <items code="at0003">
            <code_string>A41.9</code_string>
        </items>
    </term_bindings>
</template>
"#;

        let bindings = parse_term_bindings(opt_xml);

        assert_eq!(bindings.len(), 4);

        // First binding: versioned SNOMED-CT
        assert_eq!(bindings[0].terminology, "SNOMED-CT");
        assert_eq!(bindings[0].code, "364090009");
        assert_eq!(bindings[0].node_id, "at0000");

        // Second binding: unversioned SNOMED-CT
        assert_eq!(bindings[1].terminology, "SNOMED-CT");
        assert_eq!(bindings[1].code, "91302008");
        assert_eq!(bindings[1].node_id, "at0001");

        // Third binding: LOINC
        assert_eq!(bindings[2].terminology, "LOINC");
        assert_eq!(bindings[2].code, "8867-4");
        assert_eq!(bindings[2].node_id, "at0002");

        // Fourth binding: bare code (should use terminology from attribute)
        assert_eq!(bindings[3].terminology, "ICD-10");
        assert_eq!(bindings[3].code, "A41.9");
        assert_eq!(bindings[3].node_id, "at0003");
    }
}
