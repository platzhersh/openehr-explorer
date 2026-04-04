<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount, reactive } from "vue";
import * as monaco from "monaco-editor";
import { useMonacoEditor } from "../composables/useMonacoEditor";
import { registerCompletionProvider, type CompletionContext } from "../lib/aql/completionProvider";
import type { AqlPathEntry } from "../lib/aql/aqlPathIndex";

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

// Completion context shared with the provider
const completionContext = reactive<{ value: CompletionContext }>({
  value: {
    templatePaths: new Map(),
    allTemplatePaths: [],
  },
});

const { editor, getValue, setValue } = useMonacoEditor(containerEl, {
  initialValue: props.modelValue,
  onExecute() {
    emit("execute");
  },
  onChange(value: string) {
    emit("update:modelValue", value);
  },
});

// Register the completion provider once
let completionDisposable: monaco.IDisposable | null = null;

onMounted(() => {
  completionDisposable = registerCompletionProvider(completionContext);
});

onBeforeUnmount(() => {
  completionDisposable?.dispose();
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

// Sync template paths into completion context
watch(
  () => props.templatePaths,
  (paths) => {
    completionContext.value.templatePaths = paths ?? new Map();
  },
  { immediate: true },
);

watch(
  () => props.allTemplatePaths,
  (paths) => {
    completionContext.value.allTemplatePaths = paths ?? [];
  },
  { immediate: true },
);

defineExpose({ editor, getValue, setValue });
</script>

<template>
  <div ref="containerEl" class="monaco-container"></div>
</template>

<style scoped>
.monaco-container {
  width: 100%;
  height: 100%;
  min-height: 120px;
}
</style>
