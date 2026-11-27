const NAMED_ENTITIES: Record<string, string> = {
  lt: "<",
  gt: ">",
  quot: '"',
  apos: "'",
  amp: "&",
};

// One token of element content: a CDATA section, a character/entity reference, or plain text.
const TOKEN = /<!\[CDATA\[([\s\S]*?)\]\]>|&(#x[0-9a-f]+|#\d+|[a-z]+);|([^<&]+)/iy;

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
  TOKEN.lastIndex = 0;
  while (TOKEN.lastIndex < content.length) {
    const match = TOKEN.exec(content);
    if (!match) return null;
    if (match[1] !== undefined) {
      result += match[1];
    } else if (match[2] !== undefined) {
      const decoded = decodeReference(match[2]);
      if (decoded === null) return null;
      result += decoded;
    } else {
      result += match[3];
    }
  }
  return result;
}

/** Raw content of the first `<name>…</name>` element (any namespace prefix), or null. */
function elementContent(xml: string, name: string): string | null {
  const open = String.raw`<(?:[\w.-]+:)?${name}\b[^>]*>`;
  const close = String.raw`</(?:[\w.-]+:)?${name}\s*>`;
  return new RegExp(`${open}([\\s\\S]*?)${close}`).exec(xml)?.[1] ?? null;
}

/**
 * Template ID of an OPT, for display in the overwrite confirmation (the backend
 * does the authoritative extraction). Tolerates namespace prefixes. Returns null
 * rather than a partial value when the ID can't be read in full, so the dialog
 * never names a different template than the one that would be overwritten.
 */
export function optTemplateId(optXml: string): string | null {
  const templateId = elementContent(optXml, "template_id");
  const value = templateId === null ? null : elementContent(templateId, "value");
  return value === null ? null : decodeContent(value)?.trim() || null;
}
