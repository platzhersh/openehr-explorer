<script setup lang="ts">
// Terminology system picker: a searchable dropdown of the common systems the
// backend recognises by name (see `TERMINOLOGY_SYSTEMS`), plus a "Custom…"
// option that reveals a free-text field for anything else — a raw canonical
// system URI, a national extension, or an identifier the dropdown doesn't
// know about. Replaces a plain `<input list="…">` datalist: a native
// datalist gives no visible affordance that suggestions exist, so most users
// never discover it and just retype the same handful of systems by hand.
//
// Built on SearchableSelect rather than a native `<select>`, so it matches
// the dropdowns elsewhere in the app (server switcher, AQL context template)
// instead of rendering the platform's own control in the middle of the
// app's dark chrome. Each option carries its canonical URI as secondary
// text — with a native `<select>` that could only live in an `option`
// title, i.e. a tooltip nobody sees.
//
// A `modelValue` that isn't one of the dropdown's values (typed by hand
// previously, or arrived via the `system` deep-link query param — see
// `applyRouteQuery` in TerminologyBrowser.vue) is treated as custom: the
// select shows "Custom…" and the text field shows that value verbatim,
// rather than silently discarding it.
import { computed } from "vue";
import SearchableSelect, { type SearchableSelectOption } from "./SearchableSelect.vue";
import { TERMINOLOGY_SYSTEMS } from "../lib/terminology";

const props = defineProps<{
  modelValue: string;
  /**
   * Visible field label. Rendered by SearchableSelect itself (it owns the
   * `for`/`id` pairing with its trigger), not by the call site — a
   * parent-side `<label>Text<TerminologySystemSelect /></label>` looks, to
   * static a11y analysis, like a label with no real form control.
   */
  label: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const CUSTOM = "__custom__";

const options = computed<SearchableSelectOption[]>(() => [
  ...TERMINOLOGY_SYSTEMS.map((s) => ({ value: s.value, label: s.label })),
  { value: CUSTOM, label: "Custom…" },
]);

const uriByValue = computed(() =>
  Object.fromEntries(TERMINOLOGY_SYSTEMS.map((s) => [s.value, s.uri])),
);

const isCustom = computed(() => !TERMINOLOGY_SYSTEMS.some((s) => s.value === props.modelValue));
const selectValue = computed(() => (isCustom.value ? CUSTOM : props.modelValue));

function onSelect(value: string | null) {
  // Switching to Custom clears the field rather than keeping the previous
  // dropdown value around under the hood — the text input starts blank.
  emit("update:modelValue", value === CUSTOM || value === null ? "" : value);
}

function onCustomInput(e: Event) {
  emit("update:modelValue", (e.target as HTMLInputElement).value);
}
</script>

<template>
  <div class="terminology-system-select">
    <SearchableSelect
      :model-value="selectValue"
      :options="options"
      :label="label"
      search-placeholder="Filter systems..."
      @update:model-value="onSelect"
    >
      <template #option="{ option }">
        <span class="system-option">
          <span class="system-option-label">{{ option.label }}</span>
          <span v-if="uriByValue[option.value ?? '']" class="system-option-uri">
            {{ uriByValue[option.value ?? ""] }}
          </span>
        </span>
      </template>
    </SearchableSelect>
    <input
      v-if="isCustom"
      class="input"
      :value="modelValue"
      :aria-label="`Custom ${label.toLowerCase()}`"
      placeholder="Canonical system URI, e.g. http://hl7.org/fhir/sid/ndc"
      @input="onCustomInput"
    />
  </div>
</template>

<style scoped>
.terminology-system-select {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
  color: var(--color-text-secondary);
}
/* The control itself sits in a narrow toolbar column, and the panel is
   `left: 0; right: 0` on it — too narrow to read a canonical URI. It's
   absolutely positioned, so widening it costs the layout nothing. */
.terminology-system-select :deep(.searchable-select-panel) {
  min-width: 260px;
}

.system-option {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}
.system-option-label {
  color: var(--color-text);
}
.system-option-uri {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--color-text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
