# PRD-0014: CDR Server Version Detection

## Status: In Progress

## Problem

openEHR Explorer needs to display the version of connected CDR servers on the Server Profiles overview. The openEHR REST specification defines **no version or status endpoint** — version discovery is entirely vendor-specific.

Previously, only EHRBase version fetching was implemented (via `GET /rest/status` XML parsing). Better Platform and Generic servers showed no version information.

## Requirements

1. Fetch and display server version for EHRBase, Better Platform, and Generic servers
2. Show a "Test" button on each server profile card in the overview
3. Always fetch the version number on successful connection tests
4. Handle version detection gracefully when unavailable (Generic servers)

## Version Endpoints by CDR

### EHRBase

| Property | Value |
|---|---|
| Endpoint | `GET {baseUrl}/rest/status` |
| Auth required | No |
| Response format | XML |
| Version field | `<ehrbase_version>` |

Example response:
```xml
<status>
  <ehrbase_version>2.15.0</ehrbase_version>
  <openehr_sdk_version>2.21.0</openehr_sdk_version>
  <archie_version>3.12.0</archie_version>
  <jvm_version>Eclipse Adoptium 21.0.6+7-LTS</jvm_version>
  <os_version>Linux amd64 4.4.0</os_version>
  <postgres_version>PostgreSQL 16.10 ...</postgres_version>
</status>
```

Notes:
- The admin endpoint `GET /rest/admin/status` returns only a plain confirmation message — no version info.
- Spring Actuator endpoints (`/management/info`, `/management/health`) are disabled by default.

### Better Platform

| Property | Value |
|---|---|
| Endpoint | `OPTIONS {baseUrl}/rest/v1` |
| Auth required | No |
| Response format | JSON |
| Version field | `solutionVersion` |

Example response:
```json
{
  "solution": "Better EHR Server",
  "solutionVersion": "3.0.0",
  "vendor": "Better d.o.o.",
  "restapiSpecsVersion": "v1",
  "options": {
    "tags": true,
    "instructionFunctions": true,
    "incompleteVersionSupport": true
  }
}
```

Notes:
- HTTP method is **OPTIONS** (not GET) — unusual but intentional.
- `restapiSpecsVersion: "v1"` identifies the EHRScape/Better proprietary API.
- The `options` block can be used for capability detection.

### Generic / Unknown openEHR Server

No version endpoint is available. Use a liveness probe instead:

```
GET {baseUrl}/rest/openehr/v1/definition/template/adl1.4
```

A `200` or `401` response confirms the server is reachable. Version is not available.

## Summary Table

| CDR | Endpoint | Method | Auth | Version field | Format |
|---|---|---|---|---|---|
| EHRBase | `/rest/status` | `GET` | None | `ehrbase_version` | XML |
| Better Platform | `/rest/v1` | `OPTIONS` | None | `solutionVersion` | JSON |
| Generic openEHR | — | — | — | — (liveness only) | — |

## Implementation

### Backend (`src-tauri/src/commands/server.rs`)

The `get_server_version` command branches by `server_type`:

1. **EHRBase** → `GET /rest/status`, parse XML for `<ehrbase_version>`
2. **Better Platform** → `OPTIONS /rest/v1`, parse JSON for `solutionVersion`
3. **Generic** → Return error (version not available for this server type)

### Frontend

- `ServerVersionInfo` includes a generic `server_version` field used for display
- Version badge on profile cards shows `server_version` (populated from either `ehrbase_version` or `solutionVersion`)
- "Test" button on each profile card triggers connection test + version fetch
- `testConnection` in the store automatically calls `fetchServerVersion` on success
