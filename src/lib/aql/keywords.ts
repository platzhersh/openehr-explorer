export interface AqlKeywordItem {
  label: string;
  insertText: string;
  insertTextRules?: number; // monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet
  documentation: string;
  isSnippet?: boolean;
}

export const AQL_KEYWORDS: AqlKeywordItem[] = [
  {
    label: "SELECT",
    insertText: "SELECT ",
    documentation: "SELECT — specifies which fields to return",
  },
  {
    label: "FROM",
    insertText: "FROM ",
    documentation: "FROM — specifies the root RM type to query from",
  },
  {
    label: "WHERE",
    insertText: "WHERE ",
    documentation: "WHERE — filters results based on conditions",
  },
  {
    label: "CONTAINS",
    insertText: "CONTAINS ",
    documentation: "CONTAINS — specifies containment relationships between RM types",
  },
  {
    label: "ORDER BY",
    insertText: "ORDER BY ${1:path} ${2|ASC,DESC|}",
    documentation: "ORDER BY — sorts results by the specified path",
    isSnippet: true,
  },
  {
    label: "LIMIT",
    insertText: "LIMIT ${1:100}",
    documentation: "LIMIT — restricts the number of returned rows",
    isSnippet: true,
  },
  {
    label: "OFFSET",
    insertText: "OFFSET ${1:0}",
    documentation: "OFFSET — skips a number of rows before returning results",
    isSnippet: true,
  },
  {
    label: "AND",
    insertText: "AND ",
    documentation: "AND — logical AND operator for combining conditions",
  },
  {
    label: "OR",
    insertText: "OR ",
    documentation: "OR — logical OR operator for combining conditions",
  },
  {
    label: "NOT",
    insertText: "NOT ",
    documentation: "NOT — negates a condition",
  },
  {
    label: "AS",
    insertText: "AS ",
    documentation: "AS — assigns an alias to a selected field",
  },
  {
    label: "LIKE",
    insertText: "LIKE ",
    documentation: "LIKE — pattern matching with wildcards (% and _)",
  },
  {
    label: "MATCHES",
    insertText: "MATCHES {${1}}",
    documentation: "MATCHES — matches a value against a set or constraint",
    isSnippet: true,
  },
  {
    label: "EXISTS",
    insertText: "EXISTS ",
    documentation: "EXISTS — checks whether a path exists in the data",
  },
  {
    label: "TOP",
    insertText: "TOP ${1:10} ",
    documentation: "TOP — limits the number of returned rows (alternative to LIMIT)",
    isSnippet: true,
  },
  {
    label: "DISTINCT",
    insertText: "DISTINCT ",
    documentation: "DISTINCT — removes duplicate rows from the result set",
  },
  {
    label: "GROUP BY",
    insertText: "GROUP BY ${1:path}",
    documentation: "GROUP BY — groups results by the specified path for aggregation",
    isSnippet: true,
  },
  {
    label: "HAVING",
    insertText: "HAVING ",
    documentation: "HAVING — filters grouped results based on aggregate conditions",
  },
  {
    label: "UNION",
    insertText: "UNION ",
    documentation: "UNION — combines the results of two queries",
  },
  {
    label: "COUNT(*)",
    insertText: "COUNT(*)",
    documentation: "COUNT(*) — counts the number of rows",
  },
  {
    label: "MAX()",
    insertText: "MAX(${1:path})",
    documentation: "MAX() — returns the maximum value of a path",
    isSnippet: true,
  },
  {
    label: "MIN()",
    insertText: "MIN(${1:path})",
    documentation: "MIN() — returns the minimum value of a path",
    isSnippet: true,
  },
  {
    label: "SUM()",
    insertText: "SUM(${1:path})",
    documentation: "SUM() — returns the sum of values for a path",
    isSnippet: true,
  },
  {
    label: "AVG()",
    insertText: "AVG(${1:path})",
    documentation: "AVG() — returns the average value of a path",
    isSnippet: true,
  },
  {
    label: "ASC",
    insertText: "ASC",
    documentation: "ASC — ascending sort order",
  },
  {
    label: "DESC",
    insertText: "DESC",
    documentation: "DESC — descending sort order",
  },
];

/** RM type names offered as completions in FROM/CONTAINS clauses */
export const AQL_RM_TYPES: AqlKeywordItem[] = [
  {
    label: "EHR",
    insertText: "EHR ${1:e}",
    documentation: "EHR — the root electronic health record container",
    isSnippet: true,
  },
  {
    label: "COMPOSITION",
    insertText: "COMPOSITION ${1:c}",
    documentation: "COMPOSITION — a clinical document within an EHR",
    isSnippet: true,
  },
  {
    label: "OBSERVATION",
    insertText: "OBSERVATION ${1:obs}",
    documentation: "OBSERVATION — recorded clinical observations (e.g., blood pressure)",
    isSnippet: true,
  },
  {
    label: "EVALUATION",
    insertText: "EVALUATION ${1:eval}",
    documentation: "EVALUATION — clinical assessments and diagnoses",
    isSnippet: true,
  },
  {
    label: "INSTRUCTION",
    insertText: "INSTRUCTION ${1:ins}",
    documentation: "INSTRUCTION — orders and prescriptions",
    isSnippet: true,
  },
  {
    label: "ACTION",
    insertText: "ACTION ${1:act}",
    documentation: "ACTION — actions taken in response to instructions",
    isSnippet: true,
  },
  {
    label: "CLUSTER",
    insertText: "CLUSTER ${1:cl}",
    documentation: "CLUSTER — reusable data grouping structure",
    isSnippet: true,
  },
  {
    label: "SECTION",
    insertText: "SECTION ${1:sec}",
    documentation: "SECTION — organizational heading within a composition",
    isSnippet: true,
  },
  {
    label: "ADMIN_ENTRY",
    insertText: "ADMIN_ENTRY ${1:adm}",
    documentation: "ADMIN_ENTRY — administrative data entry",
    isSnippet: true,
  },
];
