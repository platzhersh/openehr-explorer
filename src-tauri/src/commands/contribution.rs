use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::server::{create_client, get_profile_by_id, make_request};
use crate::inspector::send_instrumented;

/// A single VERSION referenced by a CONTRIBUTION — i.e. one of the objects
/// (COMPOSITION, EHR_STATUS, ...) that were committed together as part of
/// this contribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionVersionRef {
    /// The OBJECT_VERSION_ID of the referenced version, e.g.
    /// `8849182c-...::local.ehrbase.org::1`.
    pub id: String,
    /// The RM type of the versioned object (COMPOSITION, EHR_STATUS, ...),
    /// when the server includes it.
    pub version_type: Option<String>,
}

/// The AUDIT_DETAILS attached to a CONTRIBUTION — who committed it, when,
/// and why.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionAudit {
    pub system_id: Option<String>,
    pub committer_name: Option<String>,
    pub time_committed: Option<String>,
    pub change_type: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionDetail {
    pub contribution_uid: String,
    pub audit: Option<ContributionAudit>,
    pub versions: Vec<ContributionVersionRef>,
}

/// Fetch a CONTRIBUTION by UID — the openEHR audit-trail record of a single
/// commit, showing who/what changed and which versions were part of it.
///
/// See PRD/OEH-28. Standard openEHR REST API endpoint:
/// `GET /ehr/{ehr_id}/contribution/{contribution_uid}`.
#[tauri::command]
pub async fn get_contribution(
    app: tauri::AppHandle,
    server_id: String,
    ehr_id: String,
    contribution_uid: String,
) -> Result<ContributionDetail, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    let url = format!(
        "{}/rest/openehr/v1/ehr/{}/contribution/{}",
        base, ehr_id, contribution_uid
    );

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
        .map_err(|e| format!("Failed to parse contribution: {}", e))?;

    Ok(parse_contribution(&body))
}

fn parse_contribution(body: &Value) -> ContributionDetail {
    let contribution_uid = body
        .get("uid")
        .and_then(|u| u.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let audit = body.get("audit").map(|audit| ContributionAudit {
        system_id: audit
            .get("system_id")
            .and_then(|v| v.as_str())
            .map(String::from),
        committer_name: audit
            .get("committer")
            .and_then(|c| c.get("name"))
            .and_then(|v| v.as_str())
            .map(String::from),
        time_committed: audit
            .get("time_committed")
            .and_then(|t| t.get("value"))
            .and_then(|v| v.as_str())
            .map(String::from),
        change_type: audit
            .get("change_type")
            .and_then(|c| c.get("value"))
            .and_then(|v| v.as_str())
            .map(String::from),
        description: audit
            .get("description")
            .and_then(|d| d.get("value"))
            .and_then(|v| v.as_str())
            .map(String::from),
    });

    let versions = body
        .get("versions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|v| ContributionVersionRef {
                    id: v
                        .get("id")
                        .and_then(|i| i.get("value"))
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string(),
                    version_type: v.get("type").and_then(|t| t.as_str()).map(String::from),
                })
                .collect()
        })
        .unwrap_or_default();

    ContributionDetail {
        contribution_uid,
        audit,
        versions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_contribution_full() {
        let body = serde_json::json!({
            "uid": {"value": "9c14c616-1234::local.ehrbase.org::1"},
            "versions": [
                {"id": {"value": "8849182c-1::local.ehrbase.org::1"}, "type": "COMPOSITION"},
                {"id": {"value": "ab12cd34-1::local.ehrbase.org::1"}, "type": "EHR_STATUS"}
            ],
            "audit": {
                "system_id": "local.ehrbase.org",
                "committer": {"name": "Silvia"},
                "time_committed": {"value": "2026-08-25T10:00:00.000Z"},
                "change_type": {"value": "creation"},
                "description": {"value": "Initial commit"}
            }
        });

        let parsed = parse_contribution(&body);
        assert_eq!(
            parsed.contribution_uid,
            "9c14c616-1234::local.ehrbase.org::1"
        );
        assert_eq!(parsed.versions.len(), 2);
        assert_eq!(parsed.versions[0].id, "8849182c-1::local.ehrbase.org::1");
        assert_eq!(
            parsed.versions[0].version_type.as_deref(),
            Some("COMPOSITION")
        );
        assert_eq!(
            parsed.versions[1].version_type.as_deref(),
            Some("EHR_STATUS")
        );

        let audit = parsed.audit.expect("audit should be present");
        assert_eq!(audit.committer_name.as_deref(), Some("Silvia"));
        assert_eq!(audit.change_type.as_deref(), Some("creation"));
        assert_eq!(audit.description.as_deref(), Some("Initial commit"));
        assert_eq!(audit.system_id.as_deref(), Some("local.ehrbase.org"));
    }

    #[test]
    fn test_parse_contribution_missing_audit_and_versions() {
        let body = serde_json::json!({
            "uid": {"value": "uid-1"},
        });
        let parsed = parse_contribution(&body);
        assert_eq!(parsed.contribution_uid, "uid-1");
        assert!(parsed.audit.is_none());
        assert!(parsed.versions.is_empty());
    }

    #[test]
    fn test_parse_contribution_empty_body() {
        let body = serde_json::json!({});
        let parsed = parse_contribution(&body);
        assert_eq!(parsed.contribution_uid, "");
        assert!(parsed.audit.is_none());
        assert!(parsed.versions.is_empty());
    }
}
