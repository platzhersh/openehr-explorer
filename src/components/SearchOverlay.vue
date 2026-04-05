<script setup lang="ts">
import { ref, watch } from "vue";

const props = defineProps<{
  placeholder?: string;
  matchCount?: number;
  totalMatches?: number;
  modelValue: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
  close: [];
  next: [];
  previous: [];
}>();

const inputRef = ref<HTMLInputElement | null>(null);

watch(
  () => props.modelValue,
  () => {
    if (props.modelValue && inputRef.value) {
      inputRef.value.focus();
    }
  },
);

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    emit("close");
  } else if (e.key === "Enter") {
    if (e.shiftKey) {
      emit("previous");
    } else {
      emit("next");
    }
  }
}

function updateValue(e: Event) {
  emit("update:modelValue", (e.target as HTMLInputElement).value);
}

defineExpose({ focus: () => inputRef.value?.focus() });
</script>

<template>
  <div class="search-overlay">
    <svg class="search-icon" width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
      <path d="M7 12C9.76142 12 12 9.76142 12 7C12 4.23858 9.76142 2 7 2C4.23858 2 2 4.23858 2 7C2 9.76142 4.23858 12 7 12Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M10.5 10.5L14 14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
    <input
      ref="inputRef"
      type="text"
      class="search-input"
      :placeholder="placeholder || 'Search...'"
      :value="modelValue"
      @input="updateValue"
      @keydown="handleKeydown"
      aria-label="Search within template"
    />
    <div v-if="totalMatches !== undefined && totalMatches > 0" class="match-counter" role="status" aria-live="polite">
      {{ matchCount !== undefined ? matchCount + 1 : 1 }} of {{ totalMatches }}
    </div>
    <div v-else-if="modelValue && totalMatches === 0" class="match-counter no-matches">No matches</div>
    <button class="close-btn" @click="emit('close')" title="Close search (Esc)">×</button>
  </div>
</template>

<style scoped>
.search-overlay {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 16px;
  background: rgba(100, 149, 237, 0.08);
  border: 1px solid rgba(100, 149, 237, 0.2);
  border-radius: var(--radius);
  margin-bottom: 16px;
  position: sticky;
  top: 0;
  z-index: 10;
  backdrop-filter: blur(8px);
}

.search-icon {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
  opacity: 0.7;
  color: var(--color-text-secondary);
}

.search-input {
  flex: 1;
  background: rgba(0, 0, 0, 0.2);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 6px 12px;
  color: var(--color-text);
  font-size: 13px;
  outline: none;
}

.search-input:focus {
  border-color: var(--color-primary);
  background: rgba(0, 0, 0, 0.3);
}

.search-input::placeholder {
  color: var(--color-text-muted);
}

.match-counter {
  font-size: 12px;
  color: var(--color-text-secondary);
  font-family: var(--font-mono);
  white-space: nowrap;
  min-width: 80px;
  text-align: right;
}

.match-counter.no-matches {
  color: var(--color-text-muted);
}

.close-btn {
  background: none;
  border: none;
  color: var(--color-text-muted);
  font-size: 24px;
  line-height: 1;
  cursor: pointer;
  padding: 0;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 3px;
  transition: all 0.15s;
  flex-shrink: 0;
}

.close-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: var(--color-text);
}
</style>
