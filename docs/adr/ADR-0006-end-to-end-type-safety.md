# ADR-0006: End-to-End Type Safety with Serde/TypeScript Mirroring

**Date:** 2026-04-03

## Status

Accepted

---

## Context

The openEHR Explorer uses Tauri, which creates a Foreign Function Interface (FFI) boundary between:
- **Rust backend** - Type-safe compiled code
- **TypeScript frontend** - Type-safe transpiled code

Data flows across this boundary via Tauri's `invoke()` IPC mechanism:

```
TypeScript (Frontend)
    ↓ invoke('command_name', { params })
Tauri IPC (JSON serialization)
    ↓
Rust (Backend)
    ↓ executes Tauri command
Rust Result<T, E>
    ↓ serde_json serialization
Tauri IPC (JSON)
    ↓
TypeScript Promise<T>
```

**The Problem:**

Without explicit type coordination, errors can occur:

1. **Structural mismatch:** Frontend sends `{ ehrId: string }`, backend expects `{ ehr_id: string }`
2. **Type mismatch:** Frontend sends `number`, backend expects `String`
3. **Missing fields:** Frontend sends partial object, backend requires all fields
4. **Null handling:** Frontend sends `null`, Rust panics on unwrap
5. **Silent failures:** Serde deserialization fails, error message is cryptic

These errors only appear at **runtime** when the frontend actually calls the backend command. TypeScript can't validate that the shape of data sent via `invoke()` matches the Rust struct expected by the command.

**Current State:**

The application already has many Rust structs and TypeScript interfaces, but there's no documented standard for:
- How to name fields (camelCase vs snake_case)
- How to handle optional fields (`Option<T>` vs `T | undefined`)
- How to document the correspondence between types
- How to ensure changes to Rust types are reflected in TypeScript

---

## Decision

We will maintain **end-to-end type safety** by mirroring Rust structs with TypeScript interfaces and following strict naming and serialization conventions.

## Naming Convention

**Rust (backend):**
- Struct names: `PascalCase` (e.g., `EhrSummary`, `CreateEhrRequest`)
- Field names: `snake_case` (Rust convention)
- Serde attributes: Use `#[serde(rename_all = "camelCase")]` when frontend expects camelCase (rare)

**TypeScript (frontend):**
- Interface names: `PascalCase`, **identical to Rust struct name**
- Field names: `camelCase` (TypeScript convention) **if backend uses snake_case**
- Use exact same structure as Rust type

**Default serialization:** Rust `snake_case` ↔ TypeScript `snake_case` (no transformation)

### Example: EHR Summary

**Rust (`src-tauri/src/commands/ehr.rs`):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EhrSummary {
    pub ehr_id: String,
    pub system_id: Option<String>,
    pub time_created: Option<String>,
    pub subject_id: Option<String>,
}
```

**TypeScript (`src/stores/ehr.ts` or `src/types/ehr.ts`):**
```typescript
export interface EhrSummary {
  ehr_id: string;
  system_id?: string;
  time_created?: string;
  subject_id?: string;
}
```

**Rationale:** Keep field names identical (`ehr_id`, not `ehrId`) to avoid `serde(rename)` overhead. TypeScript allows snake_case in interfaces.

## Optional Field Mapping

| Rust | TypeScript | Notes |
|------|------------|-------|
| `field: String` | `field: string` | Required field |
| `field: Option<String>` | `field?: string` | Optional field (`undefined` in TS, `None` in Rust) |
| `field: Option<String>` | `field: string \| null` | If `null` is semantically different from missing |
| `field: Vec<T>` | `field: T[]` | Array |
| `field: bool` | `field: boolean` | Boolean |
| `field: i32` / `field: usize` | `field: number` | Integer |

**Rule:** Use `field?: string` (TypeScript optional) for Rust `Option<String>` **unless** `null` and `undefined` have different meanings.

## Derive Macros

All Rust structs that cross the FFI boundary **must** have:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyType {
    // fields...
}
```

**Why:**
- `Serialize` - Tauri can convert Rust → JSON → TypeScript
- `Deserialize` - Tauri can convert TypeScript → JSON → Rust
- `Debug` - Enables `eprintln!("{:?}", value)` for debugging
- `Clone` - Allows passing values without ownership issues

## Generic Invoke Pattern

**TypeScript side:**

```typescript
import { invoke } from '@tauri-apps/api/core';

const result = await invoke<ReturnType>('command_name', {
  paramName: value,
});
```

**Rules:**
1. Always use generic parameter: `invoke<T>()`, never `invoke()`
2. Match `ReturnType` to the Rust command's `Result<T, E>` success type
3. Parameter object keys match Rust command parameter names exactly

**Example:**

```typescript
// Rust: pub async fn create_ehr(server_id: String, request: CreateEhrRequest) -> Result<CreateEhrResponse, String>

const response = await invoke<CreateEhrResponse>('create_ehr', {
  serverId: serverIdValue,  // Tauri converts camelCase → snake_case automatically
  request: requestObject,
});
```

**Note:** Tauri's `invoke()` automatically converts camelCase JavaScript keys to snake_case Rust parameter names, so `serverId` becomes `server_id`.

## Type Location Strategy

**Small types (used in one store):**
Define TypeScript interface inline in the store file:

```typescript
// src/stores/ehr.ts
export interface EhrSummary {
  ehr_id: string;
  system_id?: string;
  // ...
}
```

**Shared types (used across multiple files):**
Define in `src/types/`:

```typescript
// src/types/ehr.ts
export interface EhrSummary { /* ... */ }
export interface CreateEhrRequest { /* ... */ }
```

Then import:
```typescript
import type { EhrSummary } from '@/types/ehr';
```

**Rule:** Co-locate with usage first, extract to `src/types/` when second usage appears.

## Documentation

Add a comment in TypeScript interfaces linking to the Rust struct:

```typescript
/**
 * Mirrors Rust struct: src-tauri/src/commands/ehr.rs::EhrSummary
 */
export interface EhrSummary {
  ehr_id: string;
  system_id?: string;
  time_created?: string;
  subject_id?: string;
}
```

This makes it clear where the canonical type definition lives.

---

## Consequences

### Positive

- **Compile-time safety:** TypeScript catches type errors before runtime
- **Refactoring safety:** Changing a Rust struct shows TypeScript errors immediately
- **IDE support:** Autocomplete and type hints work correctly across the FFI boundary
- **Self-documenting:** Type definitions serve as API documentation
- **Fewer runtime errors:** Type mismatches caught during development, not in production
- **Clear ownership:** Rust structs are the source of truth; TypeScript mirrors them

### Negative

- **Manual synchronization:** TypeScript interfaces must be manually updated when Rust structs change (no automatic code generation)
- **Duplication:** Same type definition exists in two languages
- **snake_case in TypeScript:** TypeScript interfaces use `snake_case` field names (unconventional in JS/TS ecosystems, but necessary for simplicity)

### Alternatives Considered

#### A. Automatic Type Generation (ts-rs, typeshare)

Tools like `ts-rs` or `typeshare` can generate TypeScript types from Rust structs.

**Rejected because:**
- Adds build complexity (Rust build must generate TS files before frontend build)
- Generated files need to be checked into git or excluded (complicates CI/CD)
- Tauri already handles serialization; type generation is redundant
- Manual mirroring is simple enough for a project of this size
- Generated types often need manual tweaks for edge cases

**Could revisit if:** The project grows to 50+ shared types.

#### B. Use camelCase in Rust with serde(rename_all)

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EhrSummary {
    pub ehr_id: String,  // Serializes as "ehrId"
    // ...
}
```

**Rejected because:**
- Violates Rust naming conventions (snake_case is standard)
- Adds serde overhead to every struct
- JSON logging/debugging in Rust shows camelCase (unnatural for Rust developers)
- Still requires manual TypeScript mirroring

#### C. Use Generic JSON Values (serde_json::Value)

```rust
pub async fn create_ehr(request: Value) -> Result<Value, String> {
    // ...
}
```

**Rejected because:**
- Loses all type safety in Rust (the whole point of Rust)
- Runtime errors instead of compile-time errors
- No IDE support for structure
- Makes Rust code unmaintainable

### Neutral

- **Learning curve:** Developers must understand both Rust and TypeScript type systems, but this is expected for a Tauri project

---

## Testing Strategy

### Rust Side

Unit tests ensure structs serialize correctly:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ehr_summary_serialization() {
        let ehr = EhrSummary {
            ehr_id: "test-id".to_string(),
            system_id: Some("sys-1".to_string()),
            time_created: None,
            subject_id: None,
        };

        let json = serde_json::to_string(&ehr).unwrap();
        assert!(json.contains("\"ehr_id\":\"test-id\""));
    }
}
```

### TypeScript Side

Type tests ensure invoke calls match backend:

```typescript
import { describe, it, expect } from 'vitest';
import { invoke } from '@tauri-apps/api/core';

describe('Type safety', () => {
  it('create_ehr accepts correct type', async () => {
    const request: CreateEhrRequest = {
      subject_namespace: 'test',
      subject_id: '123',
      is_queryable: true,
      is_modifiable: true,
      ehr_id: undefined,
    };

    // Type error if request doesn't match CreateEhrRequest
    await invoke<CreateEhrResponse>('create_ehr', {
      serverId: 'test-server',
      request,
    });
  });
});
```

---

## Implementation Examples

### Example 1: EHR CRUD Types

**Rust (`src-tauri/src/commands/ehr.rs:213-226`):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEhrRequest {
    pub subject_namespace: Option<String>,
    pub subject_id: Option<String>,
    pub is_queryable: Option<bool>,
    pub is_modifiable: Option<bool>,
    pub ehr_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEhrResponse {
    pub ehr_id: String,
    pub system_id: Option<String>,
    pub time_created: Option<String>,
}
```

**TypeScript (`src/stores/ehr.ts`):**
```typescript
export interface CreateEhrRequest {
  subject_namespace?: string;
  subject_id?: string;
  is_queryable?: boolean;
  is_modifiable?: boolean;
  ehr_id?: string;
}

export interface CreateEhrResponse {
  ehr_id: string;
  system_id?: string;
  time_created?: string;
}
```

### Example 2: Composition Types

**Rust (`src-tauri/src/commands/composition.rs:25-31`):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionSummary {
    pub uid: String,
    pub template_id: Option<String>,
    pub name: Option<String>,
    pub composer: Option<String>,
    pub time_committed: Option<String>,
}
```

**TypeScript (`src/stores/ehr.ts` - used in EhrDetail):**
```typescript
export interface CompositionSummary {
  uid: string;
  template_id?: string;
  name?: string;
  composer?: string;
  time_committed?: string;
}
```

---

## Related

- ADR-0004: Separate Pinia Stores Per Domain Entity
- ADR-0005: Three-Layer Error Handling Strategy
- Tauri IPC documentation: https://tauri.app/v1/guides/features/command
- Serde documentation: https://serde.rs/

---

## Future Considerations

If the project scales to 50+ types crossing the FFI boundary, consider:

1. **Automated type generation** with ts-rs or typeshare
2. **JSON Schema validation** to catch mismatches at runtime
3. **Integration tests** that exercise all Tauri commands with real data
4. **Type version tracking** to ensure frontend/backend compatibility

For now, manual mirroring is sufficient and provides the best balance of simplicity and safety.
