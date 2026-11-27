import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick } from "vue";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-fs", () => ({ readTextFile: vi.fn() }));
vi.mock("./useAnalytics", () => ({ useAnalytics: () => ({ track: vi.fn() }) }));

import { useServerStore } from "../stores/server";
import { useTemplateUpload } from "./useTemplateUpload";

type Call = { cmd: string; args: Record<string, unknown> };

function setup(activeServerId: string) {
  setActivePinia(createPinia());
  const serverStore = useServerStore();
  serverStore.activeServerId = activeServerId;
  return { serverStore, upload: useTemplateUpload() };
}

/** `upload_template` calls to a server get a duplicate (409) unless overwrite is set. */
function mockServer(calls: Call[], gate?: Promise<void>) {
  invoke.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
    calls.push({ cmd, args });
    if (cmd === "upload_template") {
      if (gate) await gate;
      if (!args.overwrite) throw new Error("Upload failed (HTTP 409): exists");
      return "Template overwritten successfully (HTTP 204)";
    }
    return [];
  });
}

function drop(text: string) {
  return { preventDefault: () => {}, dataTransfer: { files: [{ text: async () => text }] } };
}

describe("useTemplateUpload overwrite flow", () => {
  beforeEach(() => invoke.mockReset());

  it("offers an overwrite on HTTP 409 and sends it to the same server", async () => {
    const calls: Call[] = [];
    mockServer(calls);
    const { upload } = setup("A");

    await upload.handleDrop(drop("<opt/>") as unknown as DragEvent);
    expect(upload.pendingOverwrite.value).toBe("<opt/>");

    await upload.confirmOverwrite();
    const uploads = calls.filter((c) => c.cmd === "upload_template");
    expect(uploads.map((c) => [c.args.serverId, c.args.overwrite])).toEqual([
      ["A", false],
      ["A", true],
    ]);
    expect(upload.uploadStatus.value).toContain("overwritten");
    expect(upload.pendingOverwrite.value).toBeNull();
  });

  it("clears a pending overwrite when the active server changes", async () => {
    mockServer([]);
    const { serverStore, upload } = setup("A");
    await upload.handleDrop(drop("<opt/>") as unknown as DragEvent);
    expect(upload.pendingOverwrite.value).not.toBeNull();

    serverStore.activeServerId = "B";
    await nextTick();
    expect(upload.pendingOverwrite.value).toBeNull();
    expect(upload.uploadError.value).toBeNull();
  });

  it("ignores a 409 that arrives after the user switched servers", async () => {
    const calls: Call[] = [];
    let release!: () => void;
    mockServer(calls, new Promise<void>((resolve) => (release = resolve)));
    const { serverStore, upload } = setup("A");

    const inFlight = upload.handleDrop(drop("<opt/>") as unknown as DragEvent);
    await nextTick();
    serverStore.activeServerId = "B";
    await nextTick();
    release();
    await inFlight;

    // The stale 409 must not resurrect the prompt, so no overwrite can reach B.
    expect(upload.pendingOverwrite.value).toBeNull();
    await upload.confirmOverwrite();
    expect(calls.some((c) => c.cmd === "upload_template" && c.args.overwrite === true)).toBe(false);
  });
});
