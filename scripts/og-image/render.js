#!/usr/bin/env node
// Regenerates website/public/assets/og-image.png (1200×630) from og-image.html.
//
// Usage: node scripts/og-image/render.js

import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const OUT = path.resolve(__dirname, "..", "..", "website", "public", "assets", "og-image.png");

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1200, height: 630 } });
await page.goto("file://" + path.join(__dirname, "og-image.html"));
await page.screenshot({ path: OUT, clip: { x: 0, y: 0, width: 1200, height: 630 } });
await browser.close();
console.log(`Wrote ${OUT}`);
