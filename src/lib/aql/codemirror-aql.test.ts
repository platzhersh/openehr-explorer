import { describe, expect, it } from "vitest";
import { EditorState } from "@codemirror/state";
import { syntaxTree } from "@codemirror/language";
import { aql, AQL_KEYWORDS } from "./codemirror-aql";

/**
 * Returns the CodeMirror node type name for each distinct word in `doc`,
 * keyed by its exact text. Used to assert what the tokenizer/highlighter
 * actually classifies a given word as, rather than just checking the parser
 * didn't throw. Walking every node (not just leaves) is fine here: wrapper
 * nodes like "Script"/"Statement" span the whole query and never collide
 * with a single keyword's text, so single-word tokens are captured cleanly.
 */
function tokenTypes(doc: string): Record<string, string> {
  const state = EditorState.create({ doc, extensions: [aql()] });
  const cursor = syntaxTree(state).cursor();
  const types: Record<string, string> = {};
  do {
    const text = doc.slice(cursor.from, cursor.to);
    if (text.trim()) types[text] = cursor.type.name;
  } while (cursor.next());
  return types;
}

describe("aql() CodeMirror language support", () => {
  // A regression test for OEH-19: AQL_KEYWORDS existing as a constant is not
  // enough — it has to actually reach the dialect's keyword table, and doing
  // so must not clobber the standard SQL keywords that were already
  // highlighted correctly (the bug in the first attempt at this fix).
  const query = `SELECT e FROM EHR e CONTAINS COMPOSITION c CONTAINS ${AQL_KEYWORDS.join(" c CONTAINS ")} c WHERE MATCHES {'a'} AND EXISTS c/uid LIMIT 20`;
  const types = tokenTypes(query);

  it.each(AQL_KEYWORDS)("tags AQL keyword %s as a Keyword token", (keyword) => {
    expect(types[keyword]).toBe("Keyword");
  });

  it.each(["SELECT", "FROM", "WHERE", "AND", "LIMIT"])(
    "still tags standard SQL keyword %s as a Keyword token",
    (keyword) => {
      expect(types[keyword]).toBe("Keyword");
    },
  );

  it("does not tag identifiers as keywords", () => {
    expect(types["e"]).toBe("Identifier");
    expect(types["c"]).toBe("Identifier");
  });
});
