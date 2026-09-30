// Runs axe-core (color-contrast rule) against every story in the built
// Storybook (`npm run build-storybook`). Exits 1 on any violation. See ADR-0028.
//
// Usage: npm run test:a11y            (expects ./storybook-static)
//        A11Y_RULES=color-contrast,label npm run test:a11y   (override rules)
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, isAbsolute, join, relative, resolve } from "node:path";
import { chromium } from "playwright";
import AxeBuilder from "@axe-core/playwright";

const ROOT = join(process.cwd(), "storybook-static");
const RULES = (process.env.A11Y_RULES ?? "color-contrast").split(",");
const TYPES = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".mjs": "text/javascript",
  ".css": "text/css",
  ".json": "application/json",
  ".svg": "image/svg+xml",
  ".woff2": "font/woff2",
  ".png": "image/png",
};

// Loopback-only static server for the built Storybook. Responses are only
// started after the file has been read, so a missing file is a clean 404
// (writing headers before readFile made the error path crash the process).
const server = createServer(async (req, res) => {
  try {
    const { pathname } = new URL(req.url ?? "/", "http://localhost");
    const requested = resolve(ROOT, `.${decodeURIComponent(pathname)}`);
    const rel = relative(ROOT, requested);
    if (rel.startsWith("..") || isAbsolute(rel)) throw new Error("outside root");
    const file = pathname.endsWith("/") ? join(requested, "index.html") : requested;
    const body = await readFile(file);
    res.writeHead(200, { "content-type": TYPES[extname(file)] ?? "application/octet-stream" });
    res.end(body);
  } catch {
    if (!res.headersSent) res.writeHead(404);
    res.end();
  }
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const base = `http://127.0.0.1:${server.address().port}`;

const index = JSON.parse(await readFile(join(ROOT, "index.json"), "utf8"));
const stories = Object.values(index.entries).filter((e) => e.type === "story");

const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined });
const page = await (await browser.newContext()).newPage();

// Storybook only signals "rendered and play function done" over its channel.
// Hook it before any page script runs (so an already-finished story can't be
// missed) and record the outcome on window for waitForFunction below.
await page.addInitScript(() => {
  window.__sbOutcome = null;
  const timer = setInterval(() => {
    const channel = window.__STORYBOOK_ADDONS_CHANNEL__;
    if (!channel) return;
    clearInterval(timer);
    channel.on("storyFinished", (e) => {
      window.__sbOutcome ??= e?.status === "error" ? "error" : "ok";
    });
    for (const name of ["playFunctionThrewException", "storyThrewException", "storyErrored"]) {
      channel.on(name, () => {
        window.__sbOutcome = "error";
      });
    }
  }, 0);
});

const playWarnings = []; // story ids whose play function threw
const unsettled = []; // story ids that never finished rendering
const failures = new Map(); // "rule: selector fg on bg (ratio)" -> Set(story ids)

// Waits until the story is rendered and its play function has settled, then
// scans whatever state it reached. A throwing play function (e.g. the
// Clipboard API in a headless browser) is only a warning: it's unrelated to
// contrast and the DOM it reached is still scanned.
//
// A story that never settles could pass vacuously (half-rendered DOM), so it
// fails the run in CI (or with A11Y_STRICT=1). Offline, where medblocks-ui's
// CDN script (ADR-0008) is unreachable, it stays a warning so the scan is
// still usable locally.
const STRICT = Boolean(process.env.CI) || process.env.A11Y_STRICT === "1";
async function waitForStory(id) {
  try {
    await page.waitForFunction(() => window.__sbOutcome !== null, null, { timeout: 15_000 });
  } catch {
    unsettled.push(id);
    return;
  }
  if ((await page.evaluate(() => window.__sbOutcome)) === "error") playWarnings.push(id);
}

// Storybook's addon-a11y may still be mid-run in the iframe; retry if axe is busy.
async function scan() {
  for (let attempt = 0; ; attempt++) {
    try {
      return (await new AxeBuilder({ page }).include("#storybook-root").withRules(RULES).analyze())
        .violations;
    } catch (e) {
      if (attempt >= 5 || !/already running/.test(String(e))) throw e;
      await page.waitForTimeout(300);
    }
  }
}

for (const s of stories) {
  await page.goto(`${base}/iframe.html?id=${s.id}&viewMode=story&globals=a11y.manual:true`, {
    waitUntil: "load",
  });
  await waitForStory(s.id);
  const violations = await scan();
  for (const v of violations) {
    for (const n of v.nodes) {
      const d = n.any[0]?.data ?? {};
      const key = `${v.id}: ${n.target.join(" ")} ${d.fgColor ?? ""} on ${d.bgColor ?? ""} (${d.contrastRatio ?? "?"}:1)`;
      (failures.get(key) ?? failures.set(key, new Set()).get(key)).add(s.id);
    }
  }
}
await browser.close();
server.close();

console.log(`Scanned ${stories.length} stories with rules: ${RULES.join(", ")}`);
if (unsettled.length) {
  console.log(
    `${STRICT ? "error" : "warning"}: ${unsettled.length} stories did not finish rendering in 15s${STRICT ? "" : "; scanned as-is"}: ${unsettled.join(", ")}`,
  );
  if (STRICT) process.exitCode = 1;
}
if (playWarnings.length) {
  console.warn(
    `warning: ${playWarnings.length} play function(s) failed; the state reached was scanned anyway: ${playWarnings.join(", ")}`,
  );
}
if (failures.size) {
  for (const [k, ids] of failures)
    console.log(
      `\n✗ ${k}\n    e.g. ${[...ids].slice(0, 3).join(", ")}${ids.size > 3 ? ` (+${ids.size - 3} more)` : ""}`,
    );
  console.log(`\n${failures.size} distinct violation(s).`);
  process.exit(1);
}
console.log("No violations.");
