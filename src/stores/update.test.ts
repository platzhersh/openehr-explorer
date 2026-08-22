import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/plugin-updater", () => ({
  check: vi.fn(),
}));
vi.mock("@tauri-apps/plugin-process", () => ({
  relaunch: vi.fn(),
}));

import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useUpdateStore } from "./update";

const ridByInstance = new WeakMap<object, number>();

/**
 * Minimal stand-in for `@tauri-apps/api`'s `Resource` class, which the real
 * `Update` class extends. `Resource` stores its `rid` behind a WeakMap keyed
 * by object identity (TypeScript's private-field emulation) — reading
 * `this.rid` through a Vue reactive Proxy (a *different* object than the one
 * used as the WeakMap key) throws exactly like the real class does. This
 * reproduces that failure mode without depending on Tauri's IPC runtime.
 */
class FakeResource {
  constructor(rid: number) {
    ridByInstance.set(this, rid);
  }

  get rid(): number {
    const rid = ridByInstance.get(this);
    if (rid === undefined) {
      throw new TypeError(
        "Cannot read private member from an object whose class did not declare it",
      );
    }
    return rid;
  }
}

class FakeUpdate extends FakeResource {
  available = true;
  version = "0.4.3";
  currentVersion = "0.4.2";

  async downloadAndInstall(
    onEvent?: (progress: { event: string; data: { contentLength?: number } }) => void,
  ): Promise<void> {
    // Reading `this.rid` is what actually breaks if the store ever wraps
    // this instance in a plain `ref()` instead of `shallowRef()` — see the
    // comment on `update` in update.ts.
    void this.rid;
    onEvent?.({ event: "Started", data: { contentLength: 100 } });
  }
}

describe("useUpdateStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.mocked(check).mockReset();
    vi.mocked(relaunch).mockReset();
  });

  it("marks an update available and keeps the raw instance (not a reactive proxy)", async () => {
    const fakeUpdate = new FakeUpdate(42);
    vi.mocked(check).mockResolvedValue(fakeUpdate as unknown as Update);

    const store = useUpdateStore();
    await store.checkForUpdates();

    expect(store.status).toBe("available");
    // Regression guard: a plain `ref()` deep-wraps object values in a
    // reactive Proxy on assignment, so `store.update` would no longer be
    // `===` to the object `check()` resolved with. `shallowRef()` must keep
    // the identity intact.
    expect(store.update).toBe(fakeUpdate);
  });

  it("reports up-to-date when no update is found", async () => {
    vi.mocked(check).mockResolvedValue(null);

    const store = useUpdateStore();
    await store.checkForUpdates();

    expect(store.status).toBe("up-to-date");
    expect(store.update).toBeNull();
  });

  it("surfaces a check failure without throwing", async () => {
    vi.mocked(check).mockRejectedValue(new Error("network down"));

    const store = useUpdateStore();
    await store.checkForUpdates();

    expect(store.status).toBe("error");
    expect(store.error).toBe("network down");
  });

  it("regression: downloadAndInstall does not hit the private-field error that a plain ref() would cause", async () => {
    const fakeUpdate = new FakeUpdate(42);
    vi.mocked(check).mockResolvedValue(fakeUpdate as unknown as Update);
    vi.mocked(relaunch).mockResolvedValue(undefined);

    const store = useUpdateStore();
    await store.checkForUpdates();
    await store.downloadAndInstall();

    // If `update` were a plain `ref()`, `downloadAndInstall`'s internal
    // `this.rid` read would throw inside FakeUpdate, and the store's
    // try/catch would land here as `status: "error"` instead.
    expect(store.error).toBeNull();
    expect(relaunch).toHaveBeenCalledOnce();
  });
});
