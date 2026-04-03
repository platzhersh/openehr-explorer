# ADR-0005: Three-Layer Error Handling Strategy

**Date:** 2026-04-03

## Status

Accepted

---

## Context

The openEHR Explorer is a cross-platform desktop application built with Tauri, combining:
- **Rust backend** - Handles HTTP requests to openEHR CDRs
- **TypeScript/Vue frontend** - User interface and state management
- **Tauri IPC bridge** - Communication between frontend and backend

Errors can occur at any layer:
- Network failures (CDR unreachable, timeout)
- HTTP errors (400 Bad Request, 404 Not Found, 500 Internal Server Error)
- CDR-specific errors (invalid openEHR structure, constraint violations)
- Application logic errors (invalid state, missing configuration)

Without a consistent error handling strategy, the application would have:
- Inconsistent error messages across features
- Lost error context as errors propagate
- Poor user experience (technical errors shown directly to users)
- Difficult debugging (unclear where errors originated)
- Repeated error handling boilerplate

We need a layered approach that provides:
1. Technical detail for developers (logs, stack traces)
2. User-friendly messages for end users
3. Proper error propagation without losing context
4. Consistent patterns across all features

---

## Decision

We will use a **three-layer error handling strategy** where each layer has distinct responsibilities:

## Layer 1: Rust Backend (Commands)

**Responsibility:** Catch low-level errors, add context, return structured error strings

**Pattern:**
```rust
#[tauri::command]
pub async fn some_operation(
    server_id: String,
    param: String
) -> Result<SuccessType, String> {
    let profile = get_profile_by_id(&server_id)?; // ? converts Error to String
    let client = create_client(&profile);

    let response = make_request(&client, Method::POST, &url, &profile.auth_method)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?; // Add context

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Server returned HTTP {}: {}", status, body)); // Structured error
    }

    let data = response.json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(data)
}
```

**Key decisions:**
- Use `Result<T, String>` return type (Tauri serializes this to frontend)
- Add context to errors with `map_err(|e| format!("Context: {}", e))`
- Include HTTP status codes and CDR error bodies
- Use `?` operator for clean error propagation
- Don't log errors here (logging happens at Layer 2)

## Layer 2: Pinia Store (State Management)

**Responsibility:** Catch errors, update error state, log for debugging, re-throw for UI

**Pattern:**
```typescript
import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export const useEntityStore = defineStore('entity', () => {
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function performOperation(
    serverId: string,
    param: string
  ): Promise<ResultType> {
    loading.value = true;
    error.value = null; // Clear previous errors

    try {
      const result = await invoke<ResultType>('some_operation', {
        serverId,
        param,
      });
      return result;
    } catch (e) {
      const errorMessage = String(e);
      error.value = errorMessage; // Store for reactive error display
      console.error('Operation failed:', errorMessage); // Log for debugging
      throw e; // Re-throw for UI handling
    } finally {
      loading.value = false; // Always clear loading state
    }
  }

  return { loading, error, performOperation };
});
```

**Key decisions:**
- Store `error` state in the store for reactive display
- Clear `error.value` at the start of operations
- Use `try/catch/finally` pattern consistently
- Log errors with `console.error()` for developer debugging
- Always re-throw errors (don't swallow them)
- Set `loading.value = false` in `finally` block

## Layer 3: UI Component (User Interaction)

**Responsibility:** Show user-friendly error messages, provide context about what failed

**Pattern:**
```vue
<script setup lang="ts">
import { ref } from 'vue';
import { useEntityStore } from '@/stores/entity';

const entityStore = useEntityStore();
const userMessage = ref<string | null>(null);

async function handleAction() {
  userMessage.value = null;

  try {
    await entityStore.performOperation(serverId, param);
    userMessage.value = 'Operation completed successfully';
  } catch (e) {
    // Convert technical error to user-friendly message
    userMessage.value = 'Failed to complete operation. Please check your connection and try again.';
    // Technical details are already in entityStore.error for advanced users
  }
}
</script>

<template>
  <div>
    <button @click="handleAction" :disabled="entityStore.loading">
      Perform Action
    </button>

    <!-- User-friendly message -->
    <div v-if="userMessage" class="message">
      {{ userMessage }}
    </div>

    <!-- Technical details (collapsible) -->
    <details v-if="entityStore.error">
      <summary>Error details</summary>
      <pre>{{ entityStore.error }}</pre>
    </details>
  </div>
</template>
```

**Key decisions:**
- Separate `userMessage` (friendly) from `entityStore.error` (technical)
- Use `try/catch` in UI even though store re-throws
- Show technical details in collapsible `<details>` element
- Disable buttons during loading state
- Don't show raw error strings to users by default

---

## Consequences

### Positive

- **Clear separation of concerns:** Each layer handles errors at the appropriate level of abstraction
- **User experience:** Users see friendly messages, not stack traces or HTTP 500 errors
- **Developer experience:** Technical details are preserved and logged for debugging
- **Consistency:** All features follow the same error handling pattern
- **Testability:** Each layer can be tested independently (mock Tauri commands, test store error state, test UI error display)
- **Error tracking:** Errors are logged at the store layer, making it easy to add error tracking services (Sentry, etc.) in the future
- **Progressive disclosure:** Advanced users can expand error details; regular users see only friendly messages

### Negative

- **Verbosity:** Requires boilerplate in all three layers (Rust `map_err`, store `try/catch/finally`, UI error display)
- **String-based errors:** Rust returns `String` instead of structured error types (required by Tauri serialization)
- **Re-throwing overhead:** Store layer catches and re-throws, which feels redundant but is necessary for state management
- **UI complexity:** Components need both user-friendly messages and technical error display

### Mitigation

- **Boilerplate verbosity:** The pattern is consistent and well-documented, making it easy to copy-paste and adapt
- **String errors:** We include structured information in strings (`"HTTP 404: ..."`) and could parse if needed
- **Re-throwing:** This is intentional - store manages state, UI manages user feedback

---

## Error Message Guidelines

### Rust Backend (Layer 1)

**Good:**
```rust
Err(format!("Server returned HTTP 404: EHR with ID '{}' not found", ehr_id))
Err(format!("Failed to parse Web Template: {}", e))
Err(format!("Failed to connect to server: {}", e))
```

**Bad:**
```rust
Err("Error".to_string()) // Too vague
Err(e.to_string()) // Loses context about what operation failed
return Err(format!("{:?}", e)) // Debug formatting is noisy
```

### Store Layer (Layer 2)

**Good:**
```typescript
console.error('Failed to create EHR:', errorMessage);
console.error('Failed to fetch templates for server:', serverId, errorMessage);
```

**Bad:**
```typescript
console.log(e); // Use console.error for errors
// No logging at all - makes debugging impossible
```

### UI Layer (Layer 3)

**Good:**
```typescript
userMessage.value = 'Failed to create EHR. Please check your inputs and try again.';
userMessage.value = 'Could not connect to server. Please check the server URL and authentication.';
```

**Bad:**
```typescript
userMessage.value = entityStore.error; // Exposes technical details
userMessage.value = 'Error'; // Too vague, not actionable
```

---

## Special Cases

### Network Timeouts

Rust uses `reqwest` with configurable timeouts. Timeout errors should be caught and formatted:

```rust
.await
.map_err(|e| {
    if e.is_timeout() {
        format!("Request timed out after 30 seconds")
    } else {
        format!("Network error: {}", e)
    }
})?
```

### CDR-Specific Errors

EHRBase and Better Platform return different error formats. Parse and normalize:

```rust
if !response.status().is_success() {
    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();

    // Try to parse JSON error body
    let error_message = if let Ok(json) = serde_json::from_str::<Value>(&body) {
        json.get("message")
            .and_then(|m| m.as_str())
            .unwrap_or(&body)
    } else {
        &body
    };

    return Err(format!("Server returned HTTP {}: {}", status, error_message));
}
```

### Validation Errors (Client-Side)

UI components should validate before calling store actions:

```typescript
async function handleSubmit() {
  // Validate first
  if (!ehrId.value) {
    userMessage.value = 'Please select an EHR';
    return;
  }

  // Then call store
  try {
    await compositionStore.createComposition(serverId, ehrId.value, data);
  } catch (e) {
    userMessage.value = 'Failed to create composition';
  }
}
```

---

## Implementation Examples

### Example 1: EHR Creation

**Rust (src-tauri/src/commands/ehr.rs:229)**
```rust
#[tauri::command]
pub async fn create_ehr(
    server_id: String,
    request: CreateEhrRequest,
) -> Result<CreateEhrResponse, String> {
    let profile = get_profile_by_id(&server_id)?;
    // ... implementation ...
    .map_err(|e| format!("Failed to create EHR: {}", e))?
}
```

**Store (src/stores/ehr.ts)**
```typescript
async function createEhr(
  serverId: string,
  request: CreateEhrRequest
): Promise<CreateEhrResponse> {
  loading.value = true;
  error.value = null;
  try {
    const result = await invoke<CreateEhrResponse>('create_ehr', {
      serverId,
      request,
    });
    return result;
  } catch (e) {
    error.value = String(e);
    throw e;
  } finally {
    loading.value = false;
  }
}
```

**UI (src/components/EhrCreateDialog.vue)**
```typescript
async function handleCreate() {
  try {
    const result = await ehrStore.createEhr(serverStore.activeServerId!, {
      subjectNamespace: form.subjectNamespace || undefined,
      subjectId: form.subjectId || undefined,
      isQueryable: form.isQueryable,
      isModifiable: form.isModifiable,
      ehrId: form.ehrId || undefined,
    });
    success.value = true;
    createdEhrId.value = result.ehr_id;
  } catch (e) {
    errorMessage.value = 'Failed to create EHR. Please try again.';
  }
}
```

---

## Related

- ADR-0004: Separate Pinia Stores Per Domain Entity
- ADR-0006: End-to-End Type Safety with Serde/TypeScript Mirroring
- PRD-0003: Composition & EHR CRUD

---

## Future Enhancements

Potential improvements that maintain the three-layer pattern:

1. **Structured error types in Rust** - Use an enum instead of String, serialize to JSON
2. **Error tracking service** - Add Sentry/Rollbar integration in store layer
3. **I18n error messages** - Translate user-friendly messages based on locale
4. **Retry logic** - Add automatic retry for transient network errors in store layer
5. **Error recovery actions** - Provide "Retry" or "Configure server" buttons in UI

These enhancements would be additive and wouldn't require changing the three-layer structure.
