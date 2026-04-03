# ADR-0003: Dialog Component Pattern for CRUD Operations

**Date:** 2026-04-03

## Status

Accepted

---

## Context

PRD-0003 introduces multiple CRUD operations (Create/Update/Delete for both EHRs and Compositions) that require user input through modal dialogs. Without a consistent pattern, each dialog could be implemented differently, leading to:

- Inconsistent user experience across different CRUD operations
- Duplicated state management logic
- Unclear component boundaries and responsibilities
- Difficult testing due to tightly coupled parent-child communication

The application needs a standardized approach for building dialog components that handle user input, validation, submission, and error states while maintaining clean separation between the dialog and its parent component.

### Requirements

1. Dialogs must be reusable across different parent components
2. Parent components should control dialog visibility
3. Dialogs should be self-contained (manage their own form state)
4. Clear communication pattern between dialog and parent
5. Consistent error handling and loading states
6. Support for both simple actions (create) and complex workflows (update with validation)

---

## Decision

We will use a **standardized dialog component pattern** with the following characteristics:

### Props and Emits Interface

**Props:**
- `open: boolean` - Controls dialog visibility (parent manages this state)
- Additional props for pre-population (e.g., `ehrId` for update dialogs)

**Emits:**
- `close` - Emitted when user cancels or clicks outside dialog
- Specific action emits - Named after the action (e.g., `created`, `updated`, `deleted`)
  - Action emits include relevant data (e.g., `created` emits the new resource ID)

### State Management

Each dialog manages its own **internal form state**:
- Form field values (reactive refs)
- Validation errors
- Loading state during submission
- Success state after completion
- Error messages from backend

The dialog does **not** manage the `open` prop - this remains in parent control.

### Example Implementation

```vue
<script setup lang="ts">
import { ref } from 'vue';

const props = defineProps<{
  open: boolean;
}>();

const emit = defineEmits<{
  close: [];
  created: [id: string];
}>();

// Internal form state
const formData = ref({ /* fields */ });
const loading = ref(false);
const error = ref<string | null>(null);
const success = ref(false);

async function handleSubmit() {
  loading.value = true;
  error.value = null;
  try {
    const result = await someStore.createResource(formData.value);
    success.value = true;
    emit('created', result.id);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

function handleClose() {
  // Reset state
  formData.value = { /* defaults */ };
  error.value = null;
  success.value = false;
  emit('close');
}
</script>

<template>
  <div v-if="open" class="dialog-overlay" @click.self="handleClose">
    <div class="dialog">
      <!-- Form fields -->
      <button @click="handleSubmit" :disabled="loading">Submit</button>
      <button @click="handleClose">Cancel</button>
    </div>
  </div>
</template>
```

### Parent Usage

```vue
<script setup lang="ts">
import { ref } from 'vue';

const showDialog = ref(false);

function handleCreated(id: string) {
  console.log('Resource created:', id);
  showDialog.value = false;
  // Refresh list, navigate, etc.
}
</script>

<template>
  <button @click="showDialog = true">New Resource</button>
  <ResourceDialog
    :open="showDialog"
    @close="showDialog = false"
    @created="handleCreated"
  />
</template>
```

---

## Consequences

### Positive

- **Consistency:** All CRUD dialogs follow the same interaction pattern, making the codebase more predictable and easier to navigate
- **Testability:** Dialogs are self-contained units that can be tested in isolation by providing props and asserting on emitted events
- **Reusability:** Dialogs can be used in multiple contexts without modification (e.g., "Create EHR" could be triggered from multiple views)
- **Clear boundaries:** Parent controls visibility, dialog controls form state - no ambiguity about responsibilities
- **Type safety:** TypeScript enforces correct prop types and emit signatures
- **User experience:** Consistent loading states, error handling, and success feedback across all dialogs

### Negative

- **Slight verbosity:** Requires boilerplate for `open` prop + `close` emit in every dialog component (but this is minimal and consistent)
- **State reset logic:** Each dialog must implement its own state reset logic in the close handler (but this is necessary for correct behavior)
- **Learning curve:** New contributors must understand the pattern, but it's well-documented and consistent

### Neutral

- **Alternative patterns exist:** Vue 3 supports Teleport, Provide/Inject, and other patterns for modals, but the prop/emit pattern is more explicit and easier to reason about for this use case
- **Styling:** Dialog overlay and positioning styles are duplicated across components, but can be extracted to shared CSS classes

---

## Implementation Examples

### Implemented

- **EhrCreateDialog.vue** - Create EHR with subject identity, flags, custom ID
  - Props: `open: boolean`
  - Emits: `close`, `created: [ehrId: string]`
  - State: Form fields, validation, loading, error, success
  - Reference: `src/components/EhrCreateDialog.vue`

### Planned

- **EhrUpdateDialog.vue** - Update EHR status
- **EhrDeleteDialog.vue** - Delete EHR with confirmation
- **CompositionDeleteDialog.vue** - Delete composition with confirmation

---

## Related

- PRD-0003: Composition & EHR CRUD
- ADR-0004: Separate Pinia Stores Per Domain Entity
- ADR-0005: Three-Layer Error Handling Strategy

---

## Notes

This pattern is inspired by:
- Vue 3 Component Events best practices
- Headless UI component patterns (separation of state and rendering)
- Material Design dialog interaction patterns

The pattern does **not** use a global dialog service or store-managed dialogs, as those patterns introduce indirect dependencies that make testing and reasoning about component behavior more difficult.
