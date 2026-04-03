# ADR-0009: Location Header Parsing for Resource Creation

**Date:** 2026-04-03

## Status

Accepted

---

## Context

PRD-0003 introduces EHR CRUD operations, including `POST /rest/openehr/v1/ehr` to create a new EHR. According to the openEHR REST API specification:

> **POST /ehr** - Create a new EHR
> - Success: `201 Created`
> - Response headers: `Location: /rest/openehr/v1/ehr/{ehr_id}`
> - Response body: May be empty or contain the created EHR resource

### The Implementation Problem

Initial implementation assumed the response body would contain JSON with the created EHR ID:

```rust
// ❌ Original approach (FAILED)
#[tauri::command]
pub async fn create_ehr(
    server_id: String,
    request: CreateEhrRequest,
) -> Result<CreateEhrResponse, String> {
    // ... request setup ...

    let response = make_request(/* ... */)
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(/* ... */);
    }

    // Try to parse response body
    let body: Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    // Extract EHR ID from body
    let ehr_id = body.get("ehr_id")
        .and_then(|v| v.as_str())
        .ok_or("EHR ID not found in response")?
        .to_string();

    Ok(CreateEhrResponse { ehr_id, /* ... */ })
}
```

**Error encountered:**
```
Failed to parse response: EOF while parsing a value at line 1 column 0
```

**Root cause:** EHRBase (our test CDR) returns `201 Created` with:
- **Empty response body** (no JSON)
- **Location header:** `/rest/openehr/v1/ehr/a1b2c3d4-...`

Attempting to parse an empty string as JSON causes the "EOF while parsing" error.

### REST API Standards

The HTTP specification (RFC 7231) and RESTful API best practices state:

> **201 Created** - The request has been fulfilled and resulted in a new resource being created. The newly created resource can be referenced by the URI(s) returned in the entity of the response, with the most specific URI for the resource given by a **Location header field**.

**Common patterns:**

1. **Location header + empty body** (most common)
   - Minimalist approach
   - Client makes a second `GET` request to the `Location` URL if details are needed

2. **Location header + full resource in body**
   - Avoids second round-trip
   - More bandwidth but better UX

3. **No Location header, resource in body**
   - Non-standard but sometimes used
   - Less RESTful

**EHRBase behavior:** Pattern #1 (Location header + empty body).

This is **correct** per REST standards and common in well-designed APIs (GitHub, Stripe, AWS).

---

## Decision

We will **parse the `Location` header** to extract the resource ID for all `POST` operations that create new resources.

## Implementation Pattern

### Rust (Tauri Commands)

```rust
#[tauri::command]
pub async fn create_resource(
    server_id: String,
    request: CreateResourceRequest,
) -> Result<CreateResourceResponse, String> {
    // ... request setup ...

    let response = make_request(/* ... */)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Failed to create resource: {}", e))?;

    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Server returned HTTP {}: {}", status.as_u16(), body));
    }

    // Extract resource ID from Location header
    let location = response
        .headers()
        .get("Location")
        .and_then(|v| v.to_str().ok())
        .ok_or("Location header not found in response")?;

    eprintln!("Resource created at: {}", location);

    // Parse the ID from the location path
    // Example: "/rest/openehr/v1/ehr/abc123" → "abc123"
    let resource_id = location
        .split('/')
        .last()
        .ok_or("Could not extract resource ID from Location header")?
        .to_string();

    Ok(CreateResourceResponse {
        resource_id,
        // Other fields can be None or fetched with a second GET request
        system_id: None,
        time_created: None,
    })
}
```

**Key elements:**

1. **Check status first:** `if !status.is_success()` before parsing headers
2. **Extract header:** `response.headers().get("Location")`
3. **Convert to string:** `.to_str().ok()` (HeaderValue → &str)
4. **Error if missing:** `.ok_or("Location header not found")`
5. **Parse ID:** `location.split('/').last()` (splits by `/`, takes last segment)
6. **Log for debugging:** `eprintln!("Resource created at: {}", location)`

### Example: EHR Creation

**File: `src-tauri/src/commands/ehr.rs:308-323`**

```rust
// EHRBase returns 201 Created with Location header, but empty body
// Extract EHR ID from Location header: /rest/openehr/v1/ehr/{ehr_id}
let location = response
    .headers()
    .get("Location")
    .and_then(|v| v.to_str().ok())
    .ok_or("Location header not found in response")?;

eprintln!("EHR created at: {}", location);

// Extract EHR ID from the location path
let ehr_id = location
    .split('/')
    .last()
    .ok_or("Could not extract EHR ID from Location header")?
    .to_string();

Ok(CreateEhrResponse {
    ehr_id,
    system_id: None,      // Not returned in 201 response
    time_created: None,   // Not returned in 201 response
})
```

### TypeScript (Frontend)

No changes required. The store calls `invoke<CreateEhrResponse>()` and receives the `ehr_id` as normal:

```typescript
const result = await invoke<CreateEhrResponse>('create_ehr', {
  serverId,
  request,
});

console.log('Created EHR:', result.ehr_id);
```

---

## Consequences

### Positive

- **Standards compliant:** Follows HTTP/REST best practices for 201 Created responses
- **Works with all conformant CDRs:** EHRBase, Better Platform, and any CDR that follows the openEHR REST spec
- **No assumptions about body:** Doesn't rely on non-standard response body format
- **Clear error messages:** "Location header not found" is more specific than "Failed to parse JSON"
- **Logging:** `eprintln!()` output helps debugging (shows full Location URL)
- **Efficient:** Avoids parsing potentially large JSON response bodies when not needed

### Negative

- **Limited metadata:** Response only contains the resource ID, not full details (system_id, time_created)
- **Requires second request:** If full resource details are needed immediately, must make a `GET /ehr/{id}` request
- **Assumes Location format:** Parsing logic assumes `Location` ends with `/{resource_id}` (true for openEHR REST API)

### Mitigation

**If full resource details are needed:**

Make a second request after creation:

```rust
pub async fn create_ehr_with_details(
    server_id: String,
    request: CreateEhrRequest,
) -> Result<EhrDetail, String> {
    // Create the EHR (gets ID from Location header)
    let create_response = create_ehr(server_id.clone(), request).await?;

    // Fetch full details
    let details = get_ehr_detail(server_id, create_response.ehr_id).await?;

    Ok(details)
}
```

**Current decision:** Don't do this by default. The UI only needs the ID to navigate to the detail view, which fetches full details when mounted.

---

## Alternative Approaches

### A. Parse Response Body (Original Attempt)

```rust
let body: Value = response.json().await?;
let ehr_id = body.get("ehr_id").and_then(|v| v.as_str())?.to_string();
```

**Rejected because:**
- EHRBase returns empty body (breaks with "EOF while parsing")
- Assumes CDR returns body (not guaranteed by spec)
- Less RESTful than using Location header

### B. Try Body First, Fall Back to Location

```rust
let body_text = response.text().await?;

let ehr_id = if body_text.is_empty() {
    // Parse from Location header
    extract_from_location(location_header)?
} else {
    // Parse from body
    serde_json::from_str::<Value>(&body_text)?
        .get("ehr_id")
        .and_then(|v| v.as_str())?
        .to_string()
};
```

**Rejected because:**
- More complex (two code paths)
- Consumes response body, can't retry parsing
- Location header is the standard; body is optional

### C. Require Full Response Body

Update EHRBase configuration to always return full body.

**Rejected because:**
- EHRBase behavior is correct per REST standards
- Can't control third-party CDR implementations
- Would fail against other compliant CDRs

---

## Testing

### Unit Test (Rust)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ehr_id_from_location() {
        let location = "/rest/openehr/v1/ehr/550e8400-e29b-41d4-a716-446655440000";
        let ehr_id = location.split('/').last().unwrap();
        assert_eq!(ehr_id, "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_extract_composition_id_from_location() {
        let location = "/rest/openehr/v1/ehr/abc123/composition/def456::local.ehrbase.org::1";
        let uid = location.split('/').last().unwrap();
        assert_eq!(uid, "def456::local.ehrbase.org::1");
    }
}
```

### Integration Test

```bash
# Create EHR via Tauri command
curl -X POST http://localhost:8080/ehrbase/rest/openehr/v1/ehr \
  -H "Authorization: Basic $(echo -n 'ehrbase-user:ehrbase-password' | base64)" \
  -H "Content-Type: application/json" \
  -d '{"_type": "EHR_STATUS", "subject": {"_type": "PARTY_SELF"}, "is_queryable": true, "is_modifiable": true}' \
  -i

# Expected response:
# HTTP/1.1 201 Created
# Location: /rest/openehr/v1/ehr/550e8400-e29b-41d4-a716-446655440000
# (empty body)
```

---

## Applicability to Other Resources

This pattern applies to **all resource creation endpoints**:

| Endpoint | Status | Location Header Pattern |
|----------|--------|-------------------------|
| `POST /ehr` | 201 Created | `/rest/openehr/v1/ehr/{ehr_id}` |
| `POST /ehr/{ehr_id}/composition` | 201 Created | `/rest/openehr/v1/ehr/{ehr_id}/composition/{uid}` |
| `POST /definition/template/adl1.4` | 201 Created | `/rest/openehr/v1/definition/template/adl1.4/{template_id}` |
| `POST /ehr/{ehr_id}/contribution` | 201 Created | `/rest/openehr/v1/ehr/{ehr_id}/contribution/{uid}` |

**Implementation:**

Each `create_*` command should:
1. Check for `201 Created` status
2. Extract `Location` header
3. Parse resource ID from the path
4. Return minimal response (ID only)
5. Let the frontend fetch full details if needed

---

## Related

- ADR-0005: Three-Layer Error Handling Strategy (error handling patterns)
- ADR-0006: End-to-End Type Safety (response types)
- PRD-0003: Composition & EHR CRUD
- openEHR REST API specification: https://specifications.openehr.org/releases/ITS-REST/latest/

---

## References

- RFC 7231 Section 6.3.2 (201 Created): https://tools.ietf.org/html/rfc7231#section-6.3.2
- HTTP Location header: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Location
- REST API best practices: https://restfulapi.net/http-status-201-created/
- EHRBase REST API: https://ehrbase.readthedocs.io/en/latest/03_development/04_rest_api/

---

## Future Enhancements

### Fetch Full Details Automatically

Add a `fetch_details` parameter to creation commands:

```rust
pub async fn create_ehr(
    server_id: String,
    request: CreateEhrRequest,
    fetch_details: bool,  // New parameter
) -> Result<CreateEhrResponse, String> {
    // ... create logic ...

    if fetch_details {
        // Make second GET request
        let details = get_ehr_detail(server_id, ehr_id).await?;
        return Ok(CreateEhrResponse {
            ehr_id: details.ehr_id,
            system_id: details.system_id,
            time_created: details.time_created,
        });
    }

    // Return minimal response
    Ok(CreateEhrResponse {
        ehr_id,
        system_id: None,
        time_created: None,
    })
}
```

**Decision:** Not implemented for now (YAGNI). The UI doesn't need full details immediately after creation.

### Parse Query Parameters from Location

Some CDRs include query parameters in Location:
```
Location: /rest/openehr/v1/ehr/abc123?version=1
```

If needed, parse with a URL library:

```rust
use url::Url;

let url = Url::parse(&format!("http://base{}", location))?;
let ehr_id = url.path_segments()
    .and_then(|segments| segments.last())
    .ok_or("Could not extract ID")?;
```

**Decision:** Not needed for current CDRs (EHRBase, Better Platform). Can add if needed.
