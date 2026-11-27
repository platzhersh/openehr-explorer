const NAMED_ENTITIES: Record<string, string> = {
  lt: "<",
  gt: ">",
  quot: '"',
  apos: "'",
  amp: "&",
};

// The kinds of token in element content, each matched at a given position (sticky).
const CDATA = /<!\[CDATA\[([\s\S]*?)\]\]>/y;
const REFERENCE = /&(#x[0-9a-f]+|#\d+|[a-z]+);/iy;
const TEXT = /[^<&]+/y;

function matchAt(re: RegExp, text: string, pos: number): RegExpExecArray | null {
  re.lastIndex = pos;
  return re.exec(text);
}

function decodeReference(ref: string): string | null {
  if (ref.startsWith("#")) {
    const isHex = ref[1].toLowerCase() === "x";
    const code = isHex ? Number.parseInt(ref.slice(2), 16) : Number.parseInt(ref.slice(1), 10);
    return Number.isInteger(code) && code > 0 && code <= 0x10ffff
      ? String.fromCodePoint(code)
      : null;
  }
  return NAMED_ENTITIES[ref] ?? null;
}

/** Decode XML element content (text, named/numeric references, CDATA); null if anything is left undecoded. */
function decodeContent(content: string): string | null {
  let result = "";
  let pos = 0;
  while (pos < content.length) {
    const cdata = matchAt(CDATA, content, pos);
    if (cdata) {
      result += cdata[1];
      pos += cdata[0].length;
      continue;
    }
    const reference = matchAt(REFERENCE, content, pos);
    if (reference) {
      const decoded = decodeReference(reference[1]);
      if (decoded === null) return null;
      result += decoded;
      pos += reference[0].length;
      continue;
    }
    const text = matchAt(TEXT, content, pos);
    if (!text) return null;
    result += text[0];
    pos += text[0].length;
  }
  return result;
}

/** Raw content of the first `<name>…</name>` element (any namespace prefix), or null. */
function elementContent(xml: string, name: string): string | null {
  const open = String.raw`<(?:[\w.-]+:)?${name}\b[^>]*>`;
  const close = String.raw`</(?:[\w.-]+:)?${name}\s*>`;
  return new RegExp(String.raw`${open}([\s\S]*?)${close}`).exec(xml)?.[1] ?? null;
}

/**
 * Template ID of an OPT, for display in the overwrite confirmation (the backend
 * does the authoritative extraction). Tolerates namespace prefixes. Returns null
 * rather than a partial value when the ID can't be read in full, so the dialog
 * never names a different template than the one that would be overwritten.
 */
export function optTemplateId(optXml: string): string | null {
  // Comments are skipped by the backend's XML reader, so skip them here too: a commented-out
  // <template_id> must not be named in the confirmation.
  const xml = optXml.replace(/<!--[\s\S]*?-->/g, "");
  const templateId = elementContent(xml, "template_id");
  const value = templateId === null ? null : elementContent(templateId, "value");
  return value === null ? null : decodeContent(value)?.trim() || null;
}
