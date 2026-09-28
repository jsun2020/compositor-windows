import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  root: "app",
  plugins: [react()],
  clearScreen: false,
  // Bind explicitly to the IPv4 loopback: on some Windows hosts IPv6 loopback is
  // broken or "localhost" resolves to the other stack than the dev server bound
  // (see Claude-LL.md LL-024), which makes a browser or Playwright's health check
  // fail to connect even though Vite reports itself ready.
  server: { host: "127.0.0.1", port: 1420, strictPort: true, fs: { allow: [".."] } },
  build: { outDir: "dist", emptyOutDir: true, target: "es2022" },
  // The job worker (app/src/engine/job-worker.ts) is a module worker that imports the engine's JS.
  worker: { format: "es" },
  define: { __APP_VERSION__: JSON.stringify(process.env.npm_package_version ?? "0.0.0") },
});
