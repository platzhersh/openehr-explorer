import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick } from "vue";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-fs", () => ({ readTextFile: vi.fn() }));
vi.mock("./useAnalytics", () => ({ useAnalytics: () => ({ track: vi.fn() }) }));

import { useServerStore, type ServerProfile } from "../stores/server";
import { useTemplateUpload } from "./useTemplateUpload";

type Call = { cmd: string; args: Record<string, unknown> };

function setup(activeServerId: string, serverType: ServerProfile["server_type"] = "ehrbase") {
  setActivePinia(createPinia());
  const serverStore = useServerStore();
  serverStore.profiles = [
    { id: activeServerId, name: "Local server", server_type: serverType },
  ] as ServerProfile[];
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
    if (cmd === "parse_opt_template_id") return "vital_signs.v1";
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
    // The confirmation names the template as the backend reads it, not a second parser.
    expect(upload.pendingTemplateId.value).toBe("vital_signs.v1");

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

  it("ignores a late 409 from a superseded upload so the prompt keeps its own OPT", async () => {
    const releases: Array<() => void> = [];
    invoke.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
      if (cmd !== "upload_template") return cmd === "parse_opt_template_id" ? null : [];
      await new Promise<void>((resolve) => releases.push(resolve));
      if (!args.overwrite) throw new Error("Upload failed (HTTP 409): exists");
      return "Template overwritten successfully (HTTP 204)";
    });
    const { upload } = setup("A");

    const first = upload.handleDrop(drop("<first/>") as unknown as DragEvent);
    await nextTick();
    const second = upload.handleDrop(drop("<second/>") as unknown as DragEvent);
    await nextTick();
    // The newer upload answers first and opens the prompt; the older one answers late.
    releases[1]();
    await second;
    expect(upload.pendingOverwrite.value).toBe("<second/>");
    releases[0]();
    await first;

    expect(upload.pendingOverwrite.value).toBe("<second/>");
  });

  it("ignores a 409 from an upload that started before the state was reset", async () => {
    let release!: () => void;
    mockServer([], new Promise<void>((resolve) => (release = resolve)));
    const { upload } = setup("A");

    const inFlight = upload.handleDrop(drop("<opt/>") as unknown as DragEvent);
    await nextTick();
    upload.resetState(); // e.g. the modal was reopened
    release();
    await inFlight;

    expect(upload.pendingOverwrite.value).toBeNull();
    expect(upload.uploadError.value).toBeNull();
  });

  it("still refreshes the template list when a superseded upload succeeds", async () => {
    const calls: Call[] = [];
    const releases: Array<() => void> = [];
    invoke.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
      calls.push({ cmd, args });
      if (cmd === "upload_template") {
        await new Promise<void>((resolve) => releases.push(resolve));
        return "Template uploaded successfully (HTTP 201)";
      }
      return cmd === "parse_opt_template_id" ? null : [];
    });
    const { upload } = setup("A");

    const first = upload.handleDrop(drop("<a/>") as unknown as DragEvent);
    await nextTick();
    const second = upload.handleDrop(drop("<b/>") as unknown as DragEvent);
    await nextTick();
    releases[1]();
    await second;
    releases[0](); // the older upload finishes last, after a newer attempt owns the state
    await first;

    expect(calls.filter((c) => c.cmd === "list_templates")).toHaveLength(2);
  });

  it("does not offer an overwrite on server types that can't do one", async () => {
    mockServer([]);
    const { upload } = setup("A", "generic");

    await upload.handleDrop(drop("<opt/>") as unknown as DragEvent);

    expect(upload.pendingOverwrite.value).toBeNull();
    expect(upload.uploadError.value).toContain("isn't supported for this server type");
  });

  it("explains immutability when a duplicate is refused on FerroEHR", async () => {
    mockServer([]);
    const { upload } = setup("A", "ferro_ehr");

    await upload.handleDrop(drop("<opt/>") as unknown as DragEvent);

    expect(upload.pendingOverwrite.value).toBeNull();
    expect(upload.uploadError.value).toContain("immutable");
  });
});
