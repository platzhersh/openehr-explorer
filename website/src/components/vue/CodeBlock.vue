<script setup lang="ts">
// Install-command snippet with a copy-to-clipboard button. Ported from
// docs/assets/code-copy.{css,js} (PR #168 on the old static site) —
// each command line gets its own "$ "-prefixed row (excluded from
// copy/selection via a ::before pseudo-element) and a dashed separator
// from the next, and a numbered comment starts a new visual group.
//
// The old static version kept a human-maintained `data-copy` attribute
// in sync with the visible spans by hand; here the copy text is
// derived from the same `segments` prop that renders the visible
// lines, so there's only one place to get it right.
import { ref } from "vue";

type Segment = { comment: string } | { cmd: string };

const props = defineProps<{ segments: Segment[] }>();

// Groups (comment + its command lines) are joined with a blank line;
// lines within a group are adjacent. Matches the exact `data-copy`
// strings the old site hand-wrote for these same snippets.
function buildCopyText(segments: Segment[]): string {
  return segments
    .map((seg, i) => {
      const isComment = "comment" in seg;
      const text = isComment ? seg.comment : seg.cmd;
      return (isComment && i !== 0 ? "\n" : "") + text;
    })
    .join("\n");
}

const COPIED_LABEL_MS = 1800;
const copied = ref(false);
let copyTimer: ReturnType<typeof setTimeout> | undefined;

// Fallback for browsers without the async Clipboard API — and for a
// Clipboard API call that exists but rejects (e.g. permission denied
// in an insecure or embedded context).
function legacyCopy(text: string): Promise<void> {
  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.style.position = "fixed";
  textarea.style.opacity = "0";
  document.body.appendChild(textarea);
  textarea.select();
  let succeeded = false;
  try {
    succeeded = document.execCommand("copy");
  } finally {
    textarea.remove();
  }
  return succeeded ? Promise.resolve() : Promise.reject(new Error("execCommand copy failed"));
}

function copyToClipboard(text: string): Promise<void> {
  if (navigator.clipboard?.writeText) {
    return navigator.clipboard.writeText(text).catch(() => legacyCopy(text));
  }
  return legacyCopy(text);
}

function handleCopy() {
  copyToClipboard(buildCopyText(props.segments))
    .then(() => {
      copied.value = true;
      clearTimeout(copyTimer);
      copyTimer = setTimeout(() => {
        copied.value = false;
      }, COPIED_LABEL_MS);
    })
    .catch((err) => {
      // Both the Clipboard API and the execCommand fallback failed
      // (e.g. clipboard permission denied with no execCommand
      // support) — nothing more we can do here.
      console.debug("code block: copy to clipboard failed", err);
    });
}
</script>

<template>
  <div class="code-block">
    <template v-for="(seg, i) in segments" :key="i">
      <span v-if="'comment' in seg" class="code-comment">{{ seg.comment }}</span>
      <span v-else class="code-line">{{ seg.cmd }}</span>
    </template>
    <button type="button" class="code-copy-btn" :class="{ copied }" :aria-label="copied ? 'Copied!' : 'Copy to clipboard'" @click="handleCopy">
      <svg class="icon-copy" viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <rect x="9" y="9" width="12" height="12" rx="2" />
        <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
      </svg>
      <svg class="icon-check" viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <polyline points="20 6 9 17 4 12" />
      </svg>
    </button>
  </div>
</template>

<style scoped>
.code-block {
  position: relative;
  background: var(--code-bg);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 16px 48px 16px 20px;
  margin: 16px 0;
  font-family: var(--font-mono);
  font-size: 14px;
  color: var(--primary);
  overflow-x: auto;
  line-height: 1.6;
}

.code-line {
  display: block;
  overflow-wrap: anywhere;
}

.code-line::before {
  content: "$ ";
  color: var(--text-muted);
}

.code-line + .code-line {
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px dashed var(--border);
}

.code-comment {
  display: block;
  color: var(--text-muted);
  margin-top: 12px;
}

.code-comment:first-child {
  margin-top: 0;
}

.code-copy-btn {
  position: absolute;
  top: 10px;
  right: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: 0;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--text-muted);
  cursor: pointer;
  transition:
    color 0.15s ease,
    border-color 0.15s ease,
    background 0.15s ease;
}

.code-copy-btn:hover {
  color: var(--primary);
  border-color: var(--primary-dim);
  background: var(--surface-hover);
}

.code-copy-btn .icon-check {
  display: none;
  color: var(--success);
}

.code-copy-btn.copied {
  border-color: var(--success);
}

.code-copy-btn.copied .icon-copy {
  display: none;
}

.code-copy-btn.copied .icon-check {
  display: block;
}
</style>
