//! EHR search and sort, ported from the Tauri app:
//!  * the search box is a tiny query language (`EhrBrowser.vue`: `parseSearchInput`/`applyToken`),
//!  * the AQL it turns into (`src-tauri/src/commands/ehr.rs`: `build_ehr_search_aql_paged`, `build_ehr_list_aql`).
//!
//! Everything here is pure so it can be unit-tested against the same expectations as the Tauri code.

/// Result cap shared by every search dimension (`EHR_SEARCH_DISPLAY_LIMIT`).
pub const SEARCH_DISPLAY_LIMIT: usize = 200;
/// Page size / total scan cap when scanning the CDR for a *partial* EHR id prefix.
pub const ID_SCAN_CHUNK: usize = 200;
pub const ID_SCAN_CAP: usize = 2000;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Criteria {
    pub ehr_id_prefix: Option<String>,
    pub subject_id: Option<String>,
    pub subject_namespace: Option<String>,
    pub system_id: Option<String>,
    pub modifiable: Option<bool>,
    pub has_compositions: Option<bool>,
    pub has_directory: Option<bool>,
    pub created_on: Option<String>,
    pub created_before: Option<String>,
    pub created_after: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    TimeCreated,
    EhrId,
    SystemId,
}

impl SortField {
    pub const ALL: [SortField; 3] = [SortField::TimeCreated, SortField::EhrId, SortField::SystemId];

    pub fn label(self) -> &'static str {
        match self {
            SortField::TimeCreated => "Created date",
            SortField::EhrId => "EHR ID",
            SortField::SystemId => "System ID",
        }
    }

    fn path(self) -> &'static str {
        match self {
            SortField::TimeCreated => "e/time_created/value",
            SortField::EhrId => "e/ehr_id/value",
            SortField::SystemId => "e/system_id/value",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDir {
    Asc,
    Desc,
}

impl SortDir {
    pub fn toggled(self) -> Self {
        match self {
            SortDir::Asc => SortDir::Desc,
            SortDir::Desc => SortDir::Asc,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            SortDir::Asc => "↑ Asc",
            SortDir::Desc => "↓ Desc",
        }
    }
    fn sql(self) -> &'static str {
        match self {
            SortDir::Asc => "ASC",
            SortDir::Desc => "DESC",
        }
    }
}

// ---------------------------------------------------------------------------
// List + sort
// ---------------------------------------------------------------------------

/// The paged EHR list with `ORDER BY`. `e/ehr_id/value ASC` is appended as a tiebreaker (unless the primary sort is
/// the id) so ties get a total order and pages never duplicate or skip rows.
pub fn build_list_aql(offset: usize, limit: usize, field: SortField, dir: SortDir) -> String {
    let tiebreaker = if field == SortField::EhrId { "" } else { ", e/ehr_id/value ASC" };
    format!(
        "SELECT e/ehr_id/value, e/time_created/value, e/system_id/value FROM EHR e ORDER BY {} {}{} LIMIT {} OFFSET {}",
        field.path(),
        dir.sql(),
        tiebreaker,
        limit,
        offset
    )
}

/// Same query without `ORDER BY`: the fallback when the CDR rejects sorting on EHR-level attributes (EHRBase does).
pub fn build_list_aql_unsorted(offset: usize, limit: usize) -> String {
    format!("SELECT e/ehr_id/value, e/time_created/value, e/system_id/value FROM EHR e LIMIT {limit} OFFSET {offset}")
}

/// Detects the specific "ORDER BY on an EHR-level attribute isn't implemented" error (kept narrow like the Tauri
/// check, so unrelated 400s still surface).
pub fn is_order_by_unsupported(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.contains("order_by") || lower.contains("order by")
}

// ---------------------------------------------------------------------------
// Search input parsing (EhrBrowser.vue)
// ---------------------------------------------------------------------------

fn is_date(v: &str) -> bool {
    let b = v.as_bytes();
    b.len() == 10 && b.iter().enumerate().all(|(i, c)| if i == 4 || i == 7 { *c == b'-' } else { c.is_ascii_digit() })
}

fn bool_token(slot: &mut Option<bool>, prefix: &str, value: &str) -> Result<(), String> {
    match value {
        "true" => *slot = Some(true),
        "false" => *slot = Some(false),
        _ => return Err(format!("{prefix}: expects 'true' or 'false'.")),
    }
    Ok(())
}

fn date_token(slot: &mut Option<String>, prefix: &str, value: &str) -> Result<(), String> {
    if !is_date(value) {
        return Err(format!("{prefix}: expects a date in YYYY-MM-DD format (e.g. 2026-03-12)."));
    }
    *slot = Some(value.to_string());
    Ok(())
}

fn extend_prefix(c: &mut Criteria, token: &str) {
    let mut s = c.ehr_id_prefix.take().unwrap_or_default();
    s.push_str(token);
    c.ehr_id_prefix = Some(s);
}

/// One whitespace-separated token. A bare token, or an unknown `key:`, extends the EHR id prefix.
fn apply_token(c: &mut Criteria, token: &str) -> Result<(), String> {
    let Some((prefix, value)) = token.split_once(':') else {
        extend_prefix(c, token);
        return Ok(());
    };
    if value.is_empty() {
        return Err(format!("{prefix}: value cannot be empty."));
    }
    match prefix {
        "subject" => c.subject_id = Some(value.to_string()),
        "namespace" => c.subject_namespace = Some(value.to_string()),
        "system" => c.system_id = Some(value.to_string()),
        "modifiable" => return bool_token(&mut c.modifiable, "modifiable", value),
        "hasCompositions" => return bool_token(&mut c.has_compositions, "hasCompositions", value),
        "hasDirectory" => return bool_token(&mut c.has_directory, "hasDirectory", value),
        "created-on" => return date_token(&mut c.created_on, "created-on", value),
        "created-before" => return date_token(&mut c.created_before, "created-before", value),
        "created-after" => return date_token(&mut c.created_after, "created-after", value),
        _ => extend_prefix(c, token), // unknown prefix: safe fallback, as in the Tauri UI
    }
    Ok(())
}

/// Parse the search box. `Ok(None)` = empty input. The second tuple item is a warning to show (not an error).
pub fn parse_input(raw: &str) -> Result<Option<(Criteria, Option<String>)>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let mut c = Criteria::default();
    for token in trimmed.split_whitespace() {
        apply_token(&mut c, token)?;
    }
    let mut warning = None;
    if c.created_on.is_some() && (c.created_before.is_some() || c.created_after.is_some()) {
        warning = Some("created-on overrides created-before/created-after. Only created-on is used.".to_string());
        c.created_before = None;
        c.created_after = None;
    }
    Ok(Some((c, warning)))
}

/// Chip labels for the active criteria (same wording as the Tauri UI).
pub fn chips(c: &Criteria) -> Vec<String> {
    let yn = |b: bool| if b { "Yes" } else { "No" };
    let mut out = Vec::new();
    if let Some(v) = &c.ehr_id_prefix {
        out.push(format!("ID starts with \"{v}\""));
    }
    if let Some(v) = &c.subject_id {
        out.push(format!("Subject contains \"{v}\""));
    }
    if let Some(v) = &c.subject_namespace {
        out.push(format!("Namespace: {v}"));
    }
    if let Some(v) = &c.system_id {
        out.push(format!("System: {v}"));
    }
    if let Some(v) = c.modifiable {
        out.push(format!("Modifiable: {}", yn(v)));
    }
    if let Some(v) = c.has_compositions {
        out.push(format!("Has compositions: {}", yn(v)));
    }
    if let Some(v) = c.has_directory {
        out.push(format!("Has directory entries: {}", yn(v)));
    }
    if let Some(v) = &c.created_on {
        out.push(format!("Created on {v}"));
    }
    if let Some(v) = &c.created_before {
        out.push(format!("Created before {v}"));
    }
    if let Some(v) = &c.created_after {
        out.push(format!("Created after {v}"));
    }
    out
}

// ---------------------------------------------------------------------------
// Search AQL (ehr.rs: build_ehr_search_aql_paged)
// ---------------------------------------------------------------------------

pub fn escape_aql_string(s: &str) -> String {
    s.replace('\'', "''")
}

/// A *full* EHR id (a UUID) can use equality; a partial prefix has no working predicate and is scanned client-side.
pub fn is_full_ehr_id(s: &str) -> bool {
    uuid::Uuid::parse_str(s.trim()).is_ok()
}

pub fn has_partial_prefix(c: &Criteria) -> Option<&str> {
    c.ehr_id_prefix.as_deref().filter(|p| !is_full_ehr_id(p))
}

pub fn build_search_aql(c: &Criteria, limit: usize, offset: usize) -> Result<String, String> {
    let with_compositions = c.has_compositions == Some(true);
    let (base, status) = if with_compositions {
        (
            "SELECT DISTINCT e/ehr_id/value, e/time_created/value, e/ehr_status/subject/external_ref/id/value AS subject_id, \
             e/ehr_status/subject/external_ref/namespace AS subject_namespace, e/ehr_status/is_modifiable AS modifiable, \
             e/ehr_status/is_queryable AS queryable, e/system_id/value AS system_id FROM EHR e CONTAINS COMPOSITION c",
            "e/ehr_status",
        )
    } else {
        (
            "SELECT e/ehr_id/value, e/time_created/value, s/subject/external_ref/id/value AS subject_id, \
             s/subject/external_ref/namespace AS subject_namespace, s/is_modifiable AS modifiable, \
             s/is_queryable AS queryable, e/system_id/value AS system_id FROM EHR e CONTAINS EHR_STATUS s",
            "s",
        )
    };

    let mut predicates: Vec<String> = Vec::new();
    if let Some(prefix) = &c.ehr_id_prefix {
        if let Ok(id) = uuid::Uuid::parse_str(prefix.trim()) {
            predicates.push(format!("e/ehr_id/value = '{id}'"));
        }
    }
    if let Some(v) = &c.subject_id {
        predicates.push(format!("{status}/subject/external_ref/id/value LIKE '%{}%'", escape_aql_string(v)));
    }
    if let Some(v) = &c.subject_namespace {
        predicates.push(format!("{status}/subject/external_ref/namespace = '{}'", escape_aql_string(v)));
    }
    if let Some(v) = &c.system_id {
        predicates.push(format!("e/system_id/value = '{}'", escape_aql_string(v)));
    }
    if let Some(m) = c.modifiable {
        predicates.push(format!("{status}/is_modifiable = {m}"));
    }
    if c.has_compositions == Some(false) {
        return Err("hasCompositions:false is not currently supported due to AQL limitations. To find EHRs with compositions, use hasCompositions:true instead.".to_string());
    }
    if c.created_on.is_some() || c.created_before.is_some() || c.created_after.is_some() {
        return Err("Date filters (created-on, created-before, created-after) are not currently supported due to EHRBase limitations. EHR-level attributes like time_created cannot be used in WHERE clauses. Use the paginated list view and sort by creation date instead.".to_string());
    }
    if predicates.is_empty() && !with_compositions && c.has_directory.is_none() && c.ehr_id_prefix.is_none() {
        return Err("At least one search criterion must be provided".to_string());
    }
    if predicates.is_empty() {
        Ok(format!("{base} LIMIT {limit} OFFSET {offset}"))
    } else {
        Ok(format!("{base} WHERE {} LIMIT {limit} OFFSET {offset}", predicates.join(" AND ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Criteria {
        parse_input(s).unwrap().unwrap().0
    }

    #[test]
    fn list_aql_defaults_and_tiebreaker() {
        let q = build_list_aql(0, 20, SortField::TimeCreated, SortDir::Desc);
        assert!(q.contains("ORDER BY e/time_created/value DESC, e/ehr_id/value ASC"));
        assert!(q.contains("LIMIT 20 OFFSET 0"));
        let q = build_list_aql(40, 20, SortField::TimeCreated, SortDir::Asc);
        assert!(q.contains("ORDER BY e/time_created/value ASC") && q.contains("LIMIT 20 OFFSET 40"));
        // sorting by id needs no tiebreaker
        let q = build_list_aql(0, 20, SortField::EhrId, SortDir::Desc);
        assert!(q.contains("ORDER BY e/ehr_id/value DESC LIMIT"));
        let q = build_list_aql(0, 20, SortField::SystemId, SortDir::Asc);
        assert!(q.contains("ORDER BY e/system_id/value ASC, e/ehr_id/value ASC"));
    }

    #[test]
    fn unsorted_fallback_has_no_order_by() {
        assert!(!build_list_aql_unsorted(0, 5).contains("ORDER BY"));
        assert!(is_order_by_unsupported("AQL ORDER_BY on EHR attributes is not implemented"));
        assert!(!is_order_by_unsupported("syntax error near LIMIT"));
    }

    #[test]
    fn bare_and_unknown_tokens_extend_the_id_prefix() {
        assert_eq!(parse("9c2c").ehr_id_prefix.as_deref(), Some("9c2c"));
        assert_eq!(parse("9c2c 5c5e").ehr_id_prefix.as_deref(), Some("9c2c5c5e"));
        assert_eq!(parse("weird:thing").ehr_id_prefix.as_deref(), Some("weird:thing"));
    }

    #[test]
    fn keyed_tokens_parse() {
        let c = parse("subject:P-1 namespace:nhs system:local.org modifiable:true hasCompositions:true hasDirectory:false created-on:2026-03-12");
        assert_eq!(c.subject_id.as_deref(), Some("P-1"));
        assert_eq!(c.subject_namespace.as_deref(), Some("nhs"));
        assert_eq!(c.system_id.as_deref(), Some("local.org"));
        assert_eq!((c.modifiable, c.has_compositions, c.has_directory), (Some(true), Some(true), Some(false)));
        assert_eq!(c.created_on.as_deref(), Some("2026-03-12"));
    }

    #[test]
    fn validation_errors_match_the_tauri_ui() {
        assert_eq!(parse_input("subject:").unwrap_err(), "subject: value cannot be empty.");
        assert_eq!(parse_input("modifiable:maybe").unwrap_err(), "modifiable: expects 'true' or 'false'.");
        assert_eq!(parse_input("created-on:12-03-2026").unwrap_err(), "created-on: expects a date in YYYY-MM-DD format (e.g. 2026-03-12).");
        assert!(parse_input("   ").unwrap().is_none());
    }

    #[test]
    fn created_on_overrides_the_range_with_a_warning() {
        let (c, w) = parse_input("created-on:2026-01-01 created-before:2026-02-01").unwrap().unwrap();
        assert!(c.created_before.is_none() && c.created_on.is_some());
        assert!(w.unwrap().contains("created-on overrides"));
    }

    #[test]
    fn chips_use_the_tauri_wording() {
        let c = parse("ab subject:P modifiable:false hasDirectory:true");
        assert_eq!(chips(&c), vec!["ID starts with \"ab\"", "Subject contains \"P\"", "Modifiable: No", "Has directory entries: Yes"]);
    }

    #[test]
    fn search_aql_predicates() {
        let q = build_search_aql(&parse("subject:O'Brien system:x namespace:n modifiable:true"), 200, 0).unwrap();
        assert!(q.contains("FROM EHR e CONTAINS EHR_STATUS s WHERE "));
        assert!(q.contains("s/subject/external_ref/id/value LIKE '%O''Brien%'"));
        assert!(q.contains("s/subject/external_ref/namespace = 'n'"));
        assert!(q.contains("e/system_id/value = 'x'") && q.contains("s/is_modifiable = true"));
        assert!(q.ends_with("LIMIT 200 OFFSET 0"));
    }

    #[test]
    fn full_uuid_uses_equality_but_partial_prefix_is_scanned() {
        let full = parse("9c2c5c5e-1c1b-4b0a-9d1e-9a4b2f3c8a11");
        assert!(build_search_aql(&full, 200, 0).unwrap().contains("e/ehr_id/value = '9c2c5c5e-1c1b-4b0a-9d1e-9a4b2f3c8a11'"));
        assert!(has_partial_prefix(&full).is_none());
        let partial = parse("9c2c");
        assert_eq!(has_partial_prefix(&partial), Some("9c2c"));
        // a partial prefix alone is still a valid search (no predicate; filtered client-side)
        assert!(build_search_aql(&partial, 200, 0).is_ok());
    }

    #[test]
    fn has_compositions_switches_the_from_clause() {
        let q = build_search_aql(&parse("hasCompositions:true subject:P"), 200, 0).unwrap();
        assert!(q.contains("SELECT DISTINCT") && q.contains("CONTAINS COMPOSITION c"));
        assert!(q.contains("e/ehr_status/subject/external_ref/id/value LIKE '%P%'"));
        assert!(build_search_aql(&parse("hasCompositions:false"), 200, 0).unwrap_err().contains("not currently supported"));
    }

    #[test]
    fn date_filters_are_rejected_like_tauri() {
        assert!(build_search_aql(&parse("created-after:2026-01-01"), 200, 0).unwrap_err().contains("not currently supported"));
    }

    #[test]
    fn needs_at_least_one_criterion() {
        assert!(build_search_aql(&Criteria::default(), 200, 0).is_err());
    }
}
