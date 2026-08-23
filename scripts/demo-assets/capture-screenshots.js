#!/usr/bin/env node
// Regenerates the static marketing screenshots in docs/assets/screenshots/.
// Drives the real Vue frontend in Playwright against the fixtures in
// mock.js — see the "generate-demo-assets" skill for prerequisites and the
// full explanation of why this mocks Tauri's IPC boundary instead of
// running the actual native app.
//
// Usage: node scripts/demo-assets/capture-screenshots.js [--url http://localhost:5173]

import path from "node:path";
import fs from "node:fs";
import os from "node:os";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "..", "..");
const OUT_DIR = path.join(REPO_ROOT, "docs", "assets", "screenshots");
const MOCK_PATH = path.join(__dirname, "mock.js");

const urlArgIdx = process.argv.indexOf("--url");
const APP_URL = urlArgIdx !== -1 ? process.argv[urlArgIdx + 1] : "http://localhost:5173";

const VIEWPORT = { width: 1440, height: 960 };

// Sanitize PATH before shelling out to ffmpeg below — strip empty/relative
// entries so a stray relative (or otherwise untrusted) directory earlier
// in PATH can't shadow the real binary.
process.env.PATH = (process.env.PATH || "")
  .split(path.delimiter)
  .filter((dir) => path.isAbsolute(dir))
  .join(path.delimiter);

try {
  execFileSync("ffmpeg", ["-version"], { stdio: "ignore" });
} catch {
  console.error("error: 'ffmpeg' not found on PATH — install it (e.g. apt/brew install ffmpeg) and retry.");
  console.error("(Playwright can only capture PNG directly; ffmpeg converts to the .webp files this script writes.)");
  process.exit(1);
}

const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "openehr-explorer-screenshots-"));

// Playwright's screenshot() only writes PNG/JPEG — capture PNG to a scratch
// file, then let ffmpeg do the PNG -> WEBP conversion the site's assets use.
// The view containers (.ehr-browser, .composition-viewer, etc.) are flex
// children that stretch to fill the app shell's remaining height, so a
// plain element screenshot captures a lot of empty space below the real
// content. Crop to the actual bottom-most rendered content instead — the
// max bottom edge across the container's direct child columns (sidebar,
// list, detail panel) — for a tight, content-hugging screenshot like the
// hand-curated ones this replaces.
async function saveElementScreenshot(page, selector, filename) {
  const el = page.locator(selector).first();
  await el.waitFor({ state: "visible", timeout: 10000 });
  const clip = await page.evaluate((sel) => {
    const container = document.querySelector(sel);
    const rect = container.getBoundingClientRect();
    // Flex panels mirror their parent's stretched height, so trusting any
    // wrapper element's own bottom edge overstates the real content extent.
    // Only trust genuine content nodes — things with their own text or
    // that are inherently content-sized (an image, an input, a button) —
    // and take the lowest one of those as where the real content ends.
    const LEAFY_TAGS = new Set(["IMG", "SVG", "INPUT", "BUTTON", "SELECT", "TEXTAREA", "LABEL", "A"]);
    let maxBottom = rect.top;
    for (const el of container.querySelectorAll("*")) {
      const r = el.getBoundingClientRect();
      if (r.width === 0 || r.height === 0 || r.top >= window.innerHeight) continue;
      const hasOwnText = Array.from(el.childNodes).some(
        (n) => n.nodeType === Node.TEXT_NODE && n.textContent.trim().length > 0,
      );
      if ((hasOwnText || LEAFY_TAGS.has(el.tagName)) && r.bottom > maxBottom) {
        maxBottom = r.bottom;
      }
    }
    const height = Math.min(maxBottom - rect.top + 24, window.innerHeight - rect.top);
    return { x: rect.x, y: rect.y, width: rect.width, height };
  }, selector);

  const pngPath = path.join(tmpDir, filename.replace(/\.webp$/, ".png"));
  await page.screenshot({ path: pngPath, clip });
  const outPath = path.join(OUT_DIR, filename);
  execFileSync("ffmpeg", ["-y", "-i", pngPath, "-lossless", "0", "-quality", "90", outPath], { stdio: "ignore" });
  console.log("saved:", filename);
}

async function main() {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: VIEWPORT, deviceScaleFactor: 2 });
  await page.addInitScript({ path: MOCK_PATH });

  page.on("pageerror", (err) => console.error("[pageerror]", err.message));

  await page.goto(APP_URL);
  await page.waitForSelector(".sidebar-nav", { timeout: 15000 });

  const fixtures = await page.evaluate(() => window.__DEMO_FIXTURES__);

  // ---- 05-servers.webp: add + test a server profile ----
  await page.click('a.nav-item:has-text("Servers")');
  await page.click('button:has-text("+ Add Server")');
  await page.waitForSelector("#server-name", { timeout: 5000 });
  await page.fill("#server-name", "Local EHRBase");
  await page.fill("#server-base-url", "http://localhost:8080/ehrbase");
  await page.locator("#server-username").fill("ehrbase-user");
  await page.locator("#server-password").fill("ehrbase-password");
  await page.click('button:has-text("Test Connection")');
  await page.waitForSelector(".test-result.success", { timeout: 5000 });
  await page.click('button[type="submit"]:has-text("Save")');
  await page.waitForTimeout(300);
  await page.click('.profile-actions button:has-text("Test")');
  await page.waitForSelector(".card-test-result.success", { timeout: 5000 });
  await page.waitForTimeout(300);
  await saveElementScreenshot(page, ".server-manager", "05-servers.webp");

  // ---- 01-ehr-browser.webp: EHR list + detail with grouped compositions ----
  await page.click('a.nav-item:has-text("EHR Browser")');
  await page.waitForSelector(".ehr-item", { timeout: 5000 });
  await page.click(`.ehr-item:has-text("${fixtures.primaryEhrId.slice(0, 8)}")`);
  await page.waitForTimeout(500);
  await saveElementScreenshot(page, ".ehr-browser", "01-ehr-browser.webp");

  // ---- 02/03/03b: Composition Viewer — Pretty / FLAT / JSON ----
  // Composition rows are plain clickable divs (@click="openComposition"),
  // not <a> links — a client-side router.push. A full page.goto() here
  // would reload the SPA and lose the in-memory server/mock state set up
  // above, so click through the UI instead.
  const COMPOSITION_VIEWER = ".composition-viewer";
  await page.click('.composition-item:has-text("Vital Signs")', { timeout: 5000 });
  await page.waitForSelector(COMPOSITION_VIEWER, { timeout: 5000 });
  await page.waitForSelector(".tab-bar .tab:has-text(\"FLAT\"):not([disabled])", { timeout: 5000 });
  await page.waitForTimeout(500);
  await saveElementScreenshot(page, COMPOSITION_VIEWER, "02-composition-pretty.webp");

  await page.click('.tab-bar .tab:has-text("FLAT")');
  await page.waitForTimeout(300);
  await saveElementScreenshot(page, COMPOSITION_VIEWER, "03-composition-flat.webp");

  await page.click('.tab-bar .tab:has-text("JSON")');
  await page.waitForTimeout(300);
  await saveElementScreenshot(page, COMPOSITION_VIEWER, "03b-composition-json.webp");

  // ---- 04-templates.webp: Template Browser, OPT Tree for Vital Signs ----
  await page.click('a.nav-item:has-text("Templates")');
  await page.waitForSelector(".template-item", { timeout: 5000 });
  await page.click(".template-content >> nth=0");
  await page.waitForTimeout(500);
  await saveElementScreenshot(page, ".template-browser", "04-templates.webp");

  // ---- 06-aql-runner.webp: a run query with tabular results ----
  await page.click('a.nav-item:has-text("AQL Runner")');
  await page.waitForSelector(".context-template-select", { timeout: 5000 });
  await page.selectOption(".context-template-select", fixtures.vitalSignsTemplateId);
  const editor = page.locator(".codemirror-container .cm-content");
  await editor.click();
  await page.keyboard.press("ControlOrMeta+A");
  await page.keyboard.press("Backspace");
  await page.keyboard.type(
    `SELECT c/uid/value, o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude AS systolic\n` +
      `FROM EHR e\n` +
      `CONTAINS COMPOSITION c\n` +
      `CONTAINS OBSERVATION o[${fixtures.vitalSignsArchetypeId}]`,
  );
  await page.click('button:has-text("Run (Ctrl+Enter)")');
  await page.waitForTimeout(600);
  await saveElementScreenshot(page, ".aql-runner", "06-aql-runner.webp");

  // ---- 07-aql-autocomplete.webp: Layer-3 template-path autocomplete ----
  await editor.click();
  await page.keyboard.press("ControlOrMeta+A");
  await page.keyboard.press("Backspace");
  await page.keyboard.type(
    `SELECT c/uid/value\n` +
      `FROM EHR e\n` +
      `CONTAINS COMPOSITION c\n` +
      `CONTAINS OBSERVATION o[${fixtures.vitalSignsArchetypeId}]\n` +
      `WHERE o/sy`,
  );
  await page.waitForTimeout(500);
  await saveElementScreenshot(page, ".aql-runner", "07-aql-autocomplete.webp");

  await browser.close();
  fs.rmSync(tmpDir, { recursive: true, force: true });
}

try {
  await main();
} catch (err) {
  console.error(err);
  process.exit(1);
}
