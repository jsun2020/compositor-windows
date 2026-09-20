import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "app/tests/e2e",
  timeout: 30_000,
  // 127.0.0.1, not localhost: this host's "localhost" can resolve to the IPv6
  // loopback while Vite binds IPv4 (or vice versa) - see Claude-LL.md LL-024.
  use: { baseURL: "http://127.0.0.1:1420", headless: true },
  webServer: { command: "pnpm dev", url: "http://127.0.0.1:1420", reuseExistingServer: true, env: { VITE_BRIDGE: "mock", VITE_TEST_API: "1" } },
});
