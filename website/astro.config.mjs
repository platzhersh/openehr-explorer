import { defineConfig } from "astro/config";
import vue from "@astrojs/vue";

// This site is published as a GitHub Pages site under the custom domain
// https://openehr-explorer.dev, so it's served from the domain root (no
// project-page base path). `build.format: "file"` keeps the historical
// `docs.html` / `compare.html` / `brand-kit.html` URLs (instead of
// Astro's default `/docs/` directory style) so existing links,
// bookmarks, and the sitemap keep working.
export default defineConfig({
  site: "https://openehr-explorer.dev",
  outDir: "./dist",
  build: {
    format: "file",
  },
  integrations: [vue()],
});
