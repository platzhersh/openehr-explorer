<script setup lang="ts">
// Terminology system picker: a visible dropdown of the common systems the
// backend recognises by name (see `TERMINOLOGY_SYSTEMS`), plus a "Custom…"
// option that reveals a free-text field for anything else — a raw canonical
// system URI, a national extension, or an identifier the dropdown doesn't
// know about. Replaces a plain `<input list="…">` datalist: a native
// datalist gives no visible affordance that suggestions exist, so most users
// never discover it and just retype the same handful of systems by hand.
//
// A `modelValue` that isn't one of the dropdown's values (typed by hand
// previously, or arrived via the `system` deep-link query param — see
// `applyRouteQuery` in TerminologyBrowser.vue) is treated as custom: the
// select shows "Custom…" and the text field shows that value verbatim,
// rather than silently discarding it.
import { computed, useId } from "vue";
import { TERMINOLOGY_SYSTEMS } from "../lib/terminology";

const props = defineProps<{
  modelValue: string;
  /**
   * Visible field label, rendered as a proper `<label for>` inside this
   * component — a parent-side `<label>Text<TerminologySystemSelect /></label>`
   * looks, to static a11y analysis, like a label with no real form control
   * (it can't see the `<select>` this renders), so the label lives here
   * instead of at each call site.
   */
  label: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const CUSTOM = "__custom__";
const selectId = useId();

const isCustom = computed(() => !TERMINOLOGY_SYSTEMS.some((s) => s.value === props.modelValue));
const selectValue = computed(() => (isCustom.value ? CUSTOM : props.modelValue));

function onSelectChange(e: Event) {
  const value = (e.target as HTMLSelectElement).value;
  // Switching to Custom clears the field rather than keeping the previous
  // dropdown value around under the hood — the text input starts blank.
  emit("update:modelValue", value === CUSTOM ? "" : value);
}

function onCustomInput(e: Event) {
  emit("update:modelValue", (e.target as HTMLInputElement).value);
}
</script>

<template>
  <div class="terminology-system-select">
    <label :for="selectId">{{ label }}</label>
    <select :id="selectId" class="input" :value="selectValue" @change="onSelectChange">
      <option v-for="s in TERMINOLOGY_SYSTEMS" :key="s.value" :value="s.value" :title="s.uri">
        {{ s.label }}
      </option>
      <option :value="CUSTOM">Custom…</option>
    </select>
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
.terminology-system-select .input + .input {
  margin-top: 2px;
}
</style>
