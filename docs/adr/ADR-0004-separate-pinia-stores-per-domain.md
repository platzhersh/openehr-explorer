# ADR-0004: Separate Pinia Stores Per Domain Entity

**Date:** 2026-04-03

## Status

Accepted

---

## Context

The openEHR Explorer manages multiple domain entities: Servers, Templates, EHRs, Compositions, and AQL Queries. Each entity has its own lifecycle, state management needs, and interaction with the Tauri backend.

As the application grows to support CRUD operations (PRD-0003), we need a clear strategy for organizing frontend state management. The decision is whether to:

1. Use a single monolithic Pinia store for all application state
2. Create separate Pinia stores for each domain entity
3. Use a hybrid approach with some shared state and some domain-specific stores

### Current State (Pre-PRD-0003)

The application already has domain-specific stores:
- `server.ts` - Server connection management
- `template.ts` - Template operations
- `query.ts` - AQL query execution

However, there was no documented standard for whether this pattern should continue or whether new features should consolidate into a shared store.

### Requirements

1. State management should scale as new features are added
2. Store code should be maintainable and testable
3. Related functionality should be co-located
4. Unrelated functionality should be decoupled
5. Async operations should have consistent patterns
6. TypeScript types should be clear and specific to each domain

---

## Decision

We will create **separate Pinia stores for each domain entity** following a consistent pattern.

### Store Organization

```
src/stores/
├── server.ts        # Server connection management
├── template.ts      # Template CRUD operations
├── ehr.ts          # EHR CRUD operations
├── composition.ts  # Composition CRUD operations
└── query.ts        # AQL query execution
```

Each store is responsible for:
- State for its domain entity (list, selected item, loading, error)
- Actions that call Tauri commands via `invoke()`
- Computed getters for derived state
- No cross-store dependencies (stores do not import other stores)

### Standard Store Pattern

All stores follow this structure:

```typescript
import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export const useEntityStore = defineStore('entity', () => {
  // State
  const entities = ref<Entity[]>([]);
  const selectedEntity = ref<EntityDetail | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  // Actions - Consistent async function signature
  async function fetchEntities(serverId: string): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      const result = await invoke<Entity[]>('list_entities', { serverId });
      entities.value = result;
    } catch (e) {
      error.value = String(e);
      throw e; // Re-throw for UI error handling
    } finally {
      loading.value = false;
    }
  }

  async function createEntity(
    serverId: string,
    request: CreateEntityRequest
  ): Promise<CreateEntityResponse> {
    loading.value = true;
    error.value = null;
    try {
      const result = await invoke<CreateEntityResponse>(
        'create_entity',
        { serverId, request }
      );
      // Optionally refresh list
      await fetchEntities(serverId);
      return result;
    } catch (e) {
      error.value = String(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  return {
    // State
    entities,
    selectedEntity,
    loading,
    error,
    // Actions
    fetchEntities,
    createEntity,
    // ... other CRUD operations
  };
});
```

### Key Patterns

1. **Consistent naming:** `useEntityStore` for the composable
2. **Reactive primitives:** Use `ref()` for all state (not `reactive()`)
3. **Generic invoke:** `invoke<T>()` with explicit type parameter
4. **Error handling:** Set `error.value`, then re-throw for UI handling
5. **Loading state:** Always set before/after async operations
6. **Return values:** Actions return the backend response for UI use
7. **No side effects:** Actions don't navigate or show toasts (UI responsibility)

---

## Consequences

### Positive

- **Clear boundaries:** Each store has a well-defined scope, making it obvious where to add new functionality
- **Independent testing:** Stores can be tested in isolation without mocking unrelated state
- **Type safety:** Each store has domain-specific TypeScript types, preventing type confusion (e.g., `EhrDetail` vs `CompositionDetail`)
- **Parallel development:** Multiple developers can work on different stores without merge conflicts
- **Tree-shaking:** Unused stores are not included in the bundle when building for production
- **Easier debugging:** Vue DevTools shows stores separately, making state inspection clearer
- **Consistent patterns:** All async operations follow the same structure (loading, error, try/catch, finally)
- **No circular dependencies:** Stores don't import each other, avoiding potential circular reference issues

### Negative

- **Code duplication:** Loading/error state management is duplicated across stores (though this is minimal and consistent)
- **Store proliferation:** More files to navigate (8 stores vs 1 monolithic store)
- **Cross-cutting concerns:** Features that span multiple domains require coordinating across multiple stores (e.g., "delete EHR and all its compositions")
- **No shared caching:** If multiple stores need the same data (e.g., server info), it must be passed explicitly rather than shared

### Mitigation Strategies

For cross-cutting concerns, we use **composition in components** rather than store-to-store dependencies:

```typescript
// In a component that needs both EHR and Composition data
const ehrStore = useEhrStore();
const compositionStore = useCompositionStore();

async function deleteEhrWithCompositions(ehrId: string) {
  // Component orchestrates cross-domain operations
  await compositionStore.deleteAllForEhr(ehrId);
  await ehrStore.deleteEhr(ehrId);
}
```

This keeps stores decoupled while still supporting complex workflows.

---

## Alternatives Considered

### A. Single Monolithic Store

**Rejected.** A single store with all application state would:
- Mix unrelated concerns (server config, EHR data, query results)
- Create a large, hard-to-navigate file (1000+ lines)
- Make merge conflicts more likely
- Reduce type safety (generic `state.data` could be anything)

### B. Hybrid Approach (Shared + Domain Stores)

**Rejected.** Having some state in a "global" store and some in domain stores creates ambiguity about where new state should go. It also reintroduces the problems of the monolithic approach for the shared store.

### C. Store-to-Store Dependencies

**Rejected.** Allowing stores to import and use other stores (e.g., `ehrStore` importing `serverStore`) creates tight coupling and makes testing difficult. It also risks circular dependencies. Instead, components should inject multiple stores and coordinate between them.

### D. Vuex Instead of Pinia

**Not applicable.** Pinia is the official state management library for Vue 3 and is already in use in the application. Vuex is legacy for Vue 2 projects.

---

## Implementation Examples

### Implemented Stores

1. **server.ts** - Server CRUD, active server selection, connection testing
2. **template.ts** - Template list, upload, fetch, delete
3. **ehr.ts** - EHR CRUD (create, list, detail, update status, delete)
4. **composition.ts** - Composition CRUD (create, update, delete, fetch detail)
5. **query.ts** - AQL query execution, result management

All follow the standard pattern defined in this ADR.

### Cross-Store Coordination Example

**Component: EhrBrowser.vue**

```typescript
const serverStore = useServerStore();
const ehrStore = useEhrStore();

watch(
  () => serverStore.activeServerId,
  (id) => {
    if (id) {
      ehrStore.fetchEhrs(id, 0);
    }
  }
);
```

The component watches the server store and updates the EHR store accordingly. The stores themselves remain decoupled.

---

## Related

- PRD-0003: Composition & EHR CRUD (introduced `ehr.ts` and `composition.ts`)
- ADR-0003: Dialog Component Pattern for CRUD Operations
- ADR-0005: Three-Layer Error Handling Strategy

---

## Notes

This pattern is inspired by:
- Pinia documentation on organizing stores: https://pinia.vuejs.org/cookbook/composing-stores.html
- Domain-Driven Design principles (bounded contexts)
- The Single Responsibility Principle

The pattern intentionally avoids:
- Global state singletons (use Pinia's reactive store system instead)
- Store plugins for cross-cutting concerns (keep stores simple and explicit)
- Action chaining between stores (components orchestrate instead)
