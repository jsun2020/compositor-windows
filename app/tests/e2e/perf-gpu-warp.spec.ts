import { test, expect } from "@playwright/test";

// Independent raw-kernel evidence. This does not replace the original 29 PERF
// cases or prove styled canvas preview/worker installation/native tool latency.
test.skip(!process.env.GPU_WARP_PERF, "set GPU_WARP_PERF=1 on hardware WebGL2");
test.use({ channel: "msedge", viewport: { width: 1440, height: 900 } });
for (const [name, width, height] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as const) {
  test(`tiled warp kernel on ${name}: bounded allocations, one source upload, hardware response`, async ({ page }) => {
    test.setTimeout(180_000);
    await page.goto("/"); await expect(page.getByTestId("engine-ready")).toBeVisible();
    const result = await page.evaluate(async ({ width, height }) => {
      const path = "/src/canvas/gpu-warp.ts", { GpuWarpStroke }: typeof import("../../src/canvas/gpu-warp") = await import(/* @vite-ignore */ path);
      const source = new Uint8Array(width * height * 4); new Uint32Array(source.buffer).fill(0xff000000);
      const x = Math.floor(width / 2), y = Math.floor(height / 2);
      for (let row = 0; row < height; row++) source.fill(255, (row * width + x) * 4, (row * width + x + 1) * 4);
      const measures = [];
      for (const mode of ["Liquify", "Smudge"] as const) {
        const gl = document.createElement("canvas").getContext("webgl2", { antialias: false })!;
        const info = gl.getExtension("WEBGL_debug_renderer_info"), gpu = info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : "unknown";
        // Poll a GPU fence without blocking the UI. Measure both submission and
        // actual completion, not just the time spent queueing draw commands.
        const complete = async () => {
          const sync = gl.fenceSync(gl.SYNC_GPU_COMMANDS_COMPLETE, 0)!; gl.flush(); const start = performance.now();
          try {
            for (;;) {
              const status = gl.clientWaitSync(sync, 0, 0);
              if (status === gl.ALREADY_SIGNALED || status === gl.CONDITION_SATISFIED) return;
              if (status === gl.WAIT_FAILED || performance.now() - start > 10_000) throw new Error("Warp GPU completion failed or exceeded 10 seconds");
              await new Promise<void>(resolve => setTimeout(resolve, 0));
            }
          } finally { gl.deleteSync(sync); }
        };
        const preparationStart = performance.now();
        const stroke = new GpuWarpStroke(gl, width, height, source, { mode, diameter: 40, hardness: 0.5, strength: 0.5 });
        try {
          await complete(); const preparationMs = performance.now() - preparationStart;
          const before = stroke.diagnostics(); stroke.append([x, y]);
          let maxSubmitMs = 0, maxCompleteMs = 0, maxFrameGapMs = 0, lastFrame = performance.now(), running = true;
          const tick = () => { const now = performance.now(); maxFrameGapMs = Math.max(maxFrameGapMs, now - lastFrame); lastFrame = now; if (running) requestAnimationFrame(tick); };
          await new Promise<void>(resolve => requestAnimationFrame(() => { lastFrame = performance.now(); requestAnimationFrame(tick); resolve(); }));
          for (const point of [[x + 10, y], [x + 20, y + 4], [x + 30, y], [x + 40, y + 4]] as [number, number][]) {
            const start = performance.now(); stroke.append(point); maxSubmitMs = Math.max(maxSubmitMs, performance.now() - start);
            await complete(); maxCompleteMs = Math.max(maxCompleteMs, performance.now() - start);
          }
          const readStart = performance.now(), tiles = stroke.readTiles(), readbackMs = performance.now() - readStart;
          await new Promise<void>(resolve => requestAnimationFrame(() => resolve())); running = false;
          const after = stroke.diagnostics(), readbackBytes = tiles.reduce((n, t) => n + t.pixels.length, 0);
          let changedChannels = 0;
          for (const tile of tiles) for (let row = 0; row < tile.height; row++) for (let column = 0; column < tile.width; column++) for (let k = 0; k < 4; k++)
            if (tile.pixels[(row * tile.width + column) * 4 + k] !== source[((tile.y + row) * width + tile.x + column) * 4 + k]) changedChannels++;
          measures.push({ mode, gpu, preparationMs, maxSubmitMs, maxCompleteMs, maxFrameGapMs, readbackMs, readbackBytes, changedChannels, before, after });
        } finally { stroke.dispose(); gl.getExtension("WEBGL_lose_context")?.loseContext(); }
        await new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
      }
      return measures;
    }, { width, height });
    console.log(JSON.stringify({ size: name, measures: result }));
    for (const r of result) {
      expect(r.gpu).not.toMatch(/SwiftShader|Basic Render|unknown/i);
      expect(r.preparationMs).toBeLessThan(30_000);
      expect(r.maxSubmitMs).toBeLessThan(100); expect(r.maxCompleteMs).toBeLessThan(100); expect(r.maxFrameGapMs).toBeLessThan(100);
      expect(r.after.allocatedBytes).toBeLessThanOrEqual(512 * 1024 * 1024);
      expect(r.after.uploadedBytes).toBe(width * height * 4); expect(r.after.uploadedBytes).toBe(r.before.uploadedBytes);
      expect(r.readbackBytes).toBeLessThan(4 * 1024 * 1024); expect(r.changedChannels).toBeGreaterThan(0);
    }
  });
}
