import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { trackModifierHeld } from "./useModifierHeld";

function setup(isMac: boolean) {
  const target = new EventTarget();
  let held = false;
  const stop = trackModifierHeld(target, isMac, 300, (v) => (held = v));
  const key = (type: "keydown" | "keyup", k: string) =>
    target.dispatchEvent(Object.assign(new Event(type), { key: k, repeat: false }));
  return { target, stop, key, held: () => held };
}

describe("trackModifierHeld", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("shows after the delay and hides on keyup", () => {
    const { key, held } = setup(true);
    key("keydown", "Meta");
    expect(held()).toBe(false);
    vi.advanceTimersByTime(300);
    expect(held()).toBe(true);
    key("keyup", "Meta");
    expect(held()).toBe(false);
  });

  it("does not flash on a quick Cmd+key press", () => {
    const { key, held } = setup(true);
    key("keydown", "Meta");
    vi.advanceTimersByTime(100);
    key("keyup", "Meta");
    vi.advanceTimersByTime(500);
    expect(held()).toBe(false);
  });

  it("hides on window blur", () => {
    const { target, key, held } = setup(true);
    key("keydown", "Meta");
    vi.advanceTimersByTime(300);
    target.dispatchEvent(new Event("blur"));
    expect(held()).toBe(false);
  });

  it("uses Control on non-mac platforms and ignores Meta", () => {
    const { key, held } = setup(false);
    key("keydown", "Meta");
    vi.advanceTimersByTime(300);
    expect(held()).toBe(false);
    key("keydown", "Control");
    vi.advanceTimersByTime(300);
    expect(held()).toBe(true);
  });

  it("stops listening after stop()", () => {
    const { stop, key, held } = setup(true);
    stop();
    key("keydown", "Meta");
    vi.advanceTimersByTime(300);
    expect(held()).toBe(false);
  });
});
