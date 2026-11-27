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

  it("returns null when there is no template ID", () => {
    expect(optTemplateId("<template></template>")).toBeNull();
  });
});
