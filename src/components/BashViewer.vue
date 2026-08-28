<script setup lang="ts">
// Reusable bash/cURL command display component — mirrors JsonViewer.vue and
// XmlViewer.vue (ADR-0021): tokenize the raw text once (src/lib/bash.ts),
// then render tokens directly via plain interpolation instead of building
// highlighted markup with regex + `v-html`. Used for the "cURL Command"
// panel in RequestInspector.vue, replacing a plain `<pre>` with no
// highlighting and a copy button that lived in a separate toolbar strip
// above it (giving the panel two stacked boxes instead of one).
import { computed } from "vue";
import { parseBashLines } from "../lib/bash";
import CopyButton from "./CopyButton.vue";

const props = withDefaults(
  defineProps<{
    code: string;
    showLineNumbers?: boolean;
    showCopyButton?: boolean;
    copyTitle?: string;
  }>(),
  {
    showLineNumbers: true,
    showCopyButton: true,
    copyTitle: "Copy to clipboard",
  },
);

const lines = computed(() => parseBashLines(props.code));
</script>

<template>
  <div class="bash-viewer">
    <div v-if="showCopyButton" class="bv-copy-btn-wrap">
      <CopyButton :text="code" :title="copyTitle" size="md" variant="bordered" />
    </div>

    <div class="bv-scroll">
      <div class="bv-lines">
        <div v-for="(line, lineIndex) in lines" :key="lineIndex" class="bv-line">
          <span v-if="showLineNumbers" class="bv-line-number">{{ lineIndex + 1 }}</span>
          <span class="bv-content">
            <template v-for="(token, tokenIndex) in line.tokens" :key="tokenIndex">
              <span :class="`bv-${token.type}`">{{ token.text }}</span>
            </template>
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.bash-viewer {
  position: relative;
  font-family: var(--font-mono);
  font-size: 12px;
}

.bv-copy-btn-wrap {
  position: absolute;
  top: 4px;
  right: 4px;
  z-index: 1;
}

.bv-scroll {
  overflow-x: auto;
}

.bv-lines {
  display: flex;
  flex-direction: column;
  line-height: 1.6;
}

.bv-line {
  display: flex;
  align-items: flex-start;
  white-space: pre;
}

.bv-line-number {
  flex-shrink: 0;
  width: 3em;
  margin-right: 12px;
  text-align: right;
  color: var(--color-text-muted);
  opacity: 0.6;
  user-select: none;
}

.bv-content {
  min-width: 0;
}

.bv-command {
  color: #6495ed;
  font-weight: 600;
}
.bv-flag {
  color: #ffd93d;
}
.bv-string {
  color: #a8cc8c;
}
.bv-continuation {
  color: var(--color-text-muted);
}
.bv-text {
  color: var(--color-text);
}
</style>
