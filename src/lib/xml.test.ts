import { describe, expect, it } from "vitest";
import { parseXmlLines, xmlLinesToText } from "./xml";

describe("parseXmlLines", () => {
  it("indents nested elements and dedents on close", () => {
    const lines = parseXmlLines("<a><b><c/></b></a>");
    expect(lines.map((l) => l.depth)).toEqual([0, 1, 2, 1, 0]);
  });

  it("keeps inline text content on the same line as its tags", () => {
    const lines = parseXmlLines("<value>Some text</value>");
    expect(lines).toHaveLength(1);
    expect(lines[0].tokens.map((t) => t.text).join("")).toBe("<value>Some text</value>");
  });

  it("highlights namespaced tag and attribute names (the OEH-35 regression)", () => {
    const lines = parseXmlLines('<xs:string xs:foo="bar"/>');
    const tagTokens = lines[0].tokens.filter((t) => t.type === "tag");
    const attrTokens = lines[0].tokens.filter((t) => t.type === "attr-name");
    expect(tagTokens.map((t) => t.text)).toEqual(["xs:string"]);
    expect(attrTokens.map((t) => t.text)).toEqual(["xs:foo"]);
  });

  it("tokenizes attributes with name/value pairs", () => {
    const lines = parseXmlLines("<a id=\"42\" name='x'/>");
    const types = lines[0].tokens.map((t) => t.type);
    expect(types).toEqual([
      "bracket",
      "tag",
      "punct",
      "attr-name",
      "punct",
      "attr-value",
      "punct",
      "attr-name",
      "punct",
      "attr-value",
      "bracket",
    ]);
  });

  it("tags XML declarations as a single decl token", () => {
    const lines = parseXmlLines('<?xml version="1.0" encoding="UTF-8"?><root/>');
    expect(lines[0].tokens).toEqual([
      { type: "decl", text: '<?xml version="1.0" encoding="UTF-8"?>' },
    ]);
  });

  it("tags comments as a single comment token", () => {
    const lines = parseXmlLines("<!-- a note --><root/>");
    expect(lines[0].tokens).toEqual([{ type: "comment", text: "<!-- a note -->" }]);
  });

  it("does not increment depth for a self-closing tag", () => {
    const lines = parseXmlLines("<a><b/><c/></a>");
    expect(lines.map((l) => l.depth)).toEqual([0, 1, 1, 0]);
  });

  it("round-trips through xmlLinesToText with 2-space indentation", () => {
    const lines = parseXmlLines("<a><b>text</b></a>");
    expect(xmlLinesToText(lines)).toBe("<a>\n  <b>text</b>\n</a>");
  });
});
