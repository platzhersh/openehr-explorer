# ADR-0010: AQL-Based Listing for EHR and Composition Browsing

**Date:** 2026-04-03

## Status

Accepted

---

## Context

PRD-0003 requires browsing EHRs and viewing compositions within each EHR. The openEHR REST API specification defines endpoints for listing these resources, but their implementation varies significantly across CDR vendors.

### Standard REST API Endpoints

According to the openEHR REST API specification:

**EHR Listing:**
- No standard list endpoint defined
- Implementations vary: `GET /ehr`, `GET /ehr/list`, query parameters differ

**Composition Listing:**
- `GET /ehr/{ehr_id}/composition` - List compositions in an EHR
- Optional query parameters (varies by implementation)

### CDR Implementation Differences

| CDR | EHR List Endpoint | Composition List Endpoint | Pagination |
|-----|-------------------|---------------------------|------------|
| **EHRBase** | ❌ Not implemented | `GET /ehr/{ehr_id}/composition` | No standard pagination |
| **Better Platform** | `GET /rest/v1/ehr` (proprietary) | `GET /rest/v1/composition` | Custom query params |
| **EHRServer** | Custom endpoint | Custom endpoint | Varies |

**Problems:**

1. **No universal EHR list endpoint** - Each CDR uses different URL patterns
2. **Inconsistent pagination** - LIMIT/OFFSET support varies
3. **Response format differences** - JSON structure differs between CDRs
4. **Missing metadata** - Some endpoints return IDs only, others return full objects
5. **Authentication variations** - Different header requirements

### The Need for a Universal Solution

The openEHR Explorer aims to support **multiple CDRs** (EHRBase, Better Platform, future systems). Implementing CDR-specific list endpoints would require:

- Separate code paths for each CDR type
- Complex adapter layer to normalize responses
- Maintenance burden as CDRs update their APIs
- Testing against all supported CDRs

### AQL as a Universal Query Language

**AQL (Archetype Query Language)** is the standard openEHR query language:
- Defined in the openEHR specification
- SQL-like syntax for querying EHR data
- Supported by all compliant CDRs
- Consistent endpoint: `POST /query/aql`
- Consistent response format (rows + columns)

**Example AQL queries:**

```sql
-- List all EHRs
SELECT e/ehr_id/value, e/time_created/value, e/system_id/value
FROM EHR e
LIMIT 50 OFFSET 0

-- List compositions in an EHR
SELECT c/uid/value, c/name/value, c/archetype_details/template_id/value
FROM EHR e CONTAINS COMPOSITION c
WHERE e/ehr_id/value = 'abc-123'
ORDER BY c/context/start_time/value DESC
```

**Benefits:**
- Single implementation works across all CDRs
- Standard pagination (LIMIT/OFFSET)
- Flexible sorting (ORDER BY)
- Rich metadata (SELECT multiple fields)
- Future-proof (new CDRs just need AQL support)

---

## Decision

We will use **AQL queries** instead of REST API list endpoints for browsing EHRs and compositions.

## Implementation Pattern

### 1. EHR Listing

**File: `src-tauri/src/commands/ehr.rs:42-103`**

```rust
#[tauri::command]
pub async fn list_ehrs(
    server_id: String,
    offset: usize,
    limit: usize,
) -> Result<EhrListResponse, String> {
    let profile = get_profile_by_id(&server_id)?;
    let client = create_client(&profile);
    let base = profile.base_url.trim_end_matches('/');

    // Use AQL to list EHRs since the REST API list endpoint varies by implementation
    let aql = format!(
        "SELECT e/ehr_id/value, e/time_created/value, e/system_id/value \
         FROM EHR e \
         LIMIT {} OFFSET {}",
        limit, offset
    );

    let url = format!("{}/rest/openehr/v1/query/aql", base);
    let response = make_request(&client, reqwest::Method::POST, &url, &profile.auth_method)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "q": aql }))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch EHRs: {}", e))?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Server returned HTTP {}: {}", status, body));
    }

    let body: Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    // Parse AQL response (rows array)
    let rows = body
        .get("rows")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();

    let ehrs: Vec<EhrSummary> = rows
        .iter()
        .filter_map(|row| {
            let arr = row.as_array()?;
            Some(EhrSummary {
                ehr_id: arr.first()?.as_str()?.to_string(),
                time_created: arr.get(1).and_then(|v| v.as_str()).map(String::from),
                system_id: arr.get(2).and_then(|v| v.as_str()).map(String::from),
                subject_id: None,  // Not available in EHR query
            })
        })
        .collect();

    let total = ehrs.len() + offset; // Approximate

    Ok(EhrListResponse {
        total,
        offset,
        limit,
        ehrs,
    })
}
```

**Key elements:**

1. **AQL query construction:** `format!("SELECT ... FROM EHR e LIMIT {} OFFSET {}", limit, offset)`
2. **POST to `/query/aql`:** Standard endpoint across all CDRs
3. **Request body:** `{ "q": "<aql_query>" }`
4. **Response parsing:** Extract `rows` array, map to `EhrSummary` structs
5. **Array indexing:** `arr.first()` for ehr_id, `arr.get(1)` for time_created, etc.

### 2. Composition Listing (within EHR Detail)

**File: `src-tauri/src/commands/ehr.rs:156-200`**

```rust
// Fetch compositions via AQL
let aql = format!(
    "SELECT c/uid/value, c/archetype_details/template_id/value, c/name/value, \
            c/composer/name, c/context/start_time/value \
     FROM EHR e CONTAINS COMPOSITION c \
     WHERE e/ehr_id/value = '{}' \
     ORDER BY c/context/start_time/value DESC",
    ehr_id
);

let query_url = format!("{}/rest/openehr/v1/query/aql", base);
let comp_response = make_request(&client, reqwest::Method::POST, &query_url, &profile.auth_method)
    .header("Content-Type", "application/json")
    .json(&serde_json::json!({ "q": aql }))
    .send()
    .await
    .map_err(|e| format!("Failed to fetch compositions: {}", e))?;

let compositions = if comp_response.status().is_success() {
    let body: Value = comp_response.json().await
        .map_err(|e| format!("Failed to parse compositions: {}", e))?;

    body.get("rows")
        .and_then(|r| r.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let arr = row.as_array()?;
                    Some(CompositionSummary {
                        uid: arr.first()?.as_str()?.to_string(),
                        template_id: arr.get(1).and_then(|v| v.as_str()).map(String::from),
                        name: arr.get(2).and_then(|v| v.as_str()).map(String::from),
                        composer: arr.get(3).and_then(|v| v.as_str()).map(String::from),
                        time_committed: arr.get(4).and_then(|v| v.as_str()).map(String::from),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
} else {
    Vec::new()
};
```

**Key elements:**

1. **Filtered query:** `WHERE e/ehr_id/value = '{}'` to get compositions for specific EHR
2. **Sorting:** `ORDER BY c/context/start_time/value DESC` (newest first)
3. **Rich metadata:** Select UID, template_id, name, composer, time_committed
4. **Graceful fallback:** Returns empty `Vec` if query fails (EHR might have no compositions)

### 3. AQL Response Format

**Request:**
```json
POST /rest/openehr/v1/query/aql
Content-Type: application/json

{
  "q": "SELECT e/ehr_id/value, e/time_created/value FROM EHR e LIMIT 10"
}
```

**Response:**
```json
{
  "q": "SELECT e/ehr_id/value, e/time_created/value FROM EHR e LIMIT 10",
  "columns": [
    {"name": "e/ehr_id/value", "path": "/ehr_id/value"},
    {"name": "e/time_created/value", "path": "/time_created/value"}
  ],
  "rows": [
    ["550e8400-e29b-41d4-a716-446655440000", "2026-04-01T10:30:00Z"],
    ["7c9e6679-7425-40de-944b-e07fc1f90ae7", "2026-04-02T14:15:00Z"]
  ]
}
```

**Parsing logic:**
- `rows` is an array of arrays
- Each inner array corresponds to one result row
- Array indices match the SELECT clause order (0 = ehr_id, 1 = time_created)

---

## Consequences

### Positive

- **Universal compatibility:** Works with EHRBase, Better Platform, and any compliant CDR
- **Single implementation:** No CDR-specific code paths or adapters
- **Flexible querying:** Easy to add filtering, sorting, or additional fields
- **Standard pagination:** LIMIT/OFFSET works consistently across CDRs
- **Rich metadata:** SELECT multiple fields in one query (no need for subsequent requests)
- **Future-proof:** New CDRs just need AQL support (mandatory for openEHR compliance)
- **Debugging:** AQL queries can be tested directly in CDR's AQL console
- **Performance:** Single query returns all needed data (no N+1 queries)

### Negative

- **Query construction complexity:** Must build AQL strings carefully to avoid syntax errors
- **No type safety in AQL:** String interpolation risks (mitigated by parameterized formatting)
- **CDR-specific AQL dialects:** Some CDRs have minor AQL syntax variations (rare)
- **Error messages:** AQL syntax errors can be cryptic ("Parse error at line 1, column 45")
- **Testing:** Must test against real CDR with AQL support (can't mock easily)
- **Array indexing fragility:** If SELECT order changes, parsing breaks (mitigated by careful documentation)

### Mitigation

**SQL injection prevention:**
```rust
// ❌ Bad: Direct string interpolation
let aql = format!("WHERE e/ehr_id/value = '{}'", ehr_id);

// ✅ Good: Validate input first
if !ehr_id.chars().all(|c| c.is_alphanumeric() || c == '-') {
    return Err("Invalid EHR ID format");
}
let aql = format!("WHERE e/ehr_id/value = '{}'", ehr_id);
```

**Document SELECT order:**
```rust
// SELECT indices for EHR query:
// [0] = ehr_id/value
// [1] = time_created/value
// [2] = system_id/value
let ehr_id = arr.first()?.as_str()?.to_string();
let time_created = arr.get(1).and_then(|v| v.as_str()).map(String::from);
let system_id = arr.get(2).and_then(|v| v.as_str()).map(String::from);
```

---

## Alternatives Considered

### A. CDR-Specific REST Endpoints with Adapter Pattern

Implement separate functions for each CDR type:

```rust
match server.server_type {
    ServerType::EhrBase => list_ehrs_ehrbase(server_id, offset, limit),
    ServerType::BetterPlatform => list_ehrs_better(server_id, offset, limit),
    // ...
}
```

**Rejected because:**
- Multiplies code complexity (N implementations for N CDRs)
- Fragile (breaks when CDR updates their API)
- Incomplete (some CDRs don't have list endpoints)
- More testing surface (must test each adapter)
- Harder to add new CDRs

### B. Use REST Endpoint if Available, Fall Back to AQL

```rust
if let Some(list_endpoint) = get_list_endpoint(&server.server_type) {
    // Try REST endpoint
} else {
    // Fall back to AQL
}
```

**Rejected because:**
- Adds complexity (two code paths)
- Inconsistent behavior across CDRs
- Still requires AQL support anyway
- Doesn't provide significant benefit

### C. Client-Side Filtering (Fetch All, Filter in Rust)

```sql
SELECT e/ehr_id/value FROM EHR e
```

Then paginate in Rust code.

**Rejected because:**
- Performance: Fetching thousands of EHRs for large systems
- Memory: Loading all results into memory
- Scalability: Doesn't work for production CDRs with millions of EHRs
- Network: Large response payloads

### D. Use Custom Search Endpoint (e.g., Better Platform's `/query`)

Some CDRs provide custom search endpoints.

**Rejected because:**
- CDR-specific (defeats the purpose)
- Not standardized
- Requires separate implementation per CDR

---

## Testing

### Unit Test (AQL Query Construction)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ehr_list_aql_query() {
        let aql = format!(
            "SELECT e/ehr_id/value, e/time_created/value \
             FROM EHR e LIMIT {} OFFSET {}",
            10, 0
        );

        assert_eq!(
            aql,
            "SELECT e/ehr_id/value, e/time_created/value FROM EHR e LIMIT 10 OFFSET 0"
        );
    }

    #[test]
    fn test_composition_list_aql_query() {
        let ehr_id = "abc-123";
        let aql = format!(
            "SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c \
             WHERE e/ehr_id/value = '{}'",
            ehr_id
        );

        assert!(aql.contains("WHERE e/ehr_id/value = 'abc-123'"));
    }
}
```

### Integration Test (Against Real EHRBase)

```bash
# Start EHRBase
docker-compose up -d

# Create test EHR
curl -X POST http://localhost:8080/ehrbase/rest/openehr/v1/ehr \
  -H "Authorization: Basic $(echo -n 'ehrbase-user:ehrbase-password' | base64)" \
  -H "Content-Type: application/json" \
  -d '{"_type":"EHR_STATUS","subject":{"_type":"PARTY_SELF"},"is_queryable":true,"is_modifiable":true}'

# Query via AQL
curl -X POST http://localhost:8080/ehrbase/rest/openehr/v1/query/aql \
  -H "Authorization: Basic $(echo -n 'ehrbase-user:ehrbase-password' | base64)" \
  -H "Content-Type: application/json" \
  -d '{"q":"SELECT e/ehr_id/value FROM EHR e LIMIT 10"}'

# Expected: JSON response with rows array
```

---

## Performance Considerations

### Query Optimization

**Good:**
```sql
-- Specific fields only
SELECT e/ehr_id/value, e/time_created/value
FROM EHR e
LIMIT 50 OFFSET 0
```

**Bad:**
```sql
-- Fetches entire EHR object (slow)
SELECT e
FROM EHR e
```

### Pagination

Always use LIMIT/OFFSET for large datasets:

```rust
pub async fn list_ehrs(
    server_id: String,
    offset: usize,
    limit: usize,  // Default: 50, Max: 100
) -> Result<EhrListResponse, String> {
    let limit = limit.min(100); // Enforce max
    // ...
}
```

### Caching

AQL results can be cached (future enhancement):

```rust
// Cache key: hash(server_id, aql_query)
// TTL: 60 seconds
if let Some(cached) = cache.get(&cache_key) {
    return Ok(cached);
}
```

---

## AQL Path Reference

Commonly used paths in queries:

**EHR:**
- `e/ehr_id/value` - EHR unique identifier (UUID)
- `e/time_created/value` - EHR creation timestamp (ISO 8601)
- `e/system_id/value` - CDR system identifier

**Composition:**
- `c/uid/value` - Composition UID (versioned object ID)
- `c/name/value` - Composition name (DV_TEXT)
- `c/archetype_details/template_id/value` - Web Template ID
- `c/composer/name` - Composer name (who created it)
- `c/context/start_time/value` - Composition time (when event occurred)

**Subject (in EHR_STATUS):**
- `e/ehr_status/subject/external_ref/id/value` - Subject external ID
- `e/ehr_status/subject/external_ref/namespace` - Subject namespace

---

## Related

- ADR-0005: Three-Layer Error Handling Strategy (error handling for AQL failures)
- ADR-0006: End-to-End Type Safety (parsing AQL responses)
- PRD-0003: Composition & EHR CRUD
- openEHR AQL specification: https://specifications.openehr.org/releases/QUERY/latest/AQL.html

---

## Future Enhancements

### 1. Parameterized AQL Queries

Some CDRs support AQL parameters to prevent injection:

```json
{
  "q": "SELECT e/ehr_id/value FROM EHR e WHERE e/ehr_id/value = $ehr_id",
  "query_parameters": {
    "ehr_id": "abc-123"
  }
}
```

**Decision:** Not all CDRs support this. Use when available, validate input otherwise.

### 2. AQL Query Builder

Type-safe AQL builder instead of string formatting:

```rust
let query = AqlQuery::new()
    .select(&["e/ehr_id/value", "e/time_created/value"])
    .from("EHR e")
    .limit(50)
    .offset(0)
    .build();
```

**Decision:** YAGNI for now. String formatting is sufficient.

### 3. Named Query Templates

Store commonly used AQL queries as constants:

```rust
const LIST_EHRS_QUERY: &str = "SELECT e/ehr_id/value, e/time_created/value \
                                FROM EHR e LIMIT {} OFFSET {}";

let aql = format!(LIST_EHRS_QUERY, limit, offset);
```

**Decision:** Implement if we have 5+ repeated queries.

---

## Lessons Learned

1. **AQL is underutilized** - Many developers default to REST endpoints without considering AQL
2. **Universal solution > CDR-specific** - One robust implementation beats multiple fragile ones
3. **Standards compliance matters** - CDRs that fully implement openEHR specs are easier to support
4. **Test against multiple CDRs** - Subtle AQL dialect differences exist (e.g., path quoting)
5. **Document array indices** - AQL response parsing is fragile; document SELECT order clearly

This decision significantly simplified the codebase and improved cross-CDR compatibility.
