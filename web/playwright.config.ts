import { defineConfig } from "@playwright/test";

// `reuseExistingServer` is intentionally `false`: playwright owns the
// dev server lifecycle for every test run, and shuts it down cleanly
// on exit. The previous `true` setting latched onto whatever vite was
// on :3000 and leaked the child process whenever a test wrapper was
// killed (ctrl-c, timeout, background invocation abandoned, …). If
// you need the dev server for manual visual checks, run `make dev`
// first AND stop playwright from colliding with your port by running
// tests in a different terminal against a different port, or just
// accept that `make e2e` will refuse to start while you're running it.
export default defineConfig({
  testDir: "./e2e",
  timeout: 180_000,
  retries: 1,
  use: {
    baseURL: "http://localhost:3000",
    headless: true,
  },
  webServer: {
    command: "npm run dev",
    port: 3000,
    reuseExistingServer: false,
    timeout: 30_000,
  },
});
