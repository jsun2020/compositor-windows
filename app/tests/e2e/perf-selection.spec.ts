import { test, expect } from "@playwright/test";

// Phase 4a final review F1 and F3, timed in the release engine: run `pnpm wasm`, then
// `$env:PERF = "1"; pnpm e2e -- perf-selection`, and rebuild `pnpm wasm:dev` afterwards. The
// ceilings are generous regression nets; the fix report records the measured numbers.
test.skip(!process.env.PERF, "set PERF=1 after pnpm wasm to measure");

test("Levels drag ticks on a 6000 x 4000 layer under a selection, in the release engine", async ({ page }) => {
  test.setTimeout(600_000);
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const times = await page.evaluate(() => {
    const api = (window as any).__compositor;
    const out: Record<string, number[]> = {};
    // A default Levels adjustment as the engine serializes it, from a scratch adjustment layer.
    const scratch = api.engine.newDocument(10, 10, false);
    api.engine.execute(scratch, { type: "AddAdjustmentLayer", kind: "Levels", seed: 0, shadows: null, highlights: null });
    const base = api.engine.state(scratch).layers[0].adjustment;
    api.engine.closeDocument(scratch);
    const cases: [string, number | null][] = [["no selection", null], ["plain ellipse", 0], ["ellipse feather 20", 20], ["ellipse feather 63", 63]];
    for (const [label, feather] of cases) {
      // Canvas Size's fill makes one opaque 6000 x 4000 layer without decoding an image.
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: 6000, height: 4000, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const layer = api.engine.state(doc).layers[0];
      if (feather !== null) {
        api.engine.execute(doc, { type: "SelectShape", kind: "Ellipse", points: [[700, 500], [5300, 500], [5300, 3500], [700, 3500]], mode: "Replace", antialiased: true });
        if (feather > 0) api.engine.execute(doc, { type: "FeatherSelection", amount: feather });
      }
      const ticks: number[] = [];
      for (let i = 0; i < 4; i++) {
        const adjustment = JSON.parse(JSON.stringify(base));
        adjustment.levels.ranges[0].outputWhite = 200 + i;
        const t0 = performance.now();
        api.engine.setPreview(doc, { preview: "DragAdjustment", layer: layer.id, adjustment });
        ticks.push(Math.round(performance.now() - t0));
      }
      out[label] = ticks;
      api.engine.closeDocument(doc);
    }
    return out;
  });
  console.log(`F1 drag ticks (ms, release wasm, tick 1 first after the selection changed): ${JSON.stringify(times)}`);
  for (const ticks of Object.values(times)) for (const t of ticks.slice(1)) expect(t).toBeLessThan(2_000);
});

test("the ants of a four-million-point wand outline at 1:1 and 1:2, in the release engine", async ({ page }) => {
  test.setTimeout(600_000);
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const times = await page.evaluate(async () => {
    const api = (window as any).__compositor;
    const doc = api.engine.newDocument(10, 10, false);
    api.engine.execute(doc, { type: "CanvasSize", width: 4000, height: 3000, anchor: 4, fill: [0.5, 0.4, 0.3] });
    const layer = api.engine.state(doc).layers[0].id;
    api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
    api.engine.execute(doc, { type: "ApplyFilter", id: layer, params: { filter: "AddNoise", amount: 25, gaussian: true, monochromatic: false, seed: 7 } });
    api.engine.execute(doc, { type: "MagicWand", at: [1000, 1000], mode: "Replace", settings: { tolerance: 16, sampleRadius: 0, contiguous: false, allLayers: false }, antialiased: true });
    const out: Record<string, number> = { points: api.engine.state(doc).selection.points };
    // The engine side alone: the flat outline at 1:1, and the LOD trace at 1:2.
    let t0 = performance.now(); api.engine.selectionOutline(doc, 1); out["outline fetch at 1:1"] = Math.round(performance.now() - t0);
    t0 = performance.now(); api.engine.selectionOutline(doc, 0.5); out["LOD trace at 1:2"] = Math.round(performance.now() - t0);
    api.store.getState().openDocument(doc);
    await new Promise((r) => setTimeout(r, 500));
    // Reading one overlay pixel back makes the canvas carry out the queued strokes now, inside the timing.
    const overlay = document.querySelector('[data-testid="overlay"]') as HTMLCanvasElement;
    const paint = () => { api.paintOverlay(); overlay.getContext("2d")!.getImageData(0, 0, 1, 1); };
    for (const [label, zoom] of [["1:1", 1], ["1:2", 0.5]] as [string, number][]) {
      // A new zoom: the next frame fetches the outline (traced at 1:2), builds its path and strokes it.
      t0 = performance.now(); await api.setZoom(zoom); paint(); out[`first frame at ${label}`] = Math.round(performance.now() - t0);
      await new Promise((r) => setTimeout(r, 300));
      const ticks: number[] = [];
      for (let i = 0; i < 5; i++) { t0 = performance.now(); paint(); ticks.push(performance.now() - t0); }
      out[`ants tick at ${label} (mean of 5)`] = Math.round(10 * ticks.reduce((a, b) => a + b, 0) / ticks.length) / 10;
      // What the page feels while the ants march on their own timer: the longest gap between
      // animation frames over two seconds (about 17 ms when nothing holds the main thread up).
      let last = performance.now(), gap = 0, frames = 0;
      await new Promise<void>((done) => { const end = last + 2000; const frame = (t: number) => { frames++; gap = Math.max(gap, t - last); last = t; if (t < end) requestAnimationFrame(frame); else done(); }; requestAnimationFrame(frame); });
      out[`longest frame gap at ${label}`] = Math.round(gap);
      out[`frames in 2 s at ${label}`] = frames;
    }
    // The same two seconds with nothing selected, at 1:2: the page's own frame gap here.
    api.store.getState().run({ type: "Deselect" });
    await new Promise((r) => setTimeout(r, 300));
    let last = performance.now(), gap = 0, frames = 0;
    await new Promise<void>((done) => { const end = last + 2000; const frame = (t: number) => { frames++; gap = Math.max(gap, t - last); last = t; if (t < end) requestAnimationFrame(frame); else done(); }; requestAnimationFrame(frame); });
    out["longest frame gap with no selection"] = Math.round(gap);
    out["frames in 2 s with no selection"] = frames;
    return out;
  });
  console.log(`F3 ants (ms, release wasm): ${JSON.stringify(times)}`);
  expect(times.points).toBeGreaterThan(100_000);
  expect(times["ants tick at 1:1 (mean of 5)"]).toBeLessThan(5_000);
});
