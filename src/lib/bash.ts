// Minimal bash/shell command tokenizer used by BashViewer.vue — see
// ADR-0021 (JSON) and OEH-35 (XML) for the tokenize-once-render-tokens
// precedent this follows: walk the raw text into typed tokens once, then
// render tokens as plain Vue interpolation (no `v-html`, no re-highlighting
// an already-escaped string). Good enough to highlight the `curl` commands
// generated in the Request Inspector (see `generateCurl` in
// `src/stores/inspector.ts`); not a full shell grammar.

export type BashTokenType =
  | "command" // first word of the whole snippet, e.g. `curl`
  | "flag" // `-X`, `-H`, `-d`, `--header`, ...
  | "string" // single- or double-quoted string, quotes included
  | "continuation" // trailing `\` line-continuation
  | "text" // anything else (bare words, URLs, method names)
  | "space"; // run of whitespace, preserved verbatim

export interface BashToken {
  type: BashTokenType;
  text: string;
}

export interface BashLine {
  tokens: BashToken[];
}

const TOKEN_RE = /('(?:\\.|[^'\\])*'|"(?:\\.|[^"\\])*")|(\s+)|(\S+)/g;

export function parseBashLines(source: string): BashLine[] {
  if (!source) return [];
  const rawLines = source.split("\n");
  return rawLines.map((rawLine, lineIndex) => {
    const tokens: BashToken[] = [];
    let isFirstWord = lineIndex === 0;
    TOKEN_RE.lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = TOKEN_RE.exec(rawLine))) {
      const [, quoted, space, word] = match;
      if (quoted) {
        tokens.push({ type: "string", text: quoted });
        isFirstWord = false;
      } else if (space) {
        tokens.push({ type: "space", text: space });
      } else if (word) {
        const isTrailingContinuation =
          word === "\\" &&
          lineIndex < rawLines.length - 1 &&
          match.index + word.length === rawLine.length;
        if (isTrailingContinuation) {
          tokens.push({ type: "continuation", text: word });
        } else if (isFirstWord) {
          tokens.push({ type: "command", text: word });
        } else if (word.startsWith("-")) {
          tokens.push({ type: "flag", text: word });
        } else {
          tokens.push({ type: "text", text: word });
        }
        isFirstWord = false;
      }
    }
    return { tokens };
  });
}
