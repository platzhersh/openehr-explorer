import { describe, expect, it } from "vitest";
import { classifyError, createErrorLimiter, locationFromError, scrubLocation } from "./errorReport";

describe("classifyError", () => {
  it("maps values to a fixed enum, never free text", () => {
    expect(classifyError(new TypeError("secret https://cdr.example/ehr/123"))).toBe("TypeError");
    expect(classifyError("backend said: AQL SELECT ...")).toBe("string");
    expect(classifyError({ message: "x" })).toBe("object");
    expect(classifyError(undefined)).toBe("other");
    class Custom extends Error {
      override name = "CustomLeakyName";
    }
    expect(classifyError(new Custom("x"))).toBe("other");
  });
});

describe("scrubLocation", () => {
  it("keeps only the file basename and line", () => {
    expect(scrubLocation("tauri://localhost/assets/index-ab12.js?x=1#h", 3)).toBe(
      "index-ab12.js:3",
    );
    expect(scrubLocation("C:\\Users\\bob\\app\\main.js", 9)).toBe("main.js:9");
    expect(scrubLocation(undefined, 1)).toBe("unknown");
    expect(scrubLocation("a.js", undefined)).toBe("a.js:0");
  });
});

describe("locationFromError", () => {
  it("extracts a scrubbed first-frame location", () => {
    const err = new Error("boom");
    err.stack = "Error: boom\n    at f (tauri://localhost/assets/index-ab12.js:10:5)";
    expect(locationFromError(err)).toBe("index-ab12.js:10");
  });

  it("falls back to unknown", () => {
    expect(locationFromError("nope")).toBe("unknown");
    const err = new Error("x");
    err.stack = undefined;
    expect(locationFromError(err)).toBe("unknown");
  });
});

describe("createErrorLimiter", () => {
  it("dedupes identical errors", () => {
    const send = createErrorLimiter();
    const props = { kind: "TypeError", location: "a.js:1" };
    expect(send("js_error", props)).toBe(true);
    expect(send("js_error", props)).toBe(false);
    expect(send("promise_rejected", props)).toBe(true);
  });

  it("caps total events per session", () => {
    const send = createErrorLimiter(2);
    expect(send("js_error", { kind: "Error", location: "a.js:1" })).toBe(true);
    expect(send("js_error", { kind: "Error", location: "a.js:2" })).toBe(true);
    expect(send("js_error", { kind: "Error", location: "a.js:3" })).toBe(false);
  });
});
