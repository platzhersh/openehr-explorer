import * as monaco from "monaco-editor";

export const AQL_LANGUAGE_ID = "aql";

export function registerAqlLanguage(): void {
  if (monaco.languages.getLanguages().some((lang) => lang.id === AQL_LANGUAGE_ID)) {
    return;
  }

  monaco.languages.register({ id: AQL_LANGUAGE_ID });

  monaco.languages.setMonarchTokensProvider(AQL_LANGUAGE_ID, {
    ignoreCase: true,

    keywords: [
      "SELECT",
      "FROM",
      "WHERE",
      "CONTAINS",
      "ORDER",
      "BY",
      "LIMIT",
      "OFFSET",
      "AND",
      "OR",
      "NOT",
      "AS",
      "LIKE",
      "MATCHES",
      "EXISTS",
      "TOP",
      "DISTINCT",
      "GROUP",
      "HAVING",
      "UNION",
      "ASC",
      "DESC",
    ],

    rmTypes: [
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
      "PARTY_PROXY",
      "PARTY_IDENTIFIED",
      "PARTY_RELATED",
      "PARTY_SELF",
    ],

    aggregates: ["COUNT", "MAX", "MIN", "SUM", "AVG"],

    tokenizer: {
      root: [
        // Comments
        [/--.*$/, "comment"],

        // String literals
        [/'[^']*'/, "string"],
        [/"[^"]*"/, "string"],

        // Archetype IDs: openEHR-EHR-OBSERVATION.blood_pressure.v2
        [/openEHR-[\w.-]+/, "type.archetype"],

        // Archetype node IDs: at0001, id1
        [/\b(at|id)\d+\b/, "type.archetype"],

        // Aggregate functions with parens
        [
          /\b(COUNT|MAX|MIN|SUM|AVG)\s*\(/,
          { cases: { "@default": [{ token: "keyword.function" }, { token: "@brackets" }] } },
        ],

        // Identifiers and keywords
        [
          /[a-zA-Z_][\w]*/,
          {
            cases: {
              "@keywords": "keyword",
              "@rmTypes": "type.rm",
              "@aggregates": "keyword.function",
              "@default": "identifier",
            },
          },
        ],

        // Path expressions: /data[at0001]/events[at0006]
        [/\/[\w\[\]@/.]+/, "tag.path"],

        // Numbers
        [/\d+(\.\d+)?/, "number"],

        // Brackets
        [/[{}()\[\]]/, "@brackets"],

        // Operators
        [/[<>!=]+/, "operator"],
        [/[,;]/, "delimiter"],

        // Whitespace
        [/\s+/, "white"],
      ],
    },
  });
}
