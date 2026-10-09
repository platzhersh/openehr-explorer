import { describe, expect, it, vi } from "vitest";
import { trackEscapeKey } from "./useEscapeKey";

function press(target: EventTarget, key = "Escape") {
  const event = Object.assign(new Event("keydown", { cancelable: true }), { key });
  target.dispatchEvent(event);
  return event;
}

describe("trackEscapeKey", () => {
  it("calls the handler on Escape and ignores other keys", () => {
    const target = new EventTarget();
    const close = vi.fn();
    const stop = trackEscapeKey(target, close);
    press(target, "Enter");
    expect(close).not.toHaveBeenCalled();
    press(target);
    expect(close).toHaveBeenCalledTimes(1);
    stop();
  });

  it("stops listening after the returned stop function runs", () => {
    const target = new EventTarget();
    const close = vi.fn();
    trackEscapeKey(target, close)();
    press(target);
    expect(close).not.toHaveBeenCalled();
  });

  it("skips events another handler already prevented", () => {
    const target = new EventTarget();
    const close = vi.fn();
    // Registered first, so it runs before the Escape listener (as an element handler would).
    target.addEventListener("keydown", (e) => e.preventDefault());
    const stop = trackEscapeKey(target, close);
    press(target);
    expect(close).not.toHaveBeenCalled();
    stop();
  });

  it("only closes the topmost dialog, then the next one on the following press", () => {
    const target = new EventTarget();
    const bottom = vi.fn();
    const top = vi.fn();
    const stopBottom = trackEscapeKey(target, bottom);
    const stopTop = trackEscapeKey(target, top);
    press(target);
    expect(top).toHaveBeenCalledTimes(1);
    expect(bottom).not.toHaveBeenCalled();
    stopTop();
    press(target);
    expect(bottom).toHaveBeenCalledTimes(1);
    stopBottom();
  });
});
