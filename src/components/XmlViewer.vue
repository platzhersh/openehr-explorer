<script setup lang="ts">
// Reusable XML display component — see ADR-0021 (JSON precedent) and OEH-35.
// Tokenizes the raw XML text (src/lib/xml.ts) instead of re-highlighting an
// HTML-escaped string with regex, so syntax highlighting and line numbers
// stay in sync with the real tag structure and namespaced tags (`xs:string`)
// highlight correctly everywhere. Replaces the duplicated `formatXml()` /
// `highlightXml()` helper pairs that used to live in TemplateBrowser.vue and
// RequestInspector.vue. The copy button composes CopyButton.vue (OEH-37)
// rather than reimplementing it.
//
// Rendering is virtualized (src/composables/useVirtualList.ts): a large OPT
// XML document (10k+ lines) used to render one <span> tree per line for the
// entire document at once, which froze the app and kept growing memory as
// more of the document was scrolled into (and never out of) the DOM. Only
// the lines near the viewport are rendered now — see the perf ADR.
import { computed, ref, watch } from "vue";
import { parseXmlLines, xmlLinesToText, type XmlLine, type XmlToken } from "../lib/xml";
import { useVirtualList } from "../composables/useVirtualList";
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

// Must match `.xv-line`'s fixed height in <style> below — row virtualization
// assumes every line takes exactly this many px.
const ROW_HEIGHT = 20;

const scrollEl = ref<HTMLElement | null>(null);
const {
  onScroll,
  startIndex,
  topPadding,
  bottomPadding,
  visibleItems: visibleLines,
  scrollToIndex,
} = useVirtualList(lines, scrollEl, { rowHeight: ROW_HEIGHT, overscan: 20 });

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
// Built in one pass alongside a reverse lookup (global index -> line index)
// so scrolling to the current match doesn't depend on it already being
// rendered in the (now virtualized) DOM.
const matchIndex = computed(() => {
  const byKey = new Map<string, number>(); // `${lineIndex}:${tokenIndex}:${start}` -> global index
  const lineByGlobalIndex: number[] = [];
  let counter = 0;
  lines.value.forEach((line, lineIndex) => {
    const lineMap = matchesByLine.value.get(lineIndex);
    if (!lineMap) return;
    line.tokens.forEach((_token, tokenIndex) => {
      const spans = lineMap.get(tokenIndex);
      if (!spans) return;
      for (const span of spans) {
        byKey.set(`${lineIndex}:${tokenIndex}:${span.start}`, counter);
        lineByGlobalIndex.push(lineIndex);
        counter++;
      }
    });
  });
  return { byKey, lineByGlobalIndex };
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
    const globalIdx = matchIndex.value.byKey.get(`${lineIndex}:${tokenIndex}:${span.start}`);
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

// Scrolls the current match into view by its line index directly (via the
// virtual list's own scrollTo), rather than querying the DOM for the
// rendered .xv-match-current node — that node may not exist yet since only
// lines near the viewport are rendered.
watch(
  () => [props.currentMatchIndex, props.searchTerm],
  () => {
    if (totalMatches.value === 0) return;
    const lineIndex = matchIndex.value.lineByGlobalIndex[currentGlobalIndex.value];
    if (lineIndex === undefined) return;
    scrollToIndex(lineIndex, { center: true, behavior: "smooth" });
  },
);

const copyText = computed(() => xmlLinesToText(lines.value));
</script>

<template>
  <div class="xml-viewer">
    <div v-if="showCopyButton" class="xv-copy-btn-wrap">
      <CopyButton :text="copyText" title="Copy XML to clipboard" size="md" variant="bordered" />
    </div>

    <div ref="scrollEl" class="xv-scroll" @scroll="onScroll">
      <div
        class="xv-lines"
        :style="{ paddingTop: `${topPadding}px`, paddingBottom: `${bottomPadding}px` }"
      >
        <div
          v-for="(line, i) in visibleLines"
          :key="startIndex + i"
          class="xv-line"
          :style="{ paddingLeft: `${line.depth * 16}px` }"
        >
          <span v-if="showLineNumbers" class="xv-line-number">{{ startIndex + i + 1 }}</span>
          <span class="xv-content">
            <template v-for="(token, tokenIndex) in line.tokens" :key="tokenIndex">
              <span :class="`xv-${token.type}`">
                <template
                  v-for="(part, pi) in tokenParts(startIndex + i, tokenIndex, token)"
                  :key="pi"
                >
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
  overflow: auto;
  max-height: 65vh;
}

.xv-lines {
  display: flex;
  flex-direction: column;
}

.xv-line {
  display: flex;
  align-items: center;
  height: 20px;
  line-height: 20px;
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
