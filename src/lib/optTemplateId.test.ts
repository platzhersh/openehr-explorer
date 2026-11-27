import { describe, expect, it } from "vitest";
import { optTemplateId } from "./optTemplateId";

describe("optTemplateId", () => {
  it("reads the template ID of a default-namespace OPT", () => {
    const xml = `<template xmlns="http://schemas.openehr.org/v1"><language><code_string>en</code_string></language><template_id><value>vital_signs.v1</value></template_id></template>`;
    expect(optTemplateId(xml)).toBe("vital_signs.v1");
  });

  it("reads a namespace-prefixed template ID", () => {
    expect(
      optTemplateId(
        "<o:template><o:template_id><o:value>ns.v1</o:value></o:template_id></o:template>",
      ),
    ).toBe("ns.v1");
  });

  it("decodes entities and trims whitespace", () => {
    expect(
      optTemplateId("<template><template_id>\n <value> A&amp;B </value></template_id></template>"),
    ).toBe("A&B");
  });

  it("reads CDATA content and numeric character references in full", () => {
    expect(
      optTemplateId(
        "<template><template_id><value><![CDATA[cd<1>.v2]]></value></template_id></template>",
      ),
    ).toBe("cd<1>.v2");
    expect(
      optTemplateId("<template><template_id><value>&#65;B&#x43;</value></template_id></template>"),
    ).toBe("ABC");
    expect(
      optTemplateId(
        "<template><template_id><value>x<![CDATA[y]]>&amp;z</value></template_id></template>",
      ),
    ).toBe("xy&z");
  });

  it("returns null instead of a partial ID when the value can't be fully decoded", () => {
    expect(
      optTemplateId("<template><template_id><value>a&unknown;b</value></template_id></template>"),
    ).toBeNull();
    expect(
      optTemplateId("<template><template_id><value>a<b/>c</value></template_id></template>"),
    ).toBeNull();
  });

  it("ignores a commented-out template ID, like the backend does", () => {
    const xml =
      "<!-- <template_id><value>example.v1</value></template_id> -->" +
      "<template><template_id><value>live.v1</value></template_id></template>";
    expect(optTemplateId(xml)).toBe("live.v1");
  });

  it("returns null when there is no template ID", () => {
    expect(optTemplateId("<template></template>")).toBeNull();
  });
});
