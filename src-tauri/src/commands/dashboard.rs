use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;

use super::query::execute_aql;
use super::server::get_profile_by_id;
use super::template::list_templates;

/// Live totals shown on the landing dashboard (OEH-17). Fetched fresh on
/// every call — the frontend decides when to refresh, nothing here is cached.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardCounts {
    pub ehr_count: u64,
    pub composition_count: u64,
    pub template_count: u64,
}

#[tauri::command]
pub async fn get_dashboard_counts(
    app: AppHandle,
    server_id: String,
) -> Result<DashboardCounts, String> {
    // Fail fast with a clear error if the profile doesn't exist, rather than
    // three separate "unknown server" errors from the calls below.
    get_profile_by_id(&server_id)?;

    let ehr_count = aql_count(&app, &server_id, "SELECT COUNT(e) FROM EHR e").await?;
    let composition_count = aql_count(
        &app,
        &server_id,
        "SELECT COUNT(c) FROM EHR e CONTAINS COMPOSITION c",
    )
    .await?;
    let template_count = list_templates(app.clone(), server_id.clone()).await?.len() as u64;

    Ok(DashboardCounts {
        ehr_count,
        composition_count,
        template_count,
    })
}

/// Runs an AQL `COUNT(...)` query and extracts the scalar result. A COUNT
/// query returns exactly one row with one numeric column — unlike a regular
/// SELECT, `AqlResult::total_count` (the row count) would just be `1` here,
/// so the cell value itself has to be read out.
async fn aql_count(app: &AppHandle, server_id: &str, aql: &str) -> Result<u64, String> {
    let result = execute_aql(app.clone(), server_id.to_string(), aql.to_string()).await?;
    let cell = result
        .rows
        .first()
        .and_then(|row| row.first())
        .ok_or_else(|| format!("COUNT query returned no rows: {}", aql))?;

    value_as_u64(cell).ok_or_else(|| format!("COUNT query returned a non-numeric value: {}", cell))
}

fn value_as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(n) => n.as_u64().or_else(|| n.as_f64().map(|f| f as u64)),
        Value::String(s) => s.parse::<u64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_as_u64_integer() {
        assert_eq!(value_as_u64(&serde_json::json!(42)), Some(42));
    }

    #[test]
    fn test_value_as_u64_float() {
        // Some CDRs return COUNT() as a JSON float (e.g. 3.0) rather than an
        // integer — still a valid whole count.
        assert_eq!(value_as_u64(&serde_json::json!(3.0)), Some(3));
    }

    #[test]
    fn test_value_as_u64_numeric_string() {
        assert_eq!(value_as_u64(&serde_json::json!("17")), Some(17));
    }

    #[test]
    fn test_value_as_u64_rejects_non_numeric() {
        assert_eq!(value_as_u64(&serde_json::json!("not a number")), None);
        assert_eq!(value_as_u64(&serde_json::json!(null)), None);
        assert_eq!(value_as_u64(&serde_json::json!({"a": 1})), None);
    }
}
