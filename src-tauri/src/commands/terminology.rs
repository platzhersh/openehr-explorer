use serde::Deserialize;
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

#[derive(Debug, Deserialize)]
struct FhirLookupResponse {
    parameter: Option<Vec<FhirParameter>>,
}

#[derive(Debug, Deserialize)]
struct FhirParameter {
    name: Option<String>,
    #[serde(rename = "valueString")]
    value_string: Option<String>,
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

    // Make the request via instrumented client (logs to Request Inspector)
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let resp = match send_instrumented(&app, &client, client.get(&url)).await {
        Ok(resp) => resp,
        Err(_) => return Ok(None), // Graceful degradation on network error
    };

    if !resp.is_success {
        return Ok(None); // Graceful degradation on 404/error
    }

    let body: FhirLookupResponse = match serde_json::from_str(&resp.body) {
        Ok(body) => body,
        Err(_) => return Ok(None),
    };

    // Extract display name from FHIR Parameters response
    let display = body.parameter.as_ref().and_then(|params| {
        params
            .iter()
            .find(|p| p.name.as_deref() == Some("display"))
            .and_then(|p| p.value_string.clone())
    });

    // Cache the result if found
    if let Some(ref display_val) = display {
        if let Ok(mut cache_map) = cache.cache.lock() {
            cache_map.insert((system, code), display_val.clone());
        }
    }

    Ok(display)
}
