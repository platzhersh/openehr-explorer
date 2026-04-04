<script setup lang="ts">
import { ref, watch, reactive } from "vue";
import { useCodeMirror } from "../composables/useCodeMirror";
import type { AqlPathEntry } from "../lib/aql/aqlPathIndex";
import type { AqlCompletionConfig } from "../lib/aql/codemirror-autocomplete";

const props = defineProps<{
  modelValue: string;
  templatePaths?: Map<string, AqlPathEntry[]>;
  allTemplatePaths?: AqlPathEntry[];
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
  (e: "execute"): void;
}>();

const containerEl = ref<HTMLElement | null>(null);

// Completion config shared with CodeMirror
const completionConfig = reactive<{ value: AqlCompletionConfig }>({
  value: {
    templatePaths: new Map(),
    allTemplatePaths: [],
  },
});

const { view, getValue, setValue } = useCodeMirror(containerEl, {
  initialValue: props.modelValue,
  onExecute() {
    emit("execute");
  },
  onChange(value: string) {
    emit("update:modelValue", value);
  },
  completionConfig,
});

// Sync external value changes into editor
watch(
  () => props.modelValue,
  (newVal) => {
    if (getValue() !== newVal) {
      setValue(newVal);
    }
  },
);

// Sync template paths into completion config
watch(
  () => props.templatePaths,
  (paths) => {
    completionConfig.value.templatePaths = paths ?? new Map();
  },
  { immediate: true },
);

watch(
  () => props.allTemplatePaths,
  (paths) => {
    completionConfig.value.allTemplatePaths = paths ?? [];
  },
  { immediate: true },
);

defineExpose({ view, getValue, setValue });
</script>

<template>
  <div ref="containerEl" class="codemirror-container"></div>
</template>

<style scoped>
.codemirror-container {
  width: 100%;
  height: 100%;
  min-height: 120px;
  overflow: auto;
}
</style>
