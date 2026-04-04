import { sql } from "@codemirror/lang-sql";

/**
 * AQL language support for CodeMirror.
 * AQL is SQL-like with openEHR-specific keywords and path syntax.
 */
export function aql() {
  // Start with SQL language as base since AQL is SQL-like
  const sqlLang = sql();

  // We'll use SQL highlighting as the base and customize via theme
  return sqlLang;
}

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
