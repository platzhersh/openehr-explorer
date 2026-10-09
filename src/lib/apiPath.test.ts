import { describe, expect, it } from "vitest";
import { apiRoot, customApiPrefixLabel, DEFAULT_API_PATH_PREFIX } from "./apiPath";

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

describe("customApiPrefixLabel", () => {
  it("is null for the default prefix, whether unset or stored explicitly", () => {
    expect(customApiPrefixLabel(undefined)).toBeNull();
    expect(customApiPrefixLabel(null)).toBeNull();
    expect(customApiPrefixLabel("/rest/openehr/v1")).toBeNull();
    expect(customApiPrefixLabel("rest/openehr/v1/")).toBeNull();
  });

  it("returns the normalized custom prefix", () => {
    expect(customApiPrefixLabel("openehr/v1/")).toBe("/openehr/v1");
  });

  it("shows an explicit empty prefix as '/'", () => {
    expect(customApiPrefixLabel("")).toBe("/");
    expect(customApiPrefixLabel("/")).toBe("/");
  });
});
