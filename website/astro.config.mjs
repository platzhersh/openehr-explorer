import { defineConfig } from "astro/config";
import vue from "@astrojs/vue";

// This site is published as a GitHub Pages *project* site
// (https://platzhersh.github.io/openehr-explorer/), so every generated
// page lives under the `/openehr-explorer` base path. `build.format:
// "file"` keeps the historical `docs.html` / `compare.html` /
// `brand-kit.html` URLs (instead of Astro's default `/docs/` directory
// style) so existing links, bookmarks, and the sitemap keep working.
export default defineConfig({
  site: "https://platzhersh.github.io",
  base: "/openehr-explorer",
  outDir: "./dist",
  build: {
    format: "file",
  },
  integrations: [vue()],
});
