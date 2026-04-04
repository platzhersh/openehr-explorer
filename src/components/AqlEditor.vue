<script setup lang="ts">
import { ref, watch, onMounted, nextTick } from "vue";

const props = defineProps<{
  modelValue: string;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
  (e: "keydown", event: KeyboardEvent): void;
}>();

const textarea = ref<HTMLTextAreaElement | null>(null);
const highlightedCode = ref("");

// AQL keywords and syntax
const AQL_KEYWORDS = [
  "SELECT",
  "FROM",
  "WHERE",
  "CONTAINS",
  "ORDER BY",
  "LIMIT",
  "OFFSET",
  "AND",
  "OR",
  "NOT",
  "AS",
  "DISTINCT",
  "TOP",
  "EHR",
  "COMPOSITION",
  "OBSERVATION",
  "EVALUATION",
  "INSTRUCTION",
  "ACTION",
  "ADMIN_ENTRY",
  "CLUSTER",
  "ELEMENT",
  "PARTY_PROXY",
  "PARTY_IDENTIFIED",
  "PARTY_RELATED",
  "PARTY_SELF",
];

function highlightAql(code: string): string {
  // First escape HTML
  let highlighted = code.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

  // Process in order: comments first (to protect them), then strings, then the rest
  // We use placeholder tokens to protect already-highlighted segments

  const tokens: string[] = [];
  let tokenIndex = 0;

  // 1. Highlight comments first (-- style)
  highlighted = highlighted.replace(/--.*$/gm, (match) => {
    const token = `__TOKEN_${tokenIndex}__`;
    tokens[tokenIndex] = `<span class="aql-comment">${match}</span>`;
    tokenIndex++;
    return token;
  });

  // 2. Highlight strings (single and double quotes)
  highlighted = highlighted.replace(/(["'])(?:(?=(\\?))\2.)*?\1/g, (match) => {
    const token = `__TOKEN_${tokenIndex}__`;
    tokens[tokenIndex] = `<span class="aql-string">${match}</span>`;
    tokenIndex++;
    return token;
  });

  // 3. Highlight keywords (case-insensitive)
  const keywordRegex = new RegExp(`\\b(${AQL_KEYWORDS.join("|")})\\b`, "gi");
  highlighted = highlighted.replace(keywordRegex, '<span class="aql-keyword">$1</span>');

  // 4. Highlight numbers
  highlighted = highlighted.replace(/\b(\d+)\b/g, '<span class="aql-number">$1</span>');

  // 5. Highlight paths (e.g., e/ehr_id/value, c/content[at0001]/items)
  highlighted = highlighted.replace(/\b([a-z]\/[\w/\[\]@]+)/g, '<span class="aql-path">$1</span>');

  // 6. Highlight archetype IDs (e.g., at0001, openEHR-EHR-COMPOSITION.*)
  highlighted = highlighted.replace(
    /\b(at\d+|openEHR-[\w.-]+)\b/g,
    '<span class="aql-archetype">$1</span>',
  );

  // Finally, replace all tokens with their highlighted content
  tokens.forEach((content, idx) => {
    highlighted = highlighted.replace(`__TOKEN_${idx}__`, content);
  });

  return highlighted;
}

function updateHighlight() {
  highlightedCode.value = highlightAql(props.modelValue);
}

function handleInput(event: Event) {
  const target = event.target as HTMLTextAreaElement;
  emit("update:modelValue", target.value);
}

function handleKeydown(event: KeyboardEvent) {
  emit("keydown", event);

  // Handle Tab key for indentation
  if (event.key === "Tab") {
    event.preventDefault();
    const target = event.target as HTMLTextAreaElement;
    const start = target.selectionStart;
    const end = target.selectionEnd;
    const value = target.value;

    // Insert tab character
    const newValue = value.substring(0, start) + "  " + value.substring(end);
    emit("update:modelValue", newValue);

    // Restore cursor position
    nextTick(() => {
      target.selectionStart = target.selectionEnd = start + 2;
    });
  }
}

watch(() => props.modelValue, updateHighlight, { immediate: true });

onMounted(() => {
  updateHighlight();
});
</script>

<template>
  <div class="aql-editor">
    <div class="editor-backdrop">
      <pre class="highlight-layer"><code v-html="highlightedCode"></code></pre>
    </div>
    <textarea
      ref="textarea"
      class="editor-input"
      :value="modelValue"
      @input="handleInput"
      @keydown="handleKeydown"
      spellcheck="false"
      placeholder="Enter AQL query..."
    ></textarea>
  </div>
</template>

<style scoped>
.aql-editor {
  position: relative;
  width: 100%;
  min-height: 160px;
}

.editor-backdrop {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  pointer-events: none;
  overflow: hidden;
}

.highlight-layer {
  margin: 0;
  padding: 16px 24px;
  font-family: var(--font-mono);
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-wrap: break-word;
  overflow-wrap: break-word;
  color: var(--color-text);
}

.editor-input {
  position: relative;
  width: 100%;
  min-height: 160px;
  padding: 16px 24px;
  background: transparent;
  color: transparent;
  border: none;
  font-family: var(--font-mono);
  font-size: 13px;
  line-height: 1.6;
  resize: vertical;
  outline: none;
  caret-color: var(--color-text);
  white-space: pre-wrap;
  word-wrap: break-word;
  overflow-wrap: break-word;
  -webkit-text-fill-color: transparent;
}

.editor-input::placeholder {
  color: var(--color-text-muted);
  -webkit-text-fill-color: var(--color-text-muted);
}

/* AQL Syntax highlighting styles */
.highlight-layer :deep(.aql-keyword) {
  color: #569cd6;
  font-weight: 600;
  text-transform: uppercase;
}

.highlight-layer :deep(.aql-string) {
  color: #ce9178;
}

.highlight-layer :deep(.aql-number) {
  color: #b5cea8;
}

.highlight-layer :deep(.aql-path) {
  color: #9cdcfe;
}

.highlight-layer :deep(.aql-archetype) {
  color: #4ec9b0;
}

.highlight-layer :deep(.aql-comment) {
  color: #6a9955;
  font-style: italic;
}
</style>
