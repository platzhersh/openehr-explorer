<script setup lang="ts">
// Reusable JSON display component — see ADR-0021. Walks the parsed value
// directly (not a regex over `JSON.stringify` output) so that syntax
// highlighting, line numbers, and per-node collapse all stay in sync with
// the real structure. Replaces the duplicated `highlightJson()` /
// `highlightSearchInContent()` helpers that used to live in
// CompositionViewer.vue and TemplateBrowser.vue.
import { computed, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{
    value: unknown;
    /** Case-insensitive search term to highlight within the rendered JSON. */
    searchTerm?: string;
    /** 0-based index (mod total matches) of the match to mark as "current" and scroll to. */
    currentMatchIndex?: number;
    /** Container nodes at or deeper than this depth start collapsed. Default: fully expanded. */
    defaultCollapsedDepth?: number;
    showLineNumbers?: boolean;
    showCopyButton?: boolean;
  }>(),
  {
    searchTerm: "",
    currentMatchIndex: 0,
    defaultCollapsedDepth: Infinity,
    showLineNumbers: true,
    showCopyButton: true,
  },
);

const emit = defineEmits<{
  (e: "total-matches", count: number): void;
}>();

type TokenType = "key" | "string" | "number" | "boolean" | "null" | "punct" | "meta";
interface Token {
  type: TokenType;
  text: string;
}
interface JsonLine {
  id: string;
  depth: number;
  tokens: Token[];
  trailingComma: boolean;
  isOpener: boolean;
  closingBracket?: "}" | "]";
  childCount?: number;
  /** IDs of every ancestor container (nearest last). A line is hidden if any of these is collapsed. */
  parentIds: string[];
}

function valueTokens(value: unknown): Token[] {
  if (value === null) return [{ type: "null", text: "null" }];
  if (typeof value === "boolean") return [{ type: "boolean", text: String(value) }];
  if (typeof value === "number") return [{ type: "number", text: String(value) }];
  if (typeof value === "string") return [{ type: "string", text: JSON.stringify(value) }];
  // Functions/symbols/undefined shouldn't appear in JSON-sourced data, but
  // render something rather than crash if they do.
  return [{ type: "null", text: String(value) }];
}

// Recursively flattens `value` into a list of renderable "lines", each
// carrying enough structure (depth, ancestor ids, trailing comma) to
// support independent collapse/visibility without re-walking the tree.
function buildLines(
  value: unknown,
  depth: number,
  keyTokens: Token[],
  trailingComma: boolean,
  parentIds: string[],
  lines: JsonLine[],
  nextId: () => string,
): void {
  const isArr = Array.isArray(value);
  const isObj = value !== null && typeof value === "object" && !isArr;

  if (!isArr && !isObj) {
    lines.push({
      id: nextId(),
      depth,
      tokens: [...keyTokens, ...valueTokens(value)],
      trailingComma,
      isOpener: false,
      parentIds,
    });
    return;
  }

  const entries = isArr
    ? (value as unknown[]).map((v, i) => ({ key: null as string | null, value: v, idx: i }))
    : Object.entries(value as Record<string, unknown>).map(([k, v]) => ({
        key: k,
        value: v,
        idx: 0,
      }));
  const openBracket = isArr ? "[" : "{";
  const closeBracket: "}" | "]" = isArr ? "]" : "}";

  if (entries.length === 0) {
    lines.push({
      id: nextId(),
      depth,
      tokens: [...keyTokens, { type: "punct", text: openBracket + closeBracket }],
      trailingComma,
      isOpener: false,
      parentIds,
    });
    return;
  }

  const openerId = nextId();
  lines.push({
    id: openerId,
    depth,
    tokens: [...keyTokens, { type: "punct", text: openBracket }],
    trailingComma,
    isOpener: true,
    closingBracket: closeBracket,
    childCount: entries.length,
    parentIds,
  });

  const childParentIds = [...parentIds, openerId];
  entries.forEach((entry, i) => {
    const isLast = i === entries.length - 1;
    const childKeyTokens: Token[] =
      entry.key !== null
        ? [
            { type: "key", text: JSON.stringify(entry.key) },
            { type: "punct", text: ": " },
          ]
        : [];
    buildLines(entry.value, depth + 1, childKeyTokens, !isLast, childParentIds, lines, nextId);
  });

  lines.push({
    id: `${openerId}-close`,
    depth,
    tokens: [{ type: "punct", text: closeBracket }],
    trailingComma,
    isOpener: false,
    parentIds: childParentIds,
  });
}

const lines = computed<JsonLine[]>(() => {
  let counter = 0;
  const nextId = () => `n${counter++}`;
  const out: JsonLine[] = [];
  buildLines(props.value, 0, [], false, [], out, nextId);
  return out;
});

// Collapse state: seeded from `defaultCollapsedDepth` whenever the data
// changes, then mutated in place by user toggles. Ids are stable across
// re-renders of the *same* `value` (search term changes don't rebuild
// `lines`), so manual toggles survive a search.
const collapsedIds = ref<Set<string>>(new Set());
watch(
  lines,
  (newLines) => {
    const seeded = new Set<string>();
    if (Number.isFinite(props.defaultCollapsedDepth)) {
      for (const line of newLines) {
        if (line.isOpener && line.depth >= props.defaultCollapsedDepth) seeded.add(line.id);
      }
    }
    collapsedIds.value = seeded;
  },
  { immediate: true },
);

function toggle(id: string) {
  const next = new Set(collapsedIds.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  collapsedIds.value = next;
}

const searchLower = computed(() => props.searchTerm.trim().toLowerCase());

// Every match occurrence, in document order, addressed by (lineId, tokenIndex).
// Computed against the *uncollapsed* token content — collapsed containers
// that contain a match are force-expanded below, so matched tokens are
// always the real ones, never a "{ … }" placeholder.
const matchesByLine = computed(() => {
  const map = new Map<string, Map<number, Array<{ start: number; end: number }>>>();
  const term = searchLower.value;
  if (!term) return map;
  for (const line of lines.value) {
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
    if (lineMap) map.set(line.id, lineMap);
  }
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

// Ancestors of any matched line are force-expanded, so a search never
// hides its own results inside a collapsed container.
const forceExpandIds = computed(() => {
  const ids = new Set<string>();
  if (matchesByLine.value.size === 0) return ids;
  for (const line of lines.value) {
    if (matchesByLine.value.has(line.id)) {
      for (const pid of line.parentIds) ids.add(pid);
    }
  }
  return ids;
});

const effectiveCollapsed = computed(() => {
  if (forceExpandIds.value.size === 0) return collapsedIds.value;
  const next = new Set(collapsedIds.value);
  for (const id of forceExpandIds.value) next.delete(id);
  return next;
});

function isHidden(line: JsonLine): boolean {
  return line.parentIds.some((id) => effectiveCollapsed.value.has(id));
}

const visibleLines = computed(() => lines.value.filter((line) => !isHidden(line)));

// Global running index of each match, in document order — used to mark the
// "current" one (bold/outlined + scrolled into view) from `currentMatchIndex`.
const matchGlobalIndex = computed(() => {
  const map = new Map<string, number>(); // `${lineId}:${tokenIndex}:${start}` -> global index
  let counter = 0;
  for (const line of lines.value) {
    const lineMap = matchesByLine.value.get(line.id);
    if (!lineMap) continue;
    line.tokens.forEach((_token, tokenIndex) => {
      const spans = lineMap.get(tokenIndex);
      if (!spans) return;
      for (const span of spans) {
        map.set(`${line.id}:${tokenIndex}:${span.start}`, counter++);
      }
    });
  }
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

function tokenParts(line: JsonLine, tokenIndex: number, token: Token): TokenPart[] {
  const spans = matchesByLine.value.get(line.id)?.get(tokenIndex);
  if (!spans || spans.length === 0) return [{ text: token.text, isMatch: false, isCurrent: false }];

  const parts: TokenPart[] = [];
  let cursor = 0;
  for (const span of spans) {
    if (span.start > cursor) {
      parts.push({ text: token.text.slice(cursor, span.start), isMatch: false, isCurrent: false });
    }
    const globalIdx = matchGlobalIndex.value.get(`${line.id}:${tokenIndex}:${span.start}`);
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

function collapsedSuffix(line: JsonLine): string {
  const unit = line.closingBracket === "}" ? "key" : "item";
  const count = line.childCount ?? 0;
  return ` … ${count} ${unit}${count === 1 ? "" : "s"} `;
}

const root = ref<HTMLElement | null>(null);
watch(
  () => [props.currentMatchIndex, props.searchTerm],
  async () => {
    if (totalMatches.value === 0) return;
    await new Promise((r) => requestAnimationFrame(r));
    root.value?.querySelector(".jv-match.jv-match-current")?.scrollIntoView({
      behavior: "smooth",
      block: "center",
    });
  },
);

const copied = ref(false);
async function copyAll() {
  await navigator.clipboard.writeText(JSON.stringify(props.value, null, 2));
  copied.value = true;
  setTimeout(() => (copied.value = false), 1200);
}
</script>

<template>
  <div ref="root" class="json-viewer">
    <button
      v-if="showCopyButton"
      type="button"
      class="jv-copy-btn"
      :class="{ copied }"
      :title="copied ? 'Copied!' : 'Copy JSON to clipboard'"
      @click="copyAll"
    >
      <svg
        v-if="!copied"
        width="14"
        height="14"
        viewBox="0 0 16 16"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <rect x="5" y="5" width="9" height="9" rx="1.5" stroke="currentColor" stroke-width="1.3" />
        <path
          d="M3.5 10.5h-1a1 1 0 0 1-1-1v-7a1 1 0 0 1 1-1h7a1 1 0 0 1 1 1v1"
          stroke="currentColor"
          stroke-width="1.3"
        />
      </svg>
      <svg
        v-else
        width="14"
        height="14"
        viewBox="0 0 16 16"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          d="M3 8.5l3 3 7-7"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>

    <div class="jv-scroll">
      <div class="jv-lines">
        <div
          v-for="(line, i) in visibleLines"
          :key="line.id"
          class="jv-line"
          :style="{ paddingLeft: `${line.depth * 16}px` }"
        >
          <span v-if="showLineNumbers" class="jv-line-number">{{ i + 1 }}</span>
          <button
            v-if="line.isOpener"
            type="button"
            class="jv-toggle"
            :aria-expanded="!effectiveCollapsed.has(line.id)"
            :aria-label="effectiveCollapsed.has(line.id) ? 'Expand' : 'Collapse'"
            @click="toggle(line.id)"
          >
            {{ effectiveCollapsed.has(line.id) ? "▶" : "▼" }}
          </button>
          <span v-else class="jv-toggle-spacer" />

          <span class="jv-content">
            <template v-for="(token, tokenIndex) in line.tokens" :key="tokenIndex">
              <span :class="`jv-${token.type}`">
                <template v-for="(part, pi) in tokenParts(line, tokenIndex, token)" :key="pi">
                  <mark
                    v-if="part.isMatch"
                    class="jv-match"
                    :class="{ 'jv-match-current': part.isCurrent }"
                    >{{ part.text }}</mark
                  >
                  <template v-else>{{ part.text }}</template>
                </template>
              </span>
            </template>
            <span v-if="line.isOpener && effectiveCollapsed.has(line.id)" class="jv-meta">{{
              collapsedSuffix(line)
            }}</span>
            <span v-if="line.isOpener && effectiveCollapsed.has(line.id)" class="jv-punct">{{
              line.closingBracket
            }}</span>
            <span v-if="line.trailingComma" class="jv-punct">,</span>
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.json-viewer {
  position: relative;
  font-family: var(--font-mono);
  font-size: 12px;
}

.jv-copy-btn {
  position: absolute;
  top: 4px;
  right: 4px;
  z-index: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  background: var(--color-surface);
  color: var(--color-text-muted);
  cursor: pointer;
  transition: all 0.15s;
}
.jv-copy-btn:hover {
  color: var(--color-primary);
  border-color: var(--color-primary-dim);
  background: var(--color-surface-hover);
}
.jv-copy-btn.copied {
  color: var(--color-success);
  border-color: var(--color-success);
}

.jv-scroll {
  overflow-x: auto;
}

.jv-lines {
  display: flex;
  flex-direction: column;
  line-height: 1.6;
}

.jv-line {
  display: flex;
  align-items: flex-start;
  white-space: pre;
}

.jv-line-number {
  flex-shrink: 0;
  width: 3em;
  margin-right: 12px;
  text-align: right;
  color: var(--color-text-muted);
  opacity: 0.6;
  user-select: none;
}

.jv-toggle {
  flex-shrink: 0;
  width: 14px;
  height: 14px;
  margin-right: 2px;
  padding: 0;
  border: none;
  background: none;
  font: inherit;
  font-size: 9px;
  line-height: 1;
  text-align: center;
  color: var(--color-text-muted);
  cursor: pointer;
  user-select: none;
}
.jv-toggle:focus-visible {
  outline: 1px solid var(--color-primary-dim);
  outline-offset: 1px;
}
.jv-toggle:hover {
  color: var(--color-primary);
}
.jv-toggle-spacer {
  flex-shrink: 0;
  width: 16px;
}

.jv-content {
  min-width: 0;
}

.jv-key {
  color: var(--color-primary);
}
.jv-string {
  color: #a8cc8c;
}
.jv-number {
  color: #6cb6ff;
}
.jv-boolean {
  color: #dbab79;
}
.jv-null {
  color: var(--color-text-muted);
  font-style: italic;
}
.jv-punct {
  color: var(--color-text-muted);
}
.jv-meta {
  color: var(--color-text-muted);
  font-style: italic;
}

.jv-match {
  background: rgba(255, 217, 61, 0.3);
  color: inherit;
  border-radius: 2px;
}
.jv-match-current {
  background: rgba(255, 140, 0, 0.5);
  outline: 1px solid rgba(255, 140, 0, 0.8);
}
</style>
