/**
 * Best-effort template ID of an OPT, for display only (the backend does the
 * authoritative extraction). Tolerates namespace prefixes (`<o:template_id>`).
 */
const ENTITIES: Record<string, string> = {
  lt: "<",
  gt: ">",
  quot: '"',
  apos: "'",
  amp: "&",
};

export function optTemplateId(optXml: string): string | null {
  const match = /<(?:[\w.-]+:)?template_id\b[^>]*>\s*<(?:[\w.-]+:)?value\b[^>]*>([^<]*)</.exec(
    optXml,
  );
  const id = match?.[1]
    ?.replace(/&(lt|gt|quot|apos|amp);/g, (_, name: string) => ENTITIES[name])
    .trim();
  return id ? id : null;
}
