<script setup lang="ts">
/**
 * SearchableSelect — a `<select>`-like dropdown with a filter text field,
 * for option lists where scanning by eye doesn't scale (opaque EHR UUIDs,
 * long template IDs, a growing list of server profiles). Drop-in
 * replacement for a native `<select v-model="...">` bound to a flat list
 * of `{ value, label }` options.
 *
 * Renders a closed control that looks like `.input` and shows the selected
 * option's label (or `placeholder` when nothing is selected). Clicking it
 * — or focusing it and pressing Enter/Space/ArrowDown — opens a panel with
 * a text field that filters `options` by label (case-insensitive substring
 * match) plus the resulting list. Arrow keys move the highlight, Enter
 * selects, Escape closes, and clicking outside closes without changing the
 * selection.
 *
 * Uses v-model (`defineModel`) like ToggleSwitch. For call sites that need
 * to react to a selection with a side effect rather than just assigning a
 * ref (e.g. ServerSwitcher's `serverStore.setActiveServer(id)`), bind
 * `:model-value` + `@update:model-value` instead — that's the same thing
 * v-model desugars to.
 *
 * `option`/`selected` scoped slots let a caller render richer content than
 * plain text (e.g. a connection-status suffix) while keeping the built-in
 * filtering/keyboard/open-close behaviour.
 */
import { computed, nextTick, ref, useId, watch } from "vue";

export interface SearchableSelectOption {
  value: string | null;
  label: string;
  disabled?: boolean;
}

const props = withDefaults(
  defineProps<{
    options: SearchableSelectOption[];
    /** Shown in the closed control when nothing is selected. */
    placeholder?: string;
    /** Placeholder for the filter text field inside the open panel. */
    searchPlaceholder?: string;
    /** Optional visible `<label>`, associated via `for`/`id` like TerminologySystemSelect. */
    label?: string;
    /** Shown in the panel when the filter matches nothing. */
    noOptionsText?: string;
    disabled?: boolean;
    /** Shows an inline "×" to reset the selection to null when one is made. */
    clearable?: boolean;
  }>(),
  {
    placeholder: "Select...",
    searchPlaceholder: "Search...",
    label: undefined,
    noOptionsText: "No matches",
    disabled: false,
    clearable: false,
  },
);

const modelValue = defineModel<string | null>({ default: null });

const controlId = useId();
const listboxId = useId();
const searchInputId = useId();

const rootRef = ref<HTMLElement | null>(null);
const searchInputRef = ref<HTMLInputElement | null>(null);
const listRef = ref<HTMLUListElement | null>(null);

const isOpen = ref(false);
const query = ref("");
const highlightedIndex = ref(-1);

const selectedOption = computed(
  () => props.options.find((o) => o.value === modelValue.value) ?? null,
);

const filteredOptions = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return props.options;
  return props.options.filter((o) => o.label.toLowerCase().includes(q));
});

function optionKey(option: SearchableSelectOption) {
  return option.value ?? "__none__";
}

function open() {
  if (props.disabled || isOpen.value) return;
  isOpen.value = true;
  query.value = "";
  const selectedIndex = filteredOptions.value.findIndex((o) => o.value === modelValue.value);
  highlightedIndex.value = Math.max(selectedIndex, 0);
  nextTick(() => searchInputRef.value?.focus());
}

function close() {
  isOpen.value = false;
  highlightedIndex.value = -1;
}

function toggle() {
  if (isOpen.value) close();
  else open();
}

function selectOption(option: SearchableSelectOption) {
  if (option.disabled) return;
  modelValue.value = option.value;
  close();
}

function clearSelection(event: Event) {
  event.stopPropagation();
  modelValue.value = null;
}

function scrollHighlightedIntoView() {
  nextTick(() => {
    listRef.value?.querySelector<HTMLElement>('[data-highlighted="true"]')?.scrollIntoView({
      block: "nearest",
    });
  });
}

function onControlKeydown(event: KeyboardEvent) {
  if (isOpen.value) return;
  if (event.key === "ArrowDown" || event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    open();
  }
}

function onPanelKeydown(event: KeyboardEvent) {
  const opts = filteredOptions.value;
  switch (event.key) {
    case "ArrowDown":
      event.preventDefault();
      highlightedIndex.value = Math.min(highlightedIndex.value + 1, opts.length - 1);
      scrollHighlightedIntoView();
      break;
    case "ArrowUp":
      event.preventDefault();
      highlightedIndex.value = Math.max(highlightedIndex.value - 1, 0);
      scrollHighlightedIntoView();
      break;
    case "Enter": {
      event.preventDefault();
      const opt = opts[highlightedIndex.value];
      if (opt) selectOption(opt);
      break;
    }
    case "Escape":
      event.preventDefault();
      close();
      break;
    case "Tab":
      close();
      break;
  }
}

watch(query, () => {
  highlightedIndex.value = filteredOptions.value.length > 0 ? 0 : -1;
});

function onDocumentMousedown(event: MouseEvent) {
  if (isOpen.value && !rootRef.value?.contains(event.target as Node)) {
    close();
  }
}

watch(isOpen, (open) => {
  if (open) {
    document.addEventListener("mousedown", onDocumentMousedown);
  } else {
    document.removeEventListener("mousedown", onDocumentMousedown);
  }
});
</script>

<template>
  <div ref="rootRef" class="searchable-select" :class="{ 'is-disabled': disabled }">
    <label v-if="label" :for="controlId" class="searchable-select-label">{{ label }}</label>
    <div
      :id="controlId"
      class="input searchable-select-control"
      :class="{ 'no-selection': !selectedOption, open: isOpen }"
      aria-haspopup="true"
      :aria-expanded="isOpen"
      :aria-controls="listboxId"
      :aria-disabled="disabled"
      :tabindex="disabled ? -1 : 0"
      @click="toggle"
      @keydown="onControlKeydown"
    >
      <span class="searchable-select-value" :title="selectedOption?.label">
        <slot v-if="selectedOption" name="selected" :option="selectedOption">{{
          selectedOption.label
        }}</slot>
        <span v-else class="searchable-select-placeholder">{{ placeholder }}</span>
      </span>
      <button
        v-if="clearable && selectedOption && !disabled"
        type="button"
        class="searchable-select-clear"
        title="Clear selection"
        @click="clearSelection"
      >
        &times;
      </button>
      <span class="searchable-select-arrow" aria-hidden="true">▾</span>
    </div>

    <div v-if="isOpen" class="searchable-select-panel">
      <label :for="searchInputId" class="searchable-select-search-label">{{
        searchPlaceholder
      }}</label>
      <input
        :id="searchInputId"
        ref="searchInputRef"
        v-model="query"
        type="text"
        class="input searchable-select-search"
        :placeholder="searchPlaceholder"
        :aria-controls="listboxId"
        aria-autocomplete="list"
        @keydown="onPanelKeydown"
      />
      <ul :id="listboxId" ref="listRef" class="searchable-select-list">
        <li v-if="filteredOptions.length === 0" class="searchable-select-empty">
          {{ noOptionsText }}
        </li>
        <li
          v-for="(option, index) in filteredOptions"
          :key="optionKey(option)"
          :aria-selected="option.value === modelValue"
          :data-highlighted="index === highlightedIndex"
          :title="option.label"
          class="searchable-select-option"
          :class="{
            'is-highlighted': index === highlightedIndex,
            'is-selected': option.value === modelValue,
            'is-disabled': option.disabled,
          }"
          @mouseenter="highlightedIndex = index"
          @mousedown.prevent="selectOption(option)"
        >
          <slot name="option" :option="option">{{ option.label }}</slot>
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.searchable-select {
  position: relative;
}

.searchable-select-label {
  display: block;
  margin-bottom: 4px;
}

/* Visually hidden but still readable by screen readers — associates the
   filter input with an accessible name without showing redundant text
   next to its placeholder. Same technique as ToggleSwitch's hidden
   checkbox. */
.searchable-select-search-label {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

.searchable-select-control {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  user-select: none;
}

.searchable-select.is-disabled .searchable-select-control {
  cursor: not-allowed;
  opacity: 0.6;
}

.searchable-select-control.open {
  border-color: var(--color-primary-dim);
}

.searchable-select-control.no-selection .searchable-select-value {
  color: var(--color-text-muted);
}

.searchable-select-value {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.searchable-select-clear {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  padding: 0;
  border: none;
  border-radius: 50%;
  background: transparent;
  color: var(--color-text-muted);
  font-size: 13px;
  line-height: 1;
  cursor: pointer;
}
.searchable-select-clear:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
}

.searchable-select-arrow {
  flex-shrink: 0;
  color: var(--color-text-muted);
  font-size: 10px;
}

.searchable-select-panel {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  z-index: 1000;
  display: flex;
  flex-direction: column;
  background: var(--color-bg-secondary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
  overflow: hidden;
}

.searchable-select-search {
  margin: 6px;
  border-radius: var(--radius);
}

.searchable-select-list {
  list-style: none;
  margin: 0;
  padding: 4px 0;
  max-height: 240px;
  overflow-y: auto;
  border-top: 1px solid var(--color-border);
}

.searchable-select-option {
  padding: 6px 12px;
  font-size: 13px;
  color: var(--color-text);
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.searchable-select-option.is-highlighted {
  background: var(--color-surface-hover);
}

.searchable-select-option.is-selected {
  color: var(--color-primary);
  font-weight: 600;
}

.searchable-select-option.is-disabled {
  color: var(--color-text-muted);
  cursor: not-allowed;
}

.searchable-select-empty {
  padding: 10px 12px;
  font-size: 12px;
  color: var(--color-text-muted);
  text-align: center;
}
</style>
