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

test("layer effects on a 3000 x 2000 layer export, redraw and read within budget in the release engine", async ({ page }) => {
  test.setTimeout(300_000);
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const times = await page.evaluate(() => {
    const api = (window as any).__compositor;
    // Canvas Size's fill makes one opaque 3000 x 2000 layer without decoding an image; saved and
    // reopened with all six effects on it (the drop shadow sets the margin, 82).
    const base = api.engine.newDocument(10, 10, false);
    api.engine.execute(base, { type: "CanvasSize", width: 3000, height: 2000, anchor: 4, fill: [0.8, 0.3, 0.2] });
    const files = api.engine.savePackage(base);
    const manifest = JSON.parse(files.manifest);
    manifest.layers[0].effects = {
      stroke: { blue: 0.2, green: 0.8, inside: false, opacity: 0.9, red: 0.1, size: 10 },
      shadow: { angle: 120, blue: 0.3, blur: 20, distance: 20, green: 0.2, opacity: 0.75, red: 0.2 },
      colorOverlay: { blue: 0.4, green: 0.1, opacity: 0.3, red: 0.9 },
      innerShadow: { angle: -35, blue: 0.05, blur: 10, distance: 8, green: 0.05, opacity: 0.6, red: 0.05 },
      outerGlow: { blue: 0.2, green: 0.9, opacity: 0.6, red: 1, size: 20 },
      innerGlow: { blue: 1, green: 1, opacity: 0.5, red: 1, size: 10 },
    };
    const out: Record<string, number> = {};
    const time = (label: string, f: () => void) => { const t0 = performance.now(); f(); out[label] = Math.round(performance.now() - t0); };
    const doc = api.engine.openPackage({ manifest: JSON.stringify(manifest), images: files.images }, null);
    const layer = api.engine.state(doc).layers[0].id;
    time("first texture (makes the effects image)", () => api.engine.drawPixels(doc, layer, 0, null));
    time("texture at level 2 (image kept)", () => api.engine.drawPixels(doc, layer, 2, null));
    time("export PNG (image kept)", () => api.engine.exportPng(doc));
    time("eyedropper", () => api.engine.sampleColor(doc, { x: 1500, y: 1000 }));
    // What remakes the image while editing (ruling F-I4): new pixels, and every frame of a mask
    // drag. A layer drag with a linked mask placed apart reuses it.
    api.engine.execute(doc, { type: "InvertPixels", id: layer, mask: false });
    time("texture after a pixel edit (makes a new image)", () => api.engine.drawPixels(doc, layer, 0, null));
    api.engine.execute(doc, { type: "AddMask", id: layer, revealing: true });
    const t = api.engine.state(doc).layers[0].transform;
    api.engine.execute(doc, { type: "SetMaskPlacement", id: layer, placement: { ...t, origin: [t.origin[0] + 40, t.origin[1] + 30] } });
    api.engine.drawPixels(doc, layer, 0, null);
    const moved = { ...t, origin: [t.origin[0] + 25, t.origin[1] + 10] };
    time("layer drag frame, linked placed mask (image kept)", () => api.engine.drawPixels(doc, layer, 0, { kind: "layer", id: layer, draft: moved, corners: null }));
    time("mask drag frame (makes a new image)", () => api.engine.drawPixels(doc, layer, 0, { kind: "mask", id: layer, draft: moved }));
    manifest.layers[0].effects.shadow.blur = 500;
    const wide = api.engine.openPackage({ manifest: JSON.stringify(manifest), images: files.images }, null);
    time("export PNG, shadow blur 500 (halved four times)", () => api.engine.exportPng(wide));
    return out;
  });
  console.log(`effects timings (ms, release wasm): ${JSON.stringify(times)}`);
  for (const label of ["first texture (makes the effects image)", "export PNG (image kept)", "export PNG, shadow blur 500 (halved four times)",
    "texture after a pixel edit (makes a new image)", "mask drag frame (makes a new image)"]) expect(times[label], label).toBeLessThan(20_000);
  for (const label of ["texture at level 2 (image kept)", "eyedropper", "layer drag frame, linked placed mask (image kept)"]) expect(times[label], label).toBeLessThan(2_000);
});
