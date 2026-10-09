import { describe, expect, it } from "vitest";
import { apiPathPrefix, apiRoot, DEFAULT_API_PATH_PREFIX } from "./apiPath";

describe("apiRoot", () => {
  it("defaults to /rest/openehr/v1 when no prefix is set", () => {
    expect(DEFAULT_API_PATH_PREFIX).toBe("/rest/openehr/v1");
    expect(apiRoot("https://cdr.example.com")).toBe("https://cdr.example.com/rest/openehr/v1");
    expect(apiRoot("https://cdr.example.com/", null)).toBe(
      "https://cdr.example.com/rest/openehr/v1",
    );
  });

  it("uses a custom prefix and tolerates stray slashes", () => {
    expect(apiRoot("https://cdr.example.com/", "/openehr/v1")).toBe(
      "https://cdr.example.com/openehr/v1",
    );
    expect(apiRoot("https://cdr.example.com", "openehr/v1/")).toBe(
      "https://cdr.example.com/openehr/v1",
    );
  });

  it("treats an empty or slash-only prefix as 'base URL is the API root'", () => {
    expect(apiRoot("https://cdr.example.com/openehr/v1/", "")).toBe(
      "https://cdr.example.com/openehr/v1",
    );
    expect(apiRoot("https://cdr.example.com", "/")).toBe("https://cdr.example.com");
  });
});

describe("apiPathPrefix", () => {
  it("returns a normalized path-only prefix", () => {
    expect(apiPathPrefix(undefined)).toBe("/rest/openehr/v1");
    expect(apiPathPrefix("openehr/v1/")).toBe("/openehr/v1");
    expect(apiPathPrefix("/")).toBe("");
  });
});
