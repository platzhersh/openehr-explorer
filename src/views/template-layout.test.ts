import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { chromium, type Browser } from "playwright";
import sharedCss from "../styles/shared-utilities.css?inline";
import copyButtonSfc from "../components/CopyButton.vue?raw";
import templateBrowserSfc from "./TemplateBrowser.vue?raw";

// Regression guard for OEH-96: after OEH-95 removed the 300px cap on .aql-path, the
// copy button (and its hidden tooltip) moved to the row's right edge and made the
// Templates detail panel scroll sideways; the search/download buttons' centered
// [data-tooltip] chips at the panel's right edge overflow the same way. This renders the *real* stylesheets of
// TemplateBrowser.vue / CopyButton.vue in Chromium and asserts the panel has no
// horizontal overflow however long the paths are.
// Needs Chromium (CI installs it before `npm run test`); skipped locally if absent.

const styleOf = (sfc: string): string => sfc.match(/<style scoped>([\s\S]*?)<\/style>/)![1]!;
// Scoped/:deep selectors can't match without Vue's runtime, so flatten them.
const css = [
  sharedCss,
  styleOf(copyButtonSfc),
  styleOf(templateBrowserSfc).replace(/:deep\(([^)]+)\)/g, "$1"),
].join("\n");
// vue-tsc has no Node typings in src/, so reach process.env through globalThis.
const env =
  (globalThis as unknown as { process?: { env: Record<string, string | undefined> } }).process
    ?.env ?? {};

const longPath = `/context/other_context[at0001]/items[openEHR-EHR-CLUSTER.${"case_identification_".repeat(60)}.v1]`;
const row = (depth: number): string => `
  <div class="wt-node"><div class="wt-node-header" style="padding-left:${depth * 20}px">
    <span class="toggle">▼</span><span class="wt-name">Case identification</span>
    <span class="badge rm-type">CLUSTER</span><span class="aql-path">${longPath}</span>
    <span class="copy-icon-wrap"><button class="copy-icon-btn">c</button>
    <span class="copy-tooltip">Copy AQL path</span></span>
  </div></div>`;
const page = `<style>*{box-sizing:border-box}html,body{margin:0;height:100%}${css}</style>
  <div style="height:600px"><div class="template-browser"><div class="panel-left"></div>
  <div class="panel-right">
  <div class="panel-header"><span>template.v1</span><button data-tooltip="Download OPT">d</button></div>
  <div class="tree-view"><div class="wt-tree">
  <div class="panel-actions"><button data-tooltip="Search tree (Ctrl+F)">s</button></div>
  ${row(0)}${row(1)}${row(3)}</div></div></div></div></div>`;

let browser: Browser | undefined;
beforeAll(async () => {
  try {
    browser = await chromium.launch({ executablePath: env.CHROMIUM_PATH || undefined });
  } catch (err) {
    if (env.CI) throw err;
  }
});
afterAll(async () => browser?.close());

describe("Templates detail panel layout (OEH-96)", () => {
  for (const width of [900, 1400, 2560]) {
    it(`has no horizontal overflow at ${width}px`, async (ctx) => {
      if (!browser) return ctx.skip();
      const p = await browser.newPage({ viewport: { width, height: 600 } });
      await p.setContent(page);
      const { client, scroll, pathFits } = await p.evaluate(() => {
        const panel = document.querySelector(".panel-right")!;
        const path = document.querySelector(".aql-path")!;
        return {
          client: panel.clientWidth,
          scroll: panel.scrollWidth,
          pathFits: path.scrollWidth > path.clientWidth, // ellipsized, not stretching the row
        };
      });
      await p.close();
      expect(scroll).toBe(client);
      expect(pathFits).toBe(true);
    });
  }
});
