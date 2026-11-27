use quick_xml::escape::unescape;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::server::{
    create_client, get_profile_by_id, make_request, AuthMethod, ServerProfile, ServerType,
    DEFAULT_API_PATH_PREFIX,
};
use crate::inspector::send_instrumented;

#[tauri::command]
pub async fn get_template_example(
    app: tauri::AppHandle,
    server_id: String,
    template_id: String,
) -> Result<Value, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let api_root = profile.api_root();
    let base = api_root.as_str();

    // `detail_level` defaults to `required` per the Definition API spec (ITS-REST),
    // i.e. a minimal example with only mandatory data points. EHRBase doesn't
    // enforce that default and returns a fuller example regardless, but
    // stricter CDRs (e.g. FerroEHR) do — so we ask for `medium` explicitly to
    // get a realistic, committable example on every server, not just EHRBase.
    let url = format!(
        "{}/definition/template/adl1.4/{}/example?format=FLAT&detail_level=medium",
        base,
        urlencoding::encode(&template_id)
    );

    let mut resp = send_instrumented(
        &app,
        &client,
        make_request(&client, reqwest::Method::GET, &url, &profile.auth_method)
            .header("Accept", "application/json"),
    )
    .await?;

    // Better CDR doesn't implement the standard Definition API `/example`
    // endpoint; it exposes it on its own API instead (`<host>/rest/v1/template/<id>/example`).
    if resp.status == 404 && matches!(profile.server_type, ServerType::BetterPlatform) {
        // Vendor endpoints hang off `base_url`. Only an empty prefix means
        // `base_url` may itself already point at the API root, so strip that
        // suffix then; a custom prefix keeps `base_url` as the root.
        let base_url = profile.base_url.trim_end_matches('/');
        let server_root = match profile.api_path_prefix.as_deref() {
            Some(prefix) if !prefix.trim().trim_matches('/').is_empty() => base_url,
            _ => base
                .strip_suffix(DEFAULT_API_PATH_PREFIX)
                .unwrap_or(base_url),
        };
        let better_url = format!(
            "{}/rest/v1/template/{}/example?format=FLAT&exampleFilter=INPUT",
            server_root,
            urlencoding::encode(&template_id)
        );
        resp = send_instrumented(
            &app,
            &client,
            make_request(
                &client,
                reqwest::Method::GET,
                &better_url,
                &profile.auth_method,
            )
            .header("Accept", "application/json"),
        )
        .await?;
    }

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
    let api_root = profile.api_root();
    let base = api_root.as_str();

    let url = format!("{}/definition/template/adl1.4", base);

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
    let api_root = profile.api_root();
    let base = api_root.as_str();

    let url = format!(
        "{}/definition/template/adl1.4/{}",
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
    let api_root = profile.api_root();
    let base = api_root.as_str();

    let url = format!(
        "{}/definition/template/adl1.4/{}",
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

/// Local part of a possibly namespace-prefixed XML name (`o:template_id` -> `template_id`).
fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

/// Extract the template ID (`<template_id><value>…</value></template_id>`) from OPT XML.
///
/// Accepts namespace-prefixed elements and reads the whole `<value>` content, so
/// IDs split across text, entity/character references and CDATA are returned intact.
fn extract_template_id(opt_xml: &str) -> Option<String> {
    let mut reader = Reader::from_str(opt_xml);
    let mut in_template_id = false;
    let mut in_value = false;
    let mut id = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let name = local_name(name.as_ref());
                if name == "template_id" {
                    in_template_id = true;
                } else if in_template_id && name == "value" {
                    in_value = true;
                }
            }
            Ok(Event::Text(e)) if in_value => {
                let text = e.xml10_content();
                id.push_str(&unescape(&text).ok()?);
            }
            Ok(Event::CData(e)) if in_value => id.push_str(&e.xml10_content()),
            Ok(Event::GeneralRef(e)) if in_value => {
                if let Ok(Some(c)) = e.resolve_char_ref() {
                    id.push(c);
                } else {
                    let entity = format!("&{};", e.into_inner());
                    id.push_str(&unescape(&entity).ok()?);
                }
            }
            Ok(Event::End(e)) if in_value => {
                let name = e.name();
                if local_name(name.as_ref()) == "value" {
                    let id = id.trim().to_string();
                    return if id.is_empty() { None } else { Some(id) };
                }
            }
            Ok(Event::Eof) | Err(_) => return None,
            _ => {}
        }
    }
}

/// Template ID of an OPT, exactly as `upload_template` extracts it for an overwrite.
///
/// The overwrite confirmation shows this value, so what the user confirms is by
/// construction the template that the PUT targets (`None` if it can't be read).
#[tauri::command]
pub fn parse_opt_template_id(opt_xml: String) -> Option<String> {
    extract_template_id(&opt_xml)
}

/// URL of EHRbase's Admin API endpoint that replaces an existing template.
fn ehrbase_admin_template_url(base_url: &str, template_id: &str) -> String {
    format!(
        "{}/rest/admin/template/{}",
        base_url.trim_end_matches('/'),
        urlencoding::encode(template_id)
    )
}

/// Where, and with which credentials, to overwrite an existing template, if this
/// server type can do it at all.
///
/// The openEHR REST API only defines POST for templates (a duplicate is rejected
/// with 409), so replacing one is vendor-specific (ADR-0030). EHRbase offers
/// `PUT /rest/admin/template/{template_id}` in its Admin API, mounted beside the
/// openEHR root and needing the admin role. Other server types return `None`
/// rather than a guessed request.
fn overwrite_target(profile: &ServerProfile, template_id: &str) -> Option<(String, AuthMethod)> {
    match profile.server_type {
        ServerType::Ehrbase => {
            let auth = profile
                .admin_auth_method
                .as_ref()
                .unwrap_or(&profile.auth_method);
            Some((
                ehrbase_admin_template_url(&profile.base_url, template_id),
                auth.clone(),
            ))
        }
        _ => None,
    }
}

/// Explain a refused EHRbase template overwrite (statuses from its Admin API docs).
fn ehrbase_overwrite_error(status: u16, body: &str) -> String {
    let reason = match status {
        401 => "the server rejected the credentials; overwriting uses the admin credentials of the server profile",
        403 => "the account needs the EHRbase admin role; add admin credentials to the server profile",
        404 => "EHRbase did not find the template, or its Admin API is not enabled on the server",
        422 => "EHRbase cannot replace a template that is used by at least one composition",
        _ => return format!("Overwrite failed (HTTP {}): {}", status, body),
    };
    format!("Overwrite failed (HTTP {}): {}", status, reason)
}

/// Upload an OPT. A duplicate template ID is rejected by the server (HTTP 409).
/// With `overwrite = true` the existing template is replaced instead, which only
/// works on server types that offer a way to do that (see `overwrite_target`).
#[tauri::command]
pub async fn upload_template(
    app: tauri::AppHandle,
    server_id: String,
    opt_xml: String,
    overwrite: Option<bool>,
) -> Result<String, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let api_root = profile.api_root();
    let base = api_root.as_str();
    let overwrite = overwrite.unwrap_or(false);

    let (method, url, auth) = if overwrite {
        let template_id = extract_template_id(&opt_xml)
            .ok_or_else(|| "Could not find a template ID in the OPT file".to_string())?;
        let (url, auth) = overwrite_target(&profile, &template_id).ok_or_else(|| {
            "Overwriting an existing template is not supported for this server type".to_string()
        })?;
        (reqwest::Method::PUT, url, auth)
    } else {
        (
            reqwest::Method::POST,
            format!("{}/definition/template/adl1.4", base),
            profile.auth_method.clone(),
        )
    };

    let resp = send_instrumented(
        &app,
        &client,
        make_request(&client, method, &url, &auth)
            .header("Content-Type", "application/xml")
            .body(opt_xml),
    )
    .await?;

    if resp.is_success || resp.status == 201 || resp.status == 204 {
        Ok(format!(
            "Template {} successfully (HTTP {})",
            if overwrite { "overwritten" } else { "uploaded" },
            resp.status
        ))
    } else if overwrite {
        Err(ehrbase_overwrite_error(resp.status, &resp.body))
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
                let tag = e.name().as_ref().to_string();
                current_tag = tag.clone();

                if tag == "term_bindings" {
                    in_term_bindings = true;
                    // Extract terminology attribute
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == "terminology" {
                            current_terminology = attr.value.to_string();
                        }
                    }
                } else if in_term_bindings && tag == "items" {
                    in_items = true;
                    current_code.clear();
                    current_node_id.clear();
                    // Extract code attribute if present
                    for attr in e.attributes().flatten() {
                        let key = attr.key.as_ref().to_string();
                        if key == "code" {
                            current_node_id = attr.value.to_string();
                        }
                    }
                }
            }
            Ok(Event::Text(e)) if in_items && current_tag == "code_string" => {
                let text = e.xml10_content();
                current_code = unescape(&text).unwrap_or_default().into_owned();
            }
            Ok(Event::End(e)) => {
                let tag = e.name().as_ref().to_string();
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

#[cfg(test)]
mod extract_template_id_tests {
    use super::extract_template_id;

    #[test]
    fn extracts_template_id() {
        let xml = r#"<template xmlns="http://schemas.openehr.org/v1"><language><code_string>en</code_string></language><template_id><value>vital_signs.v1</value></template_id></template>"#;
        assert_eq!(extract_template_id(xml).as_deref(), Some("vital_signs.v1"));
    }

    #[test]
    fn extracts_namespace_prefixed_template_id() {
        let xml = r#"<o:template xmlns:o="http://schemas.openehr.org/v1"><o:template_id><o:value>ns.v1</o:value></o:template_id></o:template>"#;
        assert_eq!(extract_template_id(xml).as_deref(), Some("ns.v1"));
    }

    #[test]
    fn extracts_template_id_with_entities_cdata_and_whitespace() {
        let entity = "<template><template_id><value>A&amp;B</value></template_id></template>";
        assert_eq!(extract_template_id(entity).as_deref(), Some("A&B"));
        let numeric = "<template><template_id><value>&#65;BC</value></template_id></template>";
        assert_eq!(extract_template_id(numeric).as_deref(), Some("ABC"));
        let cdata =
            "<template><template_id><value><![CDATA[cd.v2]]></value></template_id></template>";
        assert_eq!(extract_template_id(cdata).as_deref(), Some("cd.v2"));
        let spaced =
            "<template><template_id>\n  <value>  s.v1 \n</value>\n</template_id></template>";
        assert_eq!(extract_template_id(spaced).as_deref(), Some("s.v1"));
    }

    #[test]
    fn ignores_template_id_lookalikes_in_comments_and_cdata() {
        let comment = "<!-- <template_id><value>example.v1</value></template_id> --><template><template_id><value>live.v1</value></template_id></template>";
        assert_eq!(extract_template_id(comment).as_deref(), Some("live.v1"));
        let cdata = "<template><language><code_string><![CDATA[<template_id><value>example.v1</value></template_id>]]></code_string></language><template_id><value>live.v1</value></template_id></template>";
        assert_eq!(extract_template_id(cdata).as_deref(), Some("live.v1"));
    }

    #[test]
    fn ehrbase_admin_template_url_sits_beside_the_openehr_root() {
        assert_eq!(
            ehrbase_admin_template_url("http://localhost:8080/ehrbase/", "vital signs.v1"),
            "http://localhost:8080/ehrbase/rest/admin/template/vital%20signs.v1"
        );
    }

    #[test]
    fn ehrbase_overwrite_errors_explain_the_admin_api_statuses() {
        assert!(ehrbase_overwrite_error(403, "x").contains("admin role"));
        assert!(ehrbase_overwrite_error(401, "x").contains("credentials"));
        assert!(ehrbase_overwrite_error(404, "x").contains("Admin API is not enabled"));
        assert!(ehrbase_overwrite_error(422, "x").contains("used by at least one composition"));
        assert_eq!(
            ehrbase_overwrite_error(500, "boom"),
            "Overwrite failed (HTTP 500): boom"
        );
    }

    #[test]
    fn returns_none_without_template_id() {
        assert_eq!(extract_template_id("<template></template>"), None);
    }
}
