import { test, expect } from "@playwright/test";

// Timings mean something only against the release engine: run `pnpm wasm`, then
// `$env:PERF = "1"; pnpm e2e -- perf-spatial`. The ceilings are generous regression nets (an
// unreduced 250 px blur takes minutes); the task report records the measured numbers.
test.skip(!process.env.PERF, "set PERF=1 after pnpm wasm to measure");

test("blur adjustment layers export a 3000 x 2000 canvas within budget in the release engine", async ({ page }) => {
  test.setTimeout(300_000);
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const times = await page.evaluate(() => {
    const api = (window as any).__compositor;
    const doc = api.engine.newDocument(10, 10, false);
    // Canvas Size's fill makes one opaque 3000 x 2000 layer without decoding an image.
    api.engine.execute(doc, { type: "CanvasSize", width: 3000, height: 2000, anchor: 4, fill: [0.8, 0.3, 0.2] });
    api.engine.execute(doc, { type: "AddAdjustmentLayer", kind: "Gaussian Blur", seed: 0, shadows: null, highlights: null });
    const layer = api.engine.state(doc).layers.find((l: any) => l.adjustment);
    const out: Record<string, number> = {};
    const time = (label: string, settings: object) => {
      api.engine.execute(doc, { type: "SetAdjustment", id: layer.id, adjustment: { ...layer.adjustment, ...settings } });
      const t0 = performance.now(); api.engine.exportPng(doc); out[label] = Math.round(performance.now() - t0);
    };
    time("gaussian 10", { kind: "Gaussian Blur", blurRadius: 10 });
    time("gaussian 250", { kind: "Gaussian Blur", blurRadius: 250 });
    time("motion 90", { kind: "Motion Blur", motionAngle: 30, motionDistance: 90 });
    time("motion 2000", { kind: "Motion Blur", motionAngle: 30, motionDistance: 2000 });
    const t0 = performance.now(); api.engine.sampleColor(doc, { x: 1500, y: 1000 }); out["eyedropper"] = Math.round(performance.now() - t0);
    return out;
  });
  console.log(`spatial timings (ms, release wasm): ${JSON.stringify(times)}`);
  for (const label of ["gaussian 10", "gaussian 250", "motion 90", "motion 2000"]) expect(times[label], label).toBeLessThan(20_000);
  expect(times["eyedropper"]).toBeLessThan(2_000);
});
