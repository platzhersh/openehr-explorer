// Drives the REAL Vue frontend (production build via `vite preview`) in headless Chromium with the
// repo's own Tauri-IPC mock (scripts/demo-assets/mock.js). Captures screenshots + perf numbers.
// usage: node vue-bench.mjs <repoRoot> <outDir> [url]
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { execSync } from "node:child_process";
const [repo, outDir, url = "http://localhost:4173"] = process.argv.slice(2);
const require = createRequire(path.join(repo, "package.json"));
const { chromium } = require("playwright");
const MOCK = path.join(repo, "scripts/demo-assets/mock.js");
fs.mkdirSync(outDir, { recursive: true });

const median = (a) => [...a].sort((x, y) => x - y)[Math.floor(a.length / 2)];
function browserPid() {
  const lines = execSync("ps -eo pid=,args=").toString().split("\n");
  const l = lines.find((x) => /chrome-linux64\/chrome/.test(x) && !/--type=/.test(x) && !/ps -eo/.test(x));
  return Number(l.trim().split(/\s+/)[0]);
}
function descendants(root) {
  const rows = execSync("ps -eo pid=,ppid=").toString().trim().split("\n").map((l) => l.trim().split(/\s+/).map(Number));
  const kids = new Map();
  for (const [p, pp] of rows) (kids.get(pp) ?? kids.set(pp, []).get(pp)).push(p);
  const out = [root], q = [root];
  while (q.length) for (const k of kids.get(q.shift()) ?? []) { out.push(k); q.push(k); }
  return out;
}
// PSS (proportional set size) so shared pages across Chromium processes are not double counted.
function pssKb(pids) {
  let t = 0;
  for (const p of pids) {
    try { t += Number(/Pss:\s+(\d+)/.exec(fs.readFileSync(`/proc/${p}/smaps_rollup`, "utf8"))[1]); } catch {}
  }
  return t;
}
const VIEWPORT = { width: 1440, height: 960 };
// Replace execute_aql with an N-row generator (same shape as the real command's AqlResult).
const rowsOverride = () => {
  const inv = window.__TAURI_INTERNALS__.invoke;
  window.__TAURI_INTERNALS__.invoke = (cmd, args) => {
    if (cmd === "execute_aql" && window.__BENCH_ROWS__) {
      const n = window.__BENCH_ROWS__;
      const rows = new Array(n);
      for (let i = 0; i < n; i++) {
        const h = (i * 2654435761 >>> 0).toString(16).padStart(8, "0");
        rows[i] = [`${h}-1c1b-4b0a-9d1e-9a4b2f3c8a11`, 100 + (i % 60), 60 + (i % 40), `2026-07-${String(1 + (i % 28)).padStart(2, "0")}T10:00:00Z`];
      }
      return Promise.resolve({ columns: [{ name: "ehr_id", path: "/ehr_id/value" }, { name: "systolic", path: "/systolic" }, { name: "diastolic", path: "/diastolic" }, { name: "time_committed", path: "/t" }], rows, total_count: n, execution_time_ms: 1 });
    }
    return inv(cmd, args);
  };
};

async function open() {
  const t0 = performance.now();
  const browser = await chromium.launch({ executablePath: "/opt/pw-browsers/chromium", args: ["--no-sandbox"] });
  const tLaunch = performance.now() - t0;
  const page = await browser.newPage({ viewport: VIEWPORT, deviceScaleFactor: 1 });
  await page.addInitScript({ path: MOCK });
  await page.addInitScript(() => { window.__DEMO_APP_VERSION__ = "0.10.3"; });
  const t1 = performance.now();
  await page.goto(url);
  await page.waitForSelector(".sidebar-nav");
  await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
  const tNav = performance.now() - t1;
  return { browser, page, tLaunch, tNav };
}

async function addServer(page) {
  await page.click('a.nav-item:has-text("Servers")');
  await page.click('button:has-text("Add Server")');
  await page.fill("#server-name", "Local EHRBase");
  await page.fill("#server-base-url", "http://localhost:8080/ehrbase");
  await page.click('button[type="submit"]:has-text("Save")');
  await page.waitForTimeout(300);
}

const result = { startup: [], scenarios: {} };

// ---------- startup (5 cold launches) ----------
for (let i = 0; i < 5; i++) {
  const s = await open();
  const pids = descendants(browserPid());
  if (i === 4) result.idle_pss_mb = +(pssKb(pids) / 1024).toFixed(1), result.idle_procs = pids.length;
  result.startup.push({ browser_launch_ms: +s.tLaunch.toFixed(0), page_load_to_ui_ms: +s.tNav.toFixed(0) });
  await s.browser.close();
}

// ---------- screenshots + scenarios ----------
const { browser, page } = await open();
await page.evaluate(rowsOverride);
await page.evaluate(() => { window.__BENCH_ROWS__ = 0; });
await page.screenshot({ path: path.join(outDir, "vue-01-dashboard.png") });
await addServer(page);
await page.click('a.nav-item:has-text("EHR Browser")');
await page.waitForSelector(".ehr-item");
await page.click(".ehr-item >> nth=0");
await page.waitForTimeout(500);
await page.mouse.move(1439, 959);
await page.screenshot({ path: path.join(outDir, "vue-02-ehrs.png") });
await page.click('a.nav-item:has-text("Templates")');
await page.waitForSelector(".template-item");
await page.click(".template-content >> nth=0");
await page.waitForTimeout(500);
await page.screenshot({ path: path.join(outDir, "vue-03-templates.png") });
await page.click('a.nav-item:has-text("AQL Runner")');
await page.waitForSelector(".results-section, .codemirror-container");
await page.waitForTimeout(500);

async function runAql(n) {
  await page.evaluate((n) => { window.__BENCH_ROWS__ = n; }, n);
  const ms = await page.evaluate(async (n) => {
    const btn = [...document.querySelectorAll("button")].find((b) => b.textContent.includes("Run"));
    const t0 = performance.now();
    btn.click();
    await new Promise((res) => {
      const poll = () => (document.querySelectorAll(".results-table tbody tr").length >= n ? res() : requestAnimationFrame(poll));
      poll();
    });
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    return performance.now() - t0;
  }, n);
  return ms;
}
async function scrollBench() {
  return page.evaluate(async () => {
    let el = document.querySelector(".results-table");
    while (el && el.scrollHeight - el.clientHeight < 50) el = el.parentElement;
    el = el ?? document.scrollingElement;
    const max = el.scrollHeight - el.clientHeight;
    const frames = 300, iv = [];
    let last = 0, f = 0;
    await new Promise((res) => {
      const tick = (now) => {
        if (f > 0) iv.push(now - last);
        last = now;
        el.scrollTop = (max * f) / frames;
        f++;
        f > frames ? res() : requestAnimationFrame(tick);
      };
      requestAnimationFrame(tick);
    });
    iv.sort((a, b) => a - b);
    const q = (p) => iv[Math.floor((iv.length - 1) * p)];
    const total = iv.reduce((a, b) => a + b, 0);
    return { scrollable_px: max, frames: iv.length, avg_fps: +(iv.length / (total / 1000)).toFixed(1), frame_ms_p50: +q(0.5).toFixed(2), frame_ms_p95: +q(0.95).toFixed(2), frame_ms_max: +iv[iv.length - 1].toFixed(2) };
  });
}

for (const n of [3, 1000, 10000, 50000]) {
  const ms = await runAql(n);
  await page.waitForTimeout(400);
  if (n === 3) { await page.mouse.move(1439, 959); await page.screenshot({ path: path.join(outDir, "vue-04-aql.png") }); }
  const pids = descendants(browserPid());
  const sc = { rows: n, run_to_render_ms: +ms.toFixed(1), pss_mb: +(pssKb(pids) / 1024).toFixed(1) };
  if (n >= 1000) {
    sc.dom_nodes = await page.evaluate(() => document.getElementsByTagName("*").length);
    sc.scroll = await scrollBench();
  }
  result.scenarios[n] = sc;
  console.error("done", n);
}
result.startup_median = {
  browser_launch_ms: median(result.startup.map((s) => s.browser_launch_ms)),
  page_load_to_ui_ms: median(result.startup.map((s) => s.page_load_to_ui_ms)),
};
fs.writeFileSync(path.join(outDir, "vue-results.json"), JSON.stringify(result, null, 2));
console.log(JSON.stringify(result, null, 2));
await browser.close();
