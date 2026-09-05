#!/usr/bin/env node
// Regenerates the hero demo.mp4 / demo.gif / demo.vtt in docs/assets/.
// Drives the real Vue frontend in Playwright against the fixtures in
// mock.js, then encodes the raw capture with ffmpeg/gifsicle. See the
// "generate-demo-assets" skill for prerequisites and the full explanation
// of why this mocks Tauri's IPC boundary instead of running the actual
// native app.
//
// Usage: node scripts/demo-assets/record-video.js [--url http://localhost:5173]
//
// Requires ffmpeg and gifsicle on PATH (not npm packages — install via your
// system package manager, e.g. `apt install ffmpeg gifsicle` or
// `brew install ffmpeg gifsicle`).

import path from "node:path";
import fs from "node:fs";
import os from "node:os";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { requireBinary } from "./resolve-binary.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "..", "..");
const ASSETS_DIR = path.join(REPO_ROOT, "docs", "assets");
const MOCK_PATH = path.join(__dirname, "mock.js");

const urlArgIdx = process.argv.indexOf("--url");
const APP_URL = urlArgIdx !== -1 ? process.argv[urlArgIdx + 1] : "http://localhost:5173";

const VIEWPORT = { width: 1600, height: 960 };

const FFMPEG = requireBinary("ffmpeg", "e.g. apt/brew install ffmpeg");
const GIFSICLE = requireBinary("gifsicle", "e.g. apt/brew install gifsicle");

// Wall-clock cue log kept in lockstep with the recording, written out as a
// WebVTT file afterwards. `mark()` records "this caption starts now".
const cues = [];
let sceneStart = null;
function mark(text) {
  const t = Date.now();
  if (sceneStart !== null) cues.at(-1).endMs = t - sceneStart;
  if (sceneStart === null) sceneStart = t;
  cues.push({ startMs: t - sceneStart, endMs: null, text });
}

function vttTimestamp(ms) {
  const totalMs = Math.max(0, Math.round(ms));
  const h = Math.floor(totalMs / 3600000);
  const m = Math.floor((totalMs % 3600000) / 60000);
  const s = Math.floor((totalMs % 60000) / 1000);
  const msPart = totalMs % 1000;
  const pad = (n, len) => String(n).padStart(len, "0");
  return `${pad(h, 2)}:${pad(m, 2)}:${pad(s, 2)}.${pad(msPart, 3)}`;
}

async function recordScenes(page) {
  await page.waitForSelector(".sidebar-nav", { timeout: 15000 });
  await page.waitForTimeout(600);

  // ---- Scene 1: Servers — add a profile, test the connection ----
  mark("Add a server profile and test the connection");
  await page.click('a.nav-item:has-text("Servers")');
  await page.waitForTimeout(700);
  await page.click('button:has-text("Add Server")');
  await page.waitForSelector("#server-name", { timeout: 5000 });
  await page.waitForTimeout(400);

  await page.type("#server-name", "Local EHRBase", { delay: 45 });
  await page.waitForTimeout(200);
  // The Base URL field is pre-filled with this same default value, so clear
  // it first — otherwise page.type() inserts at the cursor and the field
  // ends up showing the URL twice.
  await page.fill("#server-base-url", "");
  await page.type("#server-base-url", "http://localhost:8080/ehrbase", { delay: 30 });
  await page.waitForTimeout(300);

  const usernameField = page.locator("#server-username");
  if (await usernameField.count()) await usernameField.type("ehrbase-user", { delay: 40 });
  const passwordField = page.locator("#server-password");
  if (await passwordField.count()) await passwordField.type("••••••••••", { delay: 40 });
  await page.waitForTimeout(500);

  await page.click('button:has-text("Test Connection")');
  await page.waitForSelector(".test-result.success", { timeout: 5000 });
  await page.waitForTimeout(1600);

  await page.click('button[type="submit"]:has-text("Save")');
  await page.waitForTimeout(900);

  await page.click('.profile-actions button:has-text("Test")');
  await page.waitForSelector(".card-test-result.success", { timeout: 5000 });
  await page.waitForTimeout(1400);

  // ---- Scene 2: EHR Browser ----
  mark("Browse EHRs and drill into their compositions");
  await page.click('a.nav-item:has-text("EHR Browser")');
  await page.waitForSelector(".ehr-item", { timeout: 5000 });
  await page.waitForTimeout(900);
  await page.click('.ehr-item:has-text("9c2c5c5e")');
  await page.waitForTimeout(2800);

  // ---- Scene 3: Template Browser ----
  mark("Inspect Web Templates as a readable tree");
  await page.click('a.nav-item:has-text("Templates")');
  await page.waitForSelector(".template-item", { timeout: 5000 });
  await page.waitForTimeout(800);
  await page.click(".template-content >> nth=0");
  await page.waitForTimeout(3000);

  // ---- Scene 4: AQL Runner — type a query, trigger autocomplete ----
  mark("Write AQL queries with template-aware autocomplete");
  await page.click('a.nav-item:has-text("AQL Runner")');
  await page.waitForSelector(".context-template-select", { timeout: 5000 });
  await page.waitForTimeout(700);

  await page.selectOption(".context-template-select", "vital_signs.v1");
  await page.waitForTimeout(1000);

  const editor = page.locator(".codemirror-container .cm-content");
  await editor.click();
  await page.keyboard.press("ControlOrMeta+A");
  await page.keyboard.press("Backspace");
  await page.waitForTimeout(200);

  await page.keyboard.type("SELECT c/uid/value", { delay: 55 });
  await page.keyboard.press("Enter");
  await page.keyboard.type("FROM EHR e", { delay: 55 });
  await page.keyboard.press("Enter");
  await page.keyboard.type("CONTAINS COMPOSITION c", { delay: 55 });
  await page.keyboard.press("Enter");
  await page.keyboard.type(
    "CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.vital_signs.v2]",
    { delay: 40 },
  );
  await page.keyboard.press("Enter");
  await page.keyboard.type("WHERE o", { delay: 55 });
  await page.waitForTimeout(300);
  await page.keyboard.type("/", { delay: 40 });
  // Let the Layer-3 autocomplete popup appear and hold on it.
  await page.waitForTimeout(1400);
  await page.keyboard.type("sy", { delay: 90 });
  // Hold on the narrowed single-suggestion popup.
  await page.waitForTimeout(2000);
  // Dismiss rather than accept — the suggestion's aqlPath is absolute
  // from the composition root, so inserting it after "o/" would double
  // up the archetype prefix already implied by the CONTAINS clause.
  // Type the equivalent relative path by hand for a clean result.
  await page.keyboard.press("Escape");
  await page.keyboard.press("Backspace");
  await page.keyboard.press("Backspace");
  await page.keyboard.type(
    "data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude > 140",
    { delay: 30 },
  );
  await page.waitForTimeout(1200);

  await page.click('button:has-text("Run (Ctrl+Enter)")');
  await page.waitForTimeout(2600);

  cues.at(-1).endMs = Date.now() - sceneStart;
}

function encode(rawWebm, tmpDir, leadInSeconds) {
  const mp4Path = path.join(ASSETS_DIR, "demo.mp4");
  const gifPath = path.join(ASSETS_DIR, "demo.gif");
  const rawGif = path.join(tmpDir, "raw.gif");

  // Playwright's recordVideo starts capturing at context creation — before
  // page.goto() navigates — so the raw footage always opens on a blank
  // about:blank frame or two. -ss (input seeking, before -i) trims exactly
  // that lead-in, timed from the same page.goto()-to-loaded measurement
  // the VTT cues are already zeroed against, so captions stay in sync.
  const seekArgs = ["-ss", leadInSeconds.toFixed(3)];

  console.log("encoding mp4...");
  execFileSync(FFMPEG, [
    "-y", ...seekArgs, "-i", rawWebm,
    "-vf", "scale=1400:-2:flags=lanczos",
    "-c:v", "libx264", "-preset", "slow", "-crf", "20",
    "-pix_fmt", "yuv420p", "-movflags", "+faststart", "-an",
    mp4Path,
  ], { stdio: "inherit" });

  console.log("encoding gif (palette pass)...");
  execFileSync(FFMPEG, [
    "-y", ...seekArgs, "-i", rawWebm,
    "-vf", "fps=12,scale=1000:-2:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff:max_colors=200[p];[b][p]paletteuse=dither=none",
    rawGif,
  ], { stdio: "inherit" });

  console.log("optimizing gif with gifsicle...");
  execFileSync(GIFSICLE, ["-O3", "--lossy=40", "-o", gifPath, rawGif], { stdio: "inherit" });

  console.log("mp4:", mp4Path, `(${(fs.statSync(mp4Path).size / 1024).toFixed(0)} KB)`);
  console.log("gif:", gifPath, `(${(fs.statSync(gifPath).size / 1024).toFixed(0)} KB)`);
}

function writeVtt() {
  const vttPath = path.join(ASSETS_DIR, "demo.vtt");
  const lines = ["WEBVTT", ""];
  for (const cue of cues) {
    lines.push(`${vttTimestamp(cue.startMs)} --> ${vttTimestamp(cue.endMs)}`, cue.text, "");
  }
  fs.writeFileSync(vttPath, lines.join("\n"));
  console.log("vtt:", vttPath);
}

async function main() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "openehr-explorer-demo-"));
  const videoDir = path.join(tmpDir, "video");
  fs.mkdirSync(videoDir);

  const browser = await chromium.launch();
  // recordVideo starts capturing right here, at context creation — well
  // before page.goto() below actually navigates — so this timestamp is
  // the reference point for measuring (and later trimming) that lead-in.
  const recordingStartedAt = Date.now();
  const context = await browser.newContext({
    viewport: VIEWPORT,
    deviceScaleFactor: 2, // supersampled capture for sharper downscaled text
    recordVideo: { dir: videoDir, size: VIEWPORT },
  });
  await context.addInitScript({ path: MOCK_PATH });
  const page = await context.newPage();

  await page.goto(APP_URL);
  await recordScenes(page);

  await context.close();
  await browser.close();

  const webmFiles = fs.readdirSync(videoDir).filter((f) => f.endsWith(".webm"));
  if (!webmFiles.length) throw new Error("no video was produced");
  const rawWebm = path.join(videoDir, webmFiles[0]);

  // sceneStart (set by the first mark() call, inside recordScenes) is the
  // same "recording is actually showing the app now" instant the VTT cues
  // are zeroed against — trimming the raw footage to start there too keeps
  // the encoded video and the captions in sync.
  const leadInSeconds = Math.max(0, (sceneStart - recordingStartedAt) / 1000);
  encode(rawWebm, tmpDir, leadInSeconds);
  writeVtt();

  fs.rmSync(tmpDir, { recursive: true, force: true });
  console.log("\nDone. Review docs/assets/demo.mp4 and demo.gif before committing.");
}

try {
  await main();
} catch (err) {
  console.error(err);
  process.exit(1);
}
