# Demo asset generator

Regenerates every marketing screenshot and the hero demo video/gif on the
landing page (`website/public/assets/screenshots/*.webp`,
`website/public/assets/demo.mp4`, `website/public/assets/demo.gif`,
`website/public/assets/demo.vtt`) and the README gif.

No real EHRBase server, no native Tauri shell, no Docker required — these
scripts drive the actual Vue frontend (`npm run dev`) in headless Chromium
via Playwright, with `mock.js` stubbing the Tauri IPC boundary
(`window.__TAURI_INTERNALS__.invoke`) so `@tauri-apps/api/core`'s `invoke()`
calls resolve to realistic canned data instead of hitting a Rust backend.
See the `generate-demo-assets` skill (`.claude/commands/`) for the full
rationale and a step-by-step walkthrough.

## Quick start

```bash
npm run dev &                          # terminal 1 — vite dev server
npm run demo:screenshots               # website/public/assets/screenshots/*.webp
npm run demo:video                     # website/public/assets/demo.{mp4,gif,vtt}
```

Both scripts accept `--url <vite-dev-server-url>` if you're not using the
default `http://localhost:5173`.

## Files

- `mock.js` — the shared fixture data (server profiles, EHRs, a "Vital
  Signs" composition + template, saved queries, AQL results) and the
  `window.__TAURI_INTERNALS__` stub. **Single source of truth** — every
  script imports this so the EHR IDs, clinician names, and template
  structure stay consistent across every generated asset. Edit fixture data
  here, not in the individual capture scripts.
- `capture-screenshots.js` — walks through Servers → EHR Browser →
  Composition Viewer (Pretty/FLAT/JSON) → Templates → AQL Runner → Request
  Inspector, saving a content-cropped `.webp` at each stop (the Request
  Inspector shots are full-window, since the drawer docks under the view).
  `mock.js` emits a `cdr-inspector-entry` event for each mocked command
  that would hit the CDR (see `describeRequest()`), which is what fills
  the inspector's request log.
- `record-video.js` — a slower, narrated walkthrough (Servers → EHR Browser
  → Templates → AQL Runner with autocomplete) captured as video and encoded
  to `demo.mp4` (h264) + `demo.gif` (palette-optimized, gifsicle-compressed)
  + `demo.vtt` (captions, timed from the actual recorded scene durations —
  not hand-typed).

## Requirements

- `ffmpeg` and `gifsicle` on `PATH` (system packages, not npm — e.g.
  `apt install ffmpeg gifsicle` / `brew install ffmpeg gifsicle`).
  `capture-screenshots.js` only needs `ffmpeg` (for PNG → WEBP conversion,
  since Playwright can't capture WEBP directly); `record-video.js` needs
  both.
- `playwright` (a pinned devDependency — `npm install` gets it. Its browser
  binary needs to be present too; if `npx playwright install chromium`
  hasn't been run in your environment, do that once first).

## Known limitations

- `capture-screenshots.js` crops each screenshot to the lowest real content
  element it can find in the view (to avoid a screenshot full of empty
  space below a flex-stretched panel — see the comment above
  `saveElementScreenshot`). It's a heuristic, not pixel-perfect; skim the
  output before committing.
- Re-running `record-video.js` will not byte-for-byte reproduce a previous
  recording — real interaction timing has natural jitter of a second or so
  per scene. That's expected; review the result rather than diffing it.
