import { describe, expect, it } from "vitest";
import { authLabel, tokenHost, validateOAuth2, validateTokenUrl } from "./oauth";

const base = {
  type: "oauth2_client_credentials" as const,
  token_url: "https://t.auth.prod.cadasto.io/oauth/token",
  client_id: "cid",
  client_secret: "secret",
  audience: null,
  scope: null,
  client_auth: "basic" as const,
};

describe("validateTokenUrl", () => {
  it("requires a value", () => expect(validateTokenUrl("  ")).toMatch(/required/));
  it("accepts https", () => expect(validateTokenUrl(base.token_url)).toBeNull());
  it("accepts http on localhost", () => {
    expect(validateTokenUrl("http://localhost:8080/token")).toBeNull();
    expect(validateTokenUrl("http://127.0.0.1/token")).toBeNull();
  });
  it("rejects http on other hosts", () =>
    expect(validateTokenUrl("http://auth.example.com/token")).toMatch(/https/));
  it("rejects garbage", () => expect(validateTokenUrl("not a url")).toMatch(/valid/));
});

describe("validateOAuth2", () => {
  it("accepts a complete config", () => expect(validateOAuth2(base, false)).toBeNull());
  it("requires client id", () =>
    expect(validateOAuth2({ ...base, client_id: " " }, false)).toMatch(/Client ID/));
  it("requires a secret for new profiles", () =>
    expect(validateOAuth2({ ...base, client_secret: "" }, false)).toMatch(/secret/));
  it("allows a blank secret when one is stored", () =>
    expect(validateOAuth2({ ...base, client_secret: "" }, true)).toBeNull());
});

describe("authLabel", () => {
  it("shows OAuth2 with the token host", () => {
    expect(
      authLabel({
        type: "oauth2_client_credentials",
        token_url: base.token_url,
        client_id: "c",
        has_client_secret: true,
        client_auth: "basic",
      }),
    ).toBe("OAuth2 (client credentials) · t.auth.prod.cadasto.io");
  });
  it("keeps legacy labels", () => {
    expect(authLabel({ type: "basic", username: "u", has_password: true })).toBe("basic");
    expect(authLabel({ type: "none" })).toBe("none");
  });
  it("tokenHost tolerates invalid urls", () => expect(tokenHost("nope")).toBeNull());
});
