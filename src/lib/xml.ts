// Shared XML pretty-printing/tokenizing used by XmlViewer.vue — see ADR-0021
// (JSON) and OEH-35 (XML). Kept separate from the component so the pure
// parsing logic is unit-testable without mounting Vue.
//
// Unlike the old per-view `highlightXml()` regex helpers (which HTML-escaped
// the whole document, then re-matched `&lt;...&gt;` patterns to wrap tag/
// attribute spans, then rendered via `v-html`), this walks the raw XML text
// once into a flat list of typed tokens per line. Callers render tokens as
// plain Vue text interpolation — no escaping, no `v-html`, no double-escaping
// edge cases — mirroring how JsonViewer walks a parsed value instead of
// re-highlighting stringified JSON.

export type XmlTokenType =
  | "decl" // <?xml version="1.0"?>
  | "comment" // <!-- ... -->
  | "bracket" // <  </  >  />
  | "tag" // element name (namespaced names like xs:string included)
  | "attr-name"
  | "attr-value"
  | "punct" // = between attr name/value, and inter-attribute whitespace
  | "text"; // text content between tags

export interface XmlToken {
  type: XmlTokenType;
  text: string;
}

export interface XmlLine {
  depth: number;
  tokens: XmlToken[];
}

// XML name production, simplified: letters/digits/underscore/hyphen/period,
// plus `:` for namespace prefixes (e.g. `xs:string`, common in OPT Schema
// output). This is the fix for the namespaced-tag regression described in
// OEH-35 — one of the two duplicated `highlightXml()` copies omitted `:`.
const NAME = "[\\w:.-]+";
const ATTR = `${NAME}\\s*=\\s*(?:"[^"]*"|'[^']*')`;
const TAG = `<\\/?${NAME}(?:\\s+${ATTR})*\\s*\\/?>`;
const SPECIAL_OR_TAG_RE = new RegExp(`(<\\?[\\s\\S]*?\\?>)|(<!--[\\s\\S]*?-->)|(${TAG})`, "g");
const TAG_PARTS_RE = new RegExp(`^(<\\/?)(${NAME})([\\s\\S]*?)(\\/?>)$`);
const ATTR_RE = new RegExp(`(${NAME})(\\s*=\\s*)("[^"]*"|'[^']*')`, "g");

interface TagGroup {
  openBracket: string; // "<" | "</"
  closeBracket: string; // ">" | "/>"
}

interface TokenizedLine {
  tokens: XmlToken[];
  tagGroups: TagGroup[];
}

// Splits raw XML into per-line token spans, breaking a new line at every
// direct `><` boundary (mirroring the original `formatXml()`'s indentation
// strategy) — a line with inline text between an open and close tag (e.g.
// `<value>Some text</value>`) stays on one line, same as before.
function splitIntoTokenizedLines(xml: string): TokenizedLine[] {
  const parts = xml.trim().split(/>\s*</);
  const lines: TokenizedLine[] = [];

  parts.forEach((part, index) => {
    let line = part;
    if (index > 0) line = "<" + line;
    if (index < parts.length - 1) line = line + ">";
    line = line.trim();
    if (!line) return;

    lines.push(tokenizeLine(line));
  });

  return lines;
}

// Tokenizes one line's raw text into typed spans, splitting on XML
// declarations, comments, and tags (with their attributes broken out
// individually); anything else is plain text content. Also reports each
// tag's bracket shape so the caller can derive nesting depth without
// re-matching regex against the raw line.
function tokenizeLine(content: string): TokenizedLine {
  const tokens: XmlToken[] = [];
  const tagGroups: TagGroup[] = [];
  let cursor = 0;
  SPECIAL_OR_TAG_RE.lastIndex = 0;

  let match: RegExpExecArray | null;
  while ((match = SPECIAL_OR_TAG_RE.exec(content)) !== null) {
    if (match.index > cursor) {
      tokens.push({ type: "text", text: content.slice(cursor, match.index) });
    }

    const [whole, decl, comment] = match;
    if (decl) {
      tokens.push({ type: "decl", text: decl });
    } else if (comment) {
      tokens.push({ type: "comment", text: comment });
    } else {
      tagGroups.push(tokenizeTag(whole, tokens));
    }

    cursor = match.index + whole.length;
  }

  if (cursor < content.length) {
    tokens.push({ type: "text", text: content.slice(cursor) });
  }

  return { tokens, tagGroups };
}

function tokenizeTag(tag: string, out: XmlToken[]): TagGroup {
  const parts = TAG_PARTS_RE.exec(tag);
  if (!parts) {
    out.push({ type: "text", text: tag });
    return { openBracket: "", closeBracket: "" };
  }

  const [, openBracket, tagName, attrsRaw, closeBracket] = parts;
  out.push({ type: "bracket", text: openBracket });
  out.push({ type: "tag", text: tagName });

  let cursor = 0;
  ATTR_RE.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = ATTR_RE.exec(attrsRaw)) !== null) {
    if (match.index > cursor) {
      out.push({ type: "punct", text: attrsRaw.slice(cursor, match.index) });
    }
    const [whole, name, eq, value] = match;
    out.push({ type: "attr-name", text: name });
    out.push({ type: "punct", text: eq });
    out.push({ type: "attr-value", text: value });
    cursor = match.index + whole.length;
  }
  if (cursor < attrsRaw.length) {
    out.push({ type: "punct", text: attrsRaw.slice(cursor) });
  }

  out.push({ type: "bracket", text: closeBracket });
  return { openBracket, closeBracket };
}

/** Pretty-prints and tokenizes raw XML into renderable, depth-tagged lines. */
export function parseXmlLines(xml: string): XmlLine[] {
  const tokenizedLines = splitIntoTokenizedLines(xml);
  const lines: XmlLine[] = [];
  let depth = 0;

  for (const { tokens, tagGroups } of tokenizedLines) {
    // A line is only treated as adjusting nesting depth when it contains
    // exactly one tag construct — either a standalone opening tag (indent
    // deeper from here) or a standalone closing tag (this line dedents to
    // match its opener). A self-closing tag, an open+text+close pair on one
    // line, or a bare declaration/comment line all leave depth unchanged.
    const single = tagGroups.length === 1 ? tagGroups[0] : null;
    const isPureClose = single?.openBracket === "</";
    const isPureOpen = single?.openBracket === "<" && single?.closeBracket === ">";

    if (isPureClose) depth = Math.max(0, depth - 1);
    lines.push({ depth, tokens });
    if (isPureOpen) depth++;
  }

  return lines;
}

/** Reconstructs plain-text pretty-printed XML from parsed lines (e.g. for copy-to-clipboard). */
export function xmlLinesToText(lines: XmlLine[]): string {
  return lines
    .map((line) => "  ".repeat(line.depth) + line.tokens.map((t) => t.text).join(""))
    .join("\n");
}
