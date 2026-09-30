import { defineConfig } from "vitest/config";

// Separate from vite.config.mjs on purpose: that config carries Tauri dev-server
// settings (fixed port, HMR host, etc.) that have no bearing on unit tests and
// would just add noise/fragility here.
export default defineConfig({
  test: {
    environment: "node",
    // contrast.test.ts reads src/styles/tokens.css as text; by default Vitest
    // stubs CSS imports to an empty string.
    css: { include: [/tokens\.css/] },
    include: ["src/**/*.test.ts"],
  },
});
