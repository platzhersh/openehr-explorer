import { defineConfig } from "vitest/config";

// Separate from vite.config.mjs on purpose: that config carries Tauri dev-server
// settings (fixed port, HMR host, etc.) that have no bearing on unit tests and
// would just add noise/fragility here.
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
