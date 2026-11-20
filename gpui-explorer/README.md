# openEHR Explorer — GPUI clone (just for fun)

A ~700-line [GPUI](https://crates.io/crates/gpui) (Zed's GPU UI framework) port of four screens of
openEHR Explorer: **Dashboard, EHR Browser, Templates (Web Template tree), AQL Runner**.
It is a toy: data is in-process and mirrors `scripts/demo-assets/mock.js`; there is no networking,
text editing, server profiles, Request Inspector, composition viewer, etc.

```bash
cd gpui-explorer
cargo build --release
./target/release/openehr-explorer-gpui --view aql --run --rows 10000
#   --view dashboard|ehrs|templates|aql   --ehrs N   --rows N   --select   --run   --bench-scroll
```

`xattr 0.2.3` (transitive, via gpui's http client) fails against current `libc` (`ENOATTR` removed on
Linux), so `vendor/xattr` is a one-line patched copy (`ENOATTR` → `ENODATA`).

## Reproducing the screenshots / numbers (headless Linux)

GPUI needs a Vulkan device and a window manager (its X11 refresh loop only starts after Map/Visibility
events). Here: Xvfb + openbox + Mesa lavapipe (software Vulkan), see `bench/xenv.sh`.

```bash
./shot.sh screenshots/gpui-04-aql.png --view aql --run         # GPUI screenshot
bench/gpui-bench.sh                                            # GPUI numbers
(cd .. && npm ci && npx vite build && npx vite preview --port 4173 &)
node bench/vue-bench.mjs .. screenshots                        # real Vue app numbers + screenshots
```

The reference is the **real Vue frontend** (production `vite build`) in headless Chromium, with the repo's own
Tauri-IPC mock (`scripts/demo-assets/mock.js`) — *not* the native Tauri/WebKitGTK shell.

## Results (4 vCPU container, no GPU, everything software-rendered)

| | Vue (Chromium) | GPUI |
|---|---|---|
| Startup → first UI (median of 5) | 111 ms browser launch + 373 ms page load ≈ **485 ms** | **~70 ms** (spawn → first `render()`) |
| Idle memory (PSS) | **348 MB** (8 processes) | **104 MB** (1 process) |
| Frontend artifact | 1.1 MB `dist/` (+ a ~290 MB browser runtime) | 25 MB self-contained binary |

AQL Runner, click *Run* → rows on screen, then scroll the whole table in 300 frames:

| Rows | Vue: run→render | Vue: memory | Vue: DOM nodes | Vue: scroll fps (p95 frame) | GPUI: run→render | GPUI: memory | GPUI: `render()` CPU p95 |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 3 | 29 ms | 388 MB | – | – | 1.3 ms | 105 MB | 0.04 ms |
| 1,000 | 90 ms | 419 MB | 10,248 | 60 (16.7 ms) | 1.8 ms | 109 MB | 0.04 ms |
| 10,000 | 927 ms | 737 MB | 100,248 | 44 (33 ms) | 6.5 ms | 110 MB | 0.06 ms |
| 50,000 | 4.7 s | 2.1 GB | 500,248 | **6.7 (233 ms)** | 26 ms | 122 MB | 0.04 ms |
| 1,000,000 | – | – | – | – | 513 ms | 381 MB | 0.05 ms |

## Read this before believing the numbers

* **The AQL comparison is partly architecture, not framework.** The real app renders results as a plain
  `<table>` (no virtualization); the GPUI clone uses `uniform_list`, which only builds visible rows.
  Virtualizing the Vue table would close most of the gap at 10k–50k rows.
* **GPUI frame rate is capped by software rendering here.** GPUI stays at ~31 fps (32 ms frames) at every row
  count: that is lavapipe rasterizing a full 1440×960 frame + Xvfb present, not list cost. On a real GPU
  it would not be limited this way. Use the `render()` CPU column for the framework's own cost. Chromium
  scrolls with the compositor, so it hits 60 fps on small tables.
* GPUI startup is measured to the first `render()` call, which excludes the first present (≈ one frame);
  Chromium's is to two animation frames after the sidebar appears and includes browser process launch.
  The real Tauri app spawns WebKitGTK rather than Chromium (not measured; likely comparable memory).
* GPUI's "run→render" is the time to build the rows in-process; Vue's includes the mocked IPC, reactive
  update, layout and two animation frames. The real app would add JSON (de)serialization over IPC.
* Memory is PSS (shared pages split across processes), summed over the Chromium process tree.
* One run per scenario (startup: median of 5). Treat differences under ~20% as noise.
