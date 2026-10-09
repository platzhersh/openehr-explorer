import { describe, expect, it } from "vitest";
import {
  apiRoot,
  CADASTO_API_PATH_PREFIX,
  customApiPrefixLabel,
  DEFAULT_API_PATH_PREFIX,
  defaultApiPathPrefix,
} from "./apiPath";

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

describe("server type defaults", () => {
  it("uses /openehr/v1 for Cadasto and the REST default otherwise", () => {
    expect(CADASTO_API_PATH_PREFIX).toBe("/openehr/v1");
    expect(defaultApiPathPrefix("cadasto")).toBe("/openehr/v1");
    expect(defaultApiPathPrefix("ehrbase")).toBe("/rest/openehr/v1");
    expect(defaultApiPathPrefix(undefined)).toBe("/rest/openehr/v1");
  });

  it("builds the Cadasto API root from a blank prefix", () => {
    expect(apiRoot("https://cdr.example.com", null, "cadasto")).toBe(
      "https://cdr.example.com/openehr/v1",
    );
  });

  it("lets an explicit prefix win over the type default", () => {
    expect(apiRoot("https://cdr.example.com", "/rest/openehr/v1", "cadasto")).toBe(
      "https://cdr.example.com/rest/openehr/v1",
    );
  });

  it("labels a prefix only when it differs from the type's default", () => {
    expect(customApiPrefixLabel(null, "cadasto")).toBeNull();
    expect(customApiPrefixLabel("/openehr/v1", "cadasto")).toBeNull();
    expect(customApiPrefixLabel("/openehr/v1", "ehrbase")).toBe("/openehr/v1");
    expect(customApiPrefixLabel("/rest/openehr/v1", "cadasto")).toBe("/rest/openehr/v1");
  });
});
