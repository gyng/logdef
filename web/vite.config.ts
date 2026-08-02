import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";
import { execSync } from "child_process";

// Build identity injected as define()s so any module can read them.
// `BUILD_TIME` updates on every Vite restart / HMR reload of the
// config; `BUILD_COMMIT` is the current short git SHA. Both surface
// in main.tsx as a single console line so you can eyeball "is this
// the new bundle?" without diffing files.
const buildTime = new Date().toISOString();
let buildCommit = "unknown";
try {
  buildCommit = execSync("git rev-parse --short HEAD", { cwd: import.meta.dirname })
    .toString()
    .trim();
} catch {
  // git not available — leave as "unknown"
}
let buildDirty = false;
try {
  const status = execSync("git status --porcelain", { cwd: import.meta.dirname }).toString();
  buildDirty = status.trim().length > 0;
} catch {
  // ignore
}

export default defineConfig({
  plugins: [react()],
  define: {
    __BUILD_TIME__: JSON.stringify(buildTime),
    __BUILD_COMMIT__: JSON.stringify(buildCommit),
    __BUILD_DIRTY__: JSON.stringify(buildDirty),
  },
  resolve: {
    alias: {
      "@": path.resolve(import.meta.dirname, "./src"),
    },
  },
  server: {
    port: 3000,
  },
});
