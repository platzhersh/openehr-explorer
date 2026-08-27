<script setup lang="ts">
// Reusable XML display component — see ADR-0021 (JSON precedent) and OEH-35.
// Tokenizes the raw XML text (src/lib/xml.ts) instead of re-highlighting an
// HTML-escaped string with regex, so syntax highlighting and line numbers
// stay in sync with the real tag structure and namespaced tags (`xs:string`)
// highlight correctly everywhere. Replaces the duplicated `formatXml()` /
// `highlightXml()` helper pairs that used to live in TemplateBrowser.vue and
// RequestInspector.vue. The copy button composes CopyButton.vue (OEH-37)
// rather than reimplementing it.
import { computed, ref, watch } from "vue";
import { parseXmlLines, xmlLinesToText, type XmlLine, type XmlToken } from "../lib/xml";
import CopyButton from "./CopyButton.vue";

const props = withDefaults(
  defineProps<{
    xml: string;
    /** Case-insensitive search term to highlight within the rendered XML. */
    searchTerm?: string;
    /** 0-based index (mod total matches) of the match to mark as "current" and scroll to. */
    currentMatchIndex?: number;
    showLineNumbers?: boolean;
    showCopyButton?: boolean;
  }>(),
  {
    searchTerm: "",
    currentMatchIndex: 0,
    showLineNumbers: true,
    showCopyButton: true,
  },
);

const emit = defineEmits<{
  (e: "total-matches", count: number): void;
}>();

const lines = computed<XmlLine[]>(() => (props.xml ? parseXmlLines(props.xml) : []));

const searchLower = computed(() => props.searchTerm.trim().toLowerCase());

// Every match occurrence, in document order, addressed by (lineIndex, tokenIndex).
const matchesByLine = computed(() => {
  const map = new Map<number, Map<number, Array<{ start: number; end: number }>>>();
  const term = searchLower.value;
  if (!term) return map;
  lines.value.forEach((line, lineIndex) => {
    let lineMap: Map<number, Array<{ start: number; end: number }>> | null = null;
    line.tokens.forEach((token, tokenIndex) => {
      const lower = token.text.toLowerCase();
      const spans: Array<{ start: number; end: number }> = [];
      let from = 0;
      for (;;) {
        const found = lower.indexOf(term, from);
        if (found === -1) break;
        spans.push({ start: found, end: found + term.length });
        from = found + term.length;
      }
      if (spans.length > 0) {
        if (!lineMap) lineMap = new Map();
        lineMap.set(tokenIndex, spans);
      }
    });
    if (lineMap) map.set(lineIndex, lineMap);
  });
  return map;
});

const totalMatches = computed(() => {
  let count = 0;
  for (const lineMap of matchesByLine.value.values()) {
    for (const spans of lineMap.values()) count += spans.length;
  }
  return count;
});
watch(totalMatches, (n) => emit("total-matches", n), { immediate: true });

// Global running index of each match, in document order — used to mark the
// "current" one (highlighted + scrolled into view) from `currentMatchIndex`.
const matchGlobalIndex = computed(() => {
  const map = new Map<string, number>(); // `${lineIndex}:${tokenIndex}:${start}` -> global index
  let counter = 0;
  lines.value.forEach((line, lineIndex) => {
    const lineMap = matchesByLine.value.get(lineIndex);
    if (!lineMap) return;
    line.tokens.forEach((_token, tokenIndex) => {
      const spans = lineMap.get(tokenIndex);
      if (!spans) return;
      for (const span of spans) {
        map.set(`${lineIndex}:${tokenIndex}:${span.start}`, counter++);
      }
    });
  });
  return map;
});

const currentGlobalIndex = computed(() => {
  if (totalMatches.value === 0) return -1;
  const idx = props.currentMatchIndex % totalMatches.value;
  return idx < 0 ? idx + totalMatches.value : idx;
});

interface TokenPart {
  text: string;
  isMatch: boolean;
  isCurrent: boolean;
}

function tokenParts(lineIndex: number, tokenIndex: number, token: XmlToken): TokenPart[] {
  const spans = matchesByLine.value.get(lineIndex)?.get(tokenIndex);
  if (!spans || spans.length === 0) return [{ text: token.text, isMatch: false, isCurrent: false }];

  const parts: TokenPart[] = [];
  let cursor = 0;
  for (const span of spans) {
    if (span.start > cursor) {
      parts.push({ text: token.text.slice(cursor, span.start), isMatch: false, isCurrent: false });
    }
    const globalIdx = matchGlobalIndex.value.get(`${lineIndex}:${tokenIndex}:${span.start}`);
    parts.push({
      text: token.text.slice(span.start, span.end),
      isMatch: true,
      isCurrent: globalIdx === currentGlobalIndex.value,
    });
    cursor = span.end;
  }
  if (cursor < token.text.length) {
    parts.push({ text: token.text.slice(cursor), isMatch: false, isCurrent: false });
  }
  return parts;
}

const root = ref<HTMLElement | null>(null);
watch(
  () => [props.currentMatchIndex, props.searchTerm],
  async () => {
    if (totalMatches.value === 0) return;
    await new Promise((r) => requestAnimationFrame(r));
    root.value?.querySelector(".xv-match.xv-match-current")?.scrollIntoView({
      behavior: "smooth",
      block: "center",
    });
  },
);

const copyText = computed(() => xmlLinesToText(lines.value));
</script>

<template>
  <div ref="root" class="xml-viewer">
    <div v-if="showCopyButton" class="xv-copy-btn-wrap">
      <CopyButton :text="copyText" title="Copy XML to clipboard" size="md" variant="bordered" />
    </div>

    <div class="xv-scroll">
      <div class="xv-lines">
        <div
          v-for="(line, lineIndex) in lines"
          :key="lineIndex"
          class="xv-line"
          :style="{ paddingLeft: `${line.depth * 16}px` }"
        >
          <span v-if="showLineNumbers" class="xv-line-number">{{ lineIndex + 1 }}</span>
          <span class="xv-content">
            <template v-for="(token, tokenIndex) in line.tokens" :key="tokenIndex">
              <span :class="`xv-${token.type}`">
                <template v-for="(part, pi) in tokenParts(lineIndex, tokenIndex, token)" :key="pi">
                  <mark
                    v-if="part.isMatch"
                    class="xv-match"
                    :class="{ 'xv-match-current': part.isCurrent }"
                    >{{ part.text }}</mark
                  >
                  <template v-else>{{ part.text }}</template>
                </template>
              </span>
            </template>
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.xml-viewer {
  position: relative;
  font-family: var(--font-mono);
  font-size: 12px;
}

.xv-copy-btn-wrap {
  position: absolute;
  top: 4px;
  right: 4px;
  z-index: 1;
}

.xv-scroll {
  overflow-x: auto;
}

.xv-lines {
  display: flex;
  flex-direction: column;
  line-height: 1.6;
}

.xv-line {
  display: flex;
  align-items: flex-start;
  white-space: pre;
}

.xv-line-number {
  flex-shrink: 0;
  width: 3em;
  margin-right: 12px;
  text-align: right;
  color: var(--color-text-muted);
  opacity: 0.6;
  user-select: none;
}

.xv-content {
  min-width: 0;
}

.xv-tag {
  color: #6495ed;
  font-weight: 600;
}
.xv-attr-name {
  color: #ffd93d;
}
.xv-attr-value {
  color: #6bff8e;
}
.xv-comment {
  color: var(--color-text-muted);
  font-style: italic;
}
.xv-decl {
  color: #ff6b6b;
}
.xv-bracket,
.xv-punct {
  color: var(--color-text-muted);
}
.xv-text {
  color: var(--color-text);
}

.xv-match {
  background: rgba(255, 217, 61, 0.3);
  color: inherit;
  border-radius: 2px;
}
.xv-match-current {
  background: rgba(255, 140, 0, 0.5);
  outline: 1px solid rgba(255, 140, 0, 0.8);
}
</style>
