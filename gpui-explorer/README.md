# openEHR Explorer — GPUI clone (just for fun)

A ~700-line [GPUI](https://crates.io/crates/gpui) (Zed's GPU UI framework) port of four screens of
openEHR Explorer: **Dashboard, EHR Browser, Templates (Web Template tree), AQL Runner**.
It is a toy. By default data is in-process and mirrors `scripts/demo-assets/mock.js`. With `--server` it talks to a
real openEHR CDR (see below). Still missing: Request Inspector, filters/search/sort in EHR Browser, horizontal scrolling for wide content, composition
versions/directory/contributions, writes (create/edit/delete).

```bash
cd gpui-explorer
cargo build --release
./target/release/openehr-explorer-gpui --view aql --run --rows 10000
#   --view dashboard|ehrs|templates|aql   --ehrs N   --rows N   --select   --run   --bench-scroll
```

`xattr 0.2.3` (transitive, via gpui's http client) fails against current `libc` (`ENOATTR` removed on
Linux), so `vendor/xattr` is a one-line patched copy (`ENOATTR` → `ENODATA`).

## Network mode

```bash
export OEH_USER=ehrbase-user OEH_PASSWORD=ehrbase-password      # or OEH_TOKEN for bearer auth; never on argv
./target/release/openehr-explorer-gpui --server http://localhost:8080/ehrbase --view ehrs
#   --api-prefix /rest/openehr/v1   (default; "" means base URL is the openEHR root)
#   --query "SELECT ... FROM EHR e CONTAINS COMPOSITION c LIMIT 50" --view aql --run
```

`src/net.rs` makes the same calls as the Rust backend: `/rest/status` (EHRBase) with a template-list fallback for
connection tests, EHR paging via AQL (`LIMIT n+1`/`OFFSET`, retrying without `ORDER BY` when the CDR rejects it, as
EHRBase does), `/definition/template/adl1.4` (+ Web Template with `application/openehr.wt+json`), and
`/query/aql`. A shared Tokio runtime does the I/O; results are applied on GPUI's thread. Rows are stored flat
(`Vec<SharedString>` + column count), so arbitrary AQL result shapes cost no extra per-row allocation.

Tested two ways. `bench/mock-cdr.mjs` is an EHRBase-like mock (Basic auth, rejects `ORDER BY` on EHR attributes) used for
auth, paging and analytics checks. It was also run, **read-only**, against the public sandbox
`https://sandbox.ehrbase.org/ehrbase` (EHRBase 2.33.0, reads need no credentials): EHR paging, 506 templates with
Web Template trees, compositions, and AQL all work (`gpui-sandbox-*.png`). That run caught two bugs the mock could not:
`/rest/status` answers XML unless `Accept: application/json` is sent, and the template list needs to be virtualized.
EHRBase 2.33 still rejects `ORDER BY` on EHR attributes, so the retry path is needed. Not tested against Better Platform,
and writes are not implemented.

## Server profiles (Servers tab) — compatible with the Tauri app

The **Servers** tab reads and writes the *same* profiles the Tauri app uses, so you can switch between the two:

* `<config>/openehr-explorer/profiles.json` (`$XDG_CONFIG_HOME`, else `~/.config`, exactly as the Tauri app resolves it),
  same JSON shape, secrets stripped.
* Secrets go to the OS keychain (service `openehr-explorer`, account `<profile id>:<field>`), or the encrypted-file
  fallback (`credentials.enc` + `.key`). This uses the **very same `credentials.rs`** the Tauri app compiles
  (`#[path = "../../src-tauri/src/credentials.rs"]`), so the formats cannot drift.
* Behaviour mirrors the Tauri commands: empty password/token on save = keep the stored one, saving preserves the default
  flag, delete removes all four secret fields, "Set default" toggles, and the active profile on launch is the default one
  (else the first). Test Connection is the same authenticated `GET …/definition/template/adl1.4`.

Form: name, base URL, server type, auth (none/basic/bearer), API path prefix, terminology URL, with a small built-in text
field (typing, paste, arrows, Home/End, Tab, `Ctrl+U` clear, Enter = save, Esc = cancel; no IME or selection yet). Admin
credentials are preserved as stored but not editable here. Launching with no flags connects to the active stored profile;
use `--demo` for the built-in demo data, or `--server URL` to override.

Verified two ways: unit tests against a `profiles.json` fixture in the Tauri format, and a cross-check running the Tauri
app's own `load_stored_profiles`/`get_profile_by_id`/`save_server_profile` against files written by this app and vice versa.

## Every tab follows the active server profile

Dashboard, EHR Browser (including its composition screen), Templates and AQL Runner all read from the **active profile** (the default one, else
the first; change it with *Use* on the Servers tab, which reloads everything). Each tab header names the server it is
showing, or says "demo data" when no profile is active.

* **Dashboard ("Overview"):** the stat cards are links like in the Tauri app (EHRs and Compositions open EHR Browser, Templates opens Templates), the "Connected Server" card (name, URL, type, status, version) opens Servers, and with no active profile an onboarding card ("+ Add Your First Server" / "Go to Servers") points there. Counts are server-side exactly like the Tauri dashboard (`COUNT` on concrete leaf paths, since EHRBase rejects
  `COUNT` on whole objects; integer, float or string results all parse) plus the template count, with Refresh.
* **EHR Browser:** paged EHR list from the server (+ Refresh, Load more), compositions of the selected EHR, and search + sort as in
  the Tauri app:
  * **Search** is the same small query language (`src/search.rs`, ported from `EhrBrowser.vue` + `ehr.rs`): a bare token
    is an EHR-id prefix, with `subject:`, `namespace:`, `system:`, `modifiable:true|false`, `hasCompositions:true`,
    `hasDirectory:true|false` and `created-on|before|after:YYYY-MM-DD` keys. It runs 600 ms after you stop typing or
    on Enter, Esc/Clear returns to the list, validation errors and filter chips use the Tauri wording. A full UUID uses
    equality; a *partial* id prefix has no working AQL predicate, so the CDR is paged in 200-row chunks (cap 2000) and
    filtered client-side; `hasDirectory` checks `GET /ehr/{id}/directory` (8 at a time). Results are capped at 200
    ("Showing first 200 results"). As in Tauri, `hasCompositions:false` and the date filters are rejected with the same
    messages (EHRBase can't express them).
  * **Sort** by Created date / EHR ID / System ID with an Asc/Desc toggle (`ORDER BY` with an `e/ehr_id/value ASC`
    tiebreaker). If the CDR rejects `ORDER BY` (EHRBase does) the list is re-fetched unsorted, the controls dim and
    "Sorting isn't supported by this server — showing default order." appears.
  Not ported: the Filters modal and search history (the query syntax covers the same criteria).
* **Composition screen** (opened from EHR Browser, like the Tauri app; there is no Compositions tab): click a composition to fetch it in canonical **JSON** and **FLAT**
  (FLAT media type chosen by server type, as in the Tauri backend); the two requests are independent, so JSON shows
  as soon as it arrives and a server/template without FLAT support only affects its own tab.
* **Templates:** template list and Web Template trees of the active server.
* **AQL Runner:** an editable multi-line query box (typing, paste, arrows incl. Up/Down, Home/End, Enter, Tab, `Ctrl+U`
  clear) that runs against the active server with **Ctrl+Enter** or the Run button. In demo mode the query is not executed.

Checked against the mock and the public sandbox (switching profiles from the Servers tab repoints all of the above:
the sandbox shows its own 1,422 EHRs / 11,504 compositions / 506 templates).

## Analytics (Aptabase)

The official `tauri-plugin-aptabase` is Tauri-only; this uses the community
[`aptabase-rs`](https://crates.io/crates/aptabase-rs) SDK (unofficial, needs a Tokio runtime). Same policy as the app:
**off by default**, active only with `--analytics` **and** an App Key (`APTABASE_APP_KEY`, compile-time like the Tauri
app, with a runtime env fallback for testing; self-hosted `A-SH-` keys also need `--analytics-host`). Events are
coarse (`app_started`, `connection_result`, `view_opened`, `aql_run`, `app_exited`); never query text, EHR/composition
ids, server URLs or credentials. Verified against the mock: nothing is sent without the flag, without a key or with an
invalid key.

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
| Startup → first UI (median of 5) | 111 ms browser launch + 373 ms page load ≈ **485 ms** | **~77 ms** (spawn → first `render()`) |
| Idle memory (PSS) | **348 MB** (8 processes) | **107 MB** (1 process) |
| Frontend artifact | 1.1 MB `dist/` (+ a ~290 MB browser runtime) | 36 MB self-contained binary (25 MB before `reqwest`/`tokio`/TLS were added) |

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

## Settings

The Settings view edits the same `settings.json` as the Tauri app (terminology server URL, update check, tours, analytics opt-in).
Fields this PoC does not use (updates, tours) are preserved and written back unchanged. Analytics stays opt-in and the toggle applies immediately; it is off by default.
