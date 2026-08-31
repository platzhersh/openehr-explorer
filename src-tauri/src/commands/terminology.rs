use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

use super::server::get_profile_by_id;
use crate::inspector::send_instrumented;
use crate::settings::{effective_terminology_url, load_settings};

/// In-memory cache for terminology lookups: (system, code) -> display term
pub struct TerminologyCache {
    pub cache: Mutex<HashMap<(String, String), String>>,
}

impl Default for TerminologyCache {
    fn default() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }
}

/// Maps common terminology identifiers to their FHIR CodeSystem URIs
fn terminology_to_fhir_system(terminology: &str) -> Option<&'static str> {
    match terminology.to_uppercase().as_str() {
        "SNOMED-CT" | "SNOMED" | "SCT" => Some("http://snomed.info/sct"),
        "LOINC" => Some("http://loinc.org"),
        "ICD-10" | "ICD10" => Some("http://hl7.org/fhir/sid/icd-10"),
        "ICD-11" | "ICD11" => Some("http://id.who.int/icd/release/11/mms"),
        "ATC" => Some("http://www.whocc.no/atc"),
        _ => None,
    }
}

/// A single `Parameters.parameter` entry from a FHIR terminology response.
/// Covers the value types + nested `part` entries actually used by
/// `$lookup`/`$validate-code`/`$subsumes` — `designation` and `property`
/// parameters (from `$lookup`) carry their own `value`/`code` as `part`.
#[derive(Debug, Deserialize)]
struct FhirParameter {
    name: Option<String>,
    #[serde(rename = "valueString")]
    value_string: Option<String>,
    #[serde(rename = "valueBoolean")]
    value_boolean: Option<bool>,
    #[serde(rename = "valueCode")]
    value_code: Option<String>,
    part: Option<Vec<FhirParameter>>,
}

#[derive(Debug, Deserialize)]
struct FhirParametersResponse {
    parameter: Option<Vec<FhirParameter>>,
}

/// Reads a named parameter's scalar value, trying string/code/boolean in
/// turn — the three value kinds this module's operations actually return.
fn find_param_value(params: &[FhirParameter], name: &str) -> Option<String> {
    params
        .iter()
        .find(|p| p.name.as_deref() == Some(name))
        .and_then(|p| {
            p.value_string
                .clone()
                .or_else(|| p.value_code.clone())
                .or_else(|| p.value_boolean.map(|b| b.to_string()))
        })
}

fn find_param_bool(params: &[FhirParameter], name: &str) -> Option<bool> {
    params
        .iter()
        .find(|p| p.name.as_deref() == Some(name))
        .and_then(|p| p.value_boolean)
}

/// Resolves the effective terminology server base URL for a profile, as an
/// explicit error rather than `None` — unlike `lookup_code`'s graceful
/// degradation (used for passive/lazy resolution), the Terminology Browser's
/// operations are user-triggered "run this query" actions, so a clear reason
/// belongs in the result rather than a silently empty one.
fn terminology_base_url(server_id: &str) -> Result<String, String> {
    let profile = get_profile_by_id(server_id)?;
    let settings = load_settings();
    effective_terminology_url(&profile, &settings).ok_or_else(|| {
        "No terminology server configured. Set one in Settings, or on this server profile."
            .to_string()
    })
}

fn terminology_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default()
}

#[tauri::command]
pub async fn lookup_code(
    app: tauri::AppHandle,
    server_id: String,
    system: String,
    code: String,
    cache: tauri::State<'_, TerminologyCache>,
) -> Result<Option<String>, String> {
    // Check cache first
    {
        let cache_map = cache.cache.lock().map_err(|e| e.to_string())?;
        if let Some(display) = cache_map.get(&(system.clone(), code.clone())) {
            return Ok(Some(display.clone()));
        }
    }

    // Resolve effective terminology URL
    let profile = get_profile_by_id(&server_id)?;
    let settings = load_settings();
    let base_url = match effective_terminology_url(&profile, &settings) {
        Some(url) => url,
        None => return Ok(None), // No terminology server configured
    };

    // Map terminology name to FHIR system URI
    let fhir_system = terminology_to_fhir_system(&system).unwrap_or(&system);

    // Build the $lookup URL
    let url = format!(
        "{}/CodeSystem/$lookup?system={}&code={}",
        base_url.trim_end_matches('/'),
        urlencoding::encode(fhir_system),
        urlencoding::encode(&code)
    );

    let client = terminology_client();

    let resp = match send_instrumented(&app, &client, client.get(&url)).await {
        Ok(resp) => resp,
        Err(_) => return Ok(None), // Graceful degradation on network error
    };

    if !resp.is_success {
        return Ok(None); // Graceful degradation on 404/error
    }

    let body: FhirParametersResponse = match serde_json::from_str(&resp.body) {
        Ok(body) => body,
        Err(_) => return Ok(None),
    };

    let display = body
        .parameter
        .as_ref()
        .and_then(|params| find_param_value(params, "display"));

    // Cache the result if found
    if let Some(ref display_val) = display {
        if let Ok(mut cache_map) = cache.cache.lock() {
            cache_map.insert((system, code), display_val.clone());
        }
    }

    Ok(display)
}

/// A `property` returned by `CodeSystem/$lookup` — e.g. `parent`,
/// `inactive`, `normalForm` — as a flat code/value pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminologyProperty {
    pub code: String,
    pub value: String,
}

/// Full detail for a single code, as returned by `CodeSystem/$lookup` —
/// richer than `lookup_code`'s bare display string, for the standalone
/// "Describe a code" tool in the Terminology Browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeDescription {
    pub system: String,
    pub code: String,
    pub display: Option<String>,
    pub designations: Vec<String>,
    pub properties: Vec<TerminologyProperty>,
}

fn parse_code_description(body: &str, system: &str, code: &str) -> Result<CodeDescription, String> {
    let parsed: FhirParametersResponse =
        serde_json::from_str(body).map_err(|e| format!("Failed to parse $lookup response: {e}"))?;
    let params = parsed.parameter.unwrap_or_default();

    let display = find_param_value(&params, "display");

    let mut designations = Vec::new();
    let mut properties = Vec::new();
    for p in &params {
        match (p.name.as_deref(), &p.part) {
            (Some("designation"), Some(parts)) => {
                if let Some(value) = find_param_value(parts, "value") {
                    designations.push(value);
                }
            }
            (Some("property"), Some(parts)) => {
                let prop_code = find_param_value(parts, "code");
                let prop_value = find_param_value(parts, "value");
                if let (Some(prop_code), Some(prop_value)) = (prop_code, prop_value) {
                    properties.push(TerminologyProperty {
                        code: prop_code,
                        value: prop_value,
                    });
                }
            }
            _ => {}
        }
    }

    Ok(CodeDescription {
        system: system.to_string(),
        code: code.to_string(),
        display,
        designations,
        properties,
    })
}

/// Describe a single code via `CodeSystem/$lookup` — preferred term, plus
/// any designations (synonyms/translations) and properties (e.g. `parent`,
/// `inactive`) the terminology server reports. Unlike `lookup_code`, this is
/// a user-triggered action, so failures are surfaced rather than degraded
/// away.
#[tauri::command]
pub async fn describe_code(
    app: tauri::AppHandle,
    server_id: String,
    system: String,
    code: String,
) -> Result<CodeDescription, String> {
    let base_url = terminology_base_url(&server_id)?;
    let fhir_system = terminology_to_fhir_system(&system)
        .unwrap_or(&system)
        .to_string();

    let url = format!(
        "{}/CodeSystem/$lookup?system={}&code={}",
        base_url.trim_end_matches('/'),
        urlencoding::encode(&fhir_system),
        urlencoding::encode(&code)
    );

    let client = terminology_client();
    let resp = send_instrumented(&app, &client, client.get(&url)).await?;
    if !resp.is_success {
        return Err(format!(
            "Terminology server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    parse_code_description(&resp.body, &fhir_system, &code)
}

/// One member of an expanded value set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminologyConcept {
    pub system: Option<String>,
    pub code: Option<String>,
    pub display: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueSetExpansion {
    pub total: Option<i64>,
    pub concepts: Vec<TerminologyConcept>,
}

#[derive(Debug, Deserialize)]
struct FhirValueSet {
    expansion: Option<FhirExpansion>,
}

#[derive(Debug, Deserialize)]
struct FhirExpansion {
    total: Option<i64>,
    contains: Option<Vec<FhirExpansionContains>>,
}

#[derive(Debug, Deserialize)]
struct FhirExpansionContains {
    system: Option<String>,
    code: Option<String>,
    display: Option<String>,
}

fn parse_expansion(body: &str) -> Result<ValueSetExpansion, String> {
    let parsed: FhirValueSet =
        serde_json::from_str(body).map_err(|e| format!("Failed to parse $expand response: {e}"))?;
    let expansion = parsed.expansion.unwrap_or(FhirExpansion {
        total: Some(0),
        contains: None,
    });
    let concepts = expansion
        .contains
        .unwrap_or_default()
        .into_iter()
        .map(|c| TerminologyConcept {
            system: c.system,
            code: c.code,
            display: c.display,
        })
        .collect();
    Ok(ValueSetExpansion {
        total: expansion.total,
        concepts,
    })
}

/// Expand a value set via `ValueSet/$expand` — `value_set` is either a
/// canonical URL (e.g. bound to a template node's term binding) or a raw
/// FHIR ValueSet id, and `filter` narrows by display text (the operation's
/// `filter` parameter) for large value sets. `count` caps how many concepts
/// come back (servers default this low — 20 on some public test servers).
#[tauri::command]
pub async fn expand_valueset(
    app: tauri::AppHandle,
    server_id: String,
    value_set: String,
    filter: Option<String>,
    count: Option<u32>,
) -> Result<ValueSetExpansion, String> {
    let base_url = terminology_base_url(&server_id)?;

    let mut url = format!(
        "{}/ValueSet/$expand?url={}",
        base_url.trim_end_matches('/'),
        urlencoding::encode(&value_set)
    );
    if let Some(filter) = filter.as_deref().filter(|f| !f.is_empty()) {
        url.push_str(&format!("&filter={}", urlencoding::encode(filter)));
    }
    url.push_str(&format!("&count={}", count.unwrap_or(100)));

    let client = terminology_client();
    let resp = send_instrumented(&app, &client, client.get(&url)).await?;
    if !resp.is_success {
        return Err(format!(
            "Terminology server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    parse_expansion(&resp.body)
}

/// Result of a `$validate-code` membership test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeValidation {
    pub result: bool,
    pub message: Option<String>,
    pub display: Option<String>,
}

fn parse_validation(body: &str) -> Result<CodeValidation, String> {
    let parsed: FhirParametersResponse = serde_json::from_str(body)
        .map_err(|e| format!("Failed to parse $validate-code response: {e}"))?;
    let params = parsed.parameter.unwrap_or_default();
    Ok(CodeValidation {
        result: find_param_bool(&params, "result").unwrap_or(false),
        message: find_param_value(&params, "message"),
        display: find_param_value(&params, "display"),
    })
}

/// Test whether a code is a member of a value set (`ValueSet/$validate-code`
/// when `value_set` is given) or simply valid in a code system
/// (`CodeSystem/$validate-code` otherwise).
#[tauri::command]
pub async fn validate_code(
    app: tauri::AppHandle,
    server_id: String,
    system: String,
    code: String,
    value_set: Option<String>,
) -> Result<CodeValidation, String> {
    let base_url = terminology_base_url(&server_id)?;
    let fhir_system = terminology_to_fhir_system(&system)
        .unwrap_or(&system)
        .to_string();
    let base = base_url.trim_end_matches('/');

    let url = match value_set.as_deref().filter(|v| !v.is_empty()) {
        Some(value_set) => format!(
            "{}/ValueSet/$validate-code?url={}&system={}&code={}",
            base,
            urlencoding::encode(value_set),
            urlencoding::encode(&fhir_system),
            urlencoding::encode(&code)
        ),
        None => format!(
            "{}/CodeSystem/$validate-code?url={}&code={}",
            base,
            urlencoding::encode(&fhir_system),
            urlencoding::encode(&code)
        ),
    };

    let client = terminology_client();
    let resp = send_instrumented(&app, &client, client.get(&url)).await?;
    if !resp.is_success {
        return Err(format!(
            "Terminology server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    parse_validation(&resp.body)
}

/// Outcome of a `CodeSystem/$subsumes` hierarchy test between two codes in
/// the same code system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsumptionResult {
    pub outcome: String,
}

fn parse_subsumption(body: &str) -> Result<SubsumptionResult, String> {
    let parsed: FhirParametersResponse = serde_json::from_str(body)
        .map_err(|e| format!("Failed to parse $subsumes response: {e}"))?;
    let params = parsed.parameter.unwrap_or_default();
    let outcome = find_param_value(&params, "outcome").unwrap_or_else(|| "unknown".to_string());
    Ok(SubsumptionResult { outcome })
}

/// Test the subsumption relationship between two codes in the same code
/// system via `CodeSystem/$subsumes` — one of `equivalent`, `subsumes`,
/// `subsumed-by`, or `not-subsumed`.
#[tauri::command]
pub async fn test_subsumption(
    app: tauri::AppHandle,
    server_id: String,
    system: String,
    code_a: String,
    code_b: String,
) -> Result<SubsumptionResult, String> {
    let base_url = terminology_base_url(&server_id)?;
    let fhir_system = terminology_to_fhir_system(&system)
        .unwrap_or(&system)
        .to_string();

    let url = format!(
        "{}/CodeSystem/$subsumes?system={}&codeA={}&codeB={}",
        base_url.trim_end_matches('/'),
        urlencoding::encode(&fhir_system),
        urlencoding::encode(&code_a),
        urlencoding::encode(&code_b)
    );

    let client = terminology_client();
    let resp = send_instrumented(&app, &client, client.get(&url)).await?;
    if !resp.is_success {
        return Err(format!(
            "Terminology server returned HTTP {}: {}",
            resp.status, resp.body
        ));
    }

    parse_subsumption(&resp.body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_terminology_names_to_fhir_systems() {
        assert_eq!(
            terminology_to_fhir_system("SNOMED-CT"),
            Some("http://snomed.info/sct")
        );
        assert_eq!(
            terminology_to_fhir_system("loinc"),
            Some("http://loinc.org")
        );
        assert_eq!(terminology_to_fhir_system("local"), None);
    }

    #[test]
    fn parses_lookup_response_into_code_description() {
        let body = r#"{
            "resourceType": "Parameters",
            "parameter": [
                {"name": "name", "valueString": "SNOMED CT"},
                {"name": "display", "valueString": "Septicaemia (disorder)"},
                {"name": "designation", "part": [
                    {"name": "value", "valueString": "Sepsis"}
                ]},
                {"name": "property", "part": [
                    {"name": "code", "valueCode": "inactive"},
                    {"name": "value", "valueBoolean": false}
                ]}
            ]
        }"#;
        let desc = parse_code_description(body, "http://snomed.info/sct", "91302008").unwrap();
        assert_eq!(desc.display.as_deref(), Some("Septicaemia (disorder)"));
        assert_eq!(desc.designations, vec!["Sepsis".to_string()]);
        assert_eq!(desc.properties.len(), 1);
        assert_eq!(desc.properties[0].code, "inactive");
        assert_eq!(desc.properties[0].value, "false");
    }

    #[test]
    fn parses_expand_response_into_concepts() {
        let body = r#"{
            "resourceType": "ValueSet",
            "expansion": {
                "total": 2,
                "contains": [
                    {"system": "http://snomed.info/sct", "code": "271737000", "display": "Anaemia"},
                    {"system": "http://snomed.info/sct", "code": "10725009", "display": "Fever"}
                ]
            }
        }"#;
        let expansion = parse_expansion(body).unwrap();
        assert_eq!(expansion.total, Some(2));
        assert_eq!(expansion.concepts.len(), 2);
        assert_eq!(expansion.concepts[0].code.as_deref(), Some("271737000"));
    }

    #[test]
    fn parses_expand_response_with_no_expansion_as_empty() {
        let body = r#"{"resourceType": "ValueSet"}"#;
        let expansion = parse_expansion(body).unwrap();
        assert_eq!(expansion.concepts.len(), 0);
    }

    #[test]
    fn parses_validate_code_response() {
        let body = r#"{
            "resourceType": "Parameters",
            "parameter": [
                {"name": "result", "valueBoolean": true},
                {"name": "display", "valueString": "Fever"}
            ]
        }"#;
        let validation = parse_validation(body).unwrap();
        assert!(validation.result);
        assert_eq!(validation.display.as_deref(), Some("Fever"));
        assert_eq!(validation.message, None);
    }

    #[test]
    fn parses_validate_code_failure_response() {
        let body = r#"{
            "resourceType": "Parameters",
            "parameter": [
                {"name": "result", "valueBoolean": false},
                {"name": "message", "valueString": "Code not found in value set"}
            ]
        }"#;
        let validation = parse_validation(body).unwrap();
        assert!(!validation.result);
        assert_eq!(
            validation.message.as_deref(),
            Some("Code not found in value set")
        );
    }

    #[test]
    fn parses_subsumption_response() {
        let body = r#"{
            "resourceType": "Parameters",
            "parameter": [{"name": "outcome", "valueCode": "subsumes"}]
        }"#;
        let result = parse_subsumption(body).unwrap();
        assert_eq!(result.outcome, "subsumes");
    }
}
