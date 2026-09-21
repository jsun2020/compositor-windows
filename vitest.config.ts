import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// `.tsx` is included and the React plugin loaded so a component's own DOM behaviour can be
// tested directly (ContextMenu's window listeners). Those files opt into jsdom with a
// `@vitest-environment jsdom` docblock; everything else stays on the faster node environment.
export default defineConfig({
  plugins: [react()],
  test: { include: ["app/tests/unit/**/*.test.ts", "app/tests/unit/**/*.test.tsx"], environment: "node" },
});
