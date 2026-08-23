import { sql, SQLDialect, StandardSQL } from "@codemirror/lang-sql";

/**
 * AQL-specific keywords (in addition to SQL keywords)
 */
export const AQL_KEYWORDS = [
  "CONTAINS",
  "MATCHES",
  "EXISTS",
  "EHR",
  "COMPOSITION",
  "OBSERVATION",
  "EVALUATION",
  "INSTRUCTION",
  "ACTION",
  "ADMIN_ENTRY",
  "CLUSTER",
  "ELEMENT",
  "SECTION",
];

/**
 * Standard SQL dialect extended with openEHR/AQL-specific keywords, so the
 * tokenizer highlights them the same way it highlights standard SQL
 * keywords (SELECT, FROM, WHERE, ...) rather than rendering them as plain
 * identifiers. Keywords are tagged the same as standard SQL keywords, so
 * the existing highlight style picks them up without further changes.
 */
const aqlDialect = SQLDialect.define({
  ...StandardSQL.spec,
  keywords: `${StandardSQL.spec.keywords ?? ""} ${AQL_KEYWORDS.join(" ").toLowerCase()}`,
});

/**
 * AQL language support for CodeMirror.
 * AQL is SQL-like with openEHR-specific keywords and path syntax.
 */
export function aql() {
  // Use the standard SQL dialect as base (AQL is SQL-like), extended with
  // AQL-specific keywords so they're tokenized/highlighted as keywords too.
  return sql({ dialect: aqlDialect });
}
