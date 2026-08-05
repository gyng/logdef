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
//
// **The port is overridable, and that is not a convenience.** Playwright
// refuses to start when it thinks 3000 is taken, and it thinks so for a
// half-dead listener that answers a TCP connect and never a request —
// a state a killed test wrapper can leave behind and which `dev-stop`
// does not clear, because nothing is LISTENING for it to find. When
// that happens the whole browser suite is unrunnable and the error
// names the port rather than the cause. `UNDERSTORY_PORT=3100` gets you
// working again in one command.
const PORT = Number.parseInt(process.env.UNDERSTORY_PORT ?? "3000", 10);

export default defineConfig({
  testDir: "./e2e",
  timeout: 180_000,
  retries: 1,
  use: {
    baseURL: `http://localhost:${String(PORT)}`,
    headless: true,
  },
  webServer: {
    command: `npm run dev -- --port ${String(PORT)}`,
    port: PORT,
    reuseExistingServer: false,
    timeout: 30_000,
  },
});
