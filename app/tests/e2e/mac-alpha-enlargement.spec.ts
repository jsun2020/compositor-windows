import { test, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
test("Mac 1.4.5 alpha-grid enlargement agrees on CPU and GPU without changing sampling", async ({ page }) => {
  const oracle = JSON.parse(fs.readFileSync(path.resolve("engine/tests/fixtures/mac-alpha-grid-1.4.5.json"), "utf8"));
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result = await page.evaluate(async () => {
    const api = (window as any).__compositor;
    const input = document.createElement("canvas");
    input.width = input.height = 32;
    const c = input.getContext("2d")!, im = c.createImageData(32, 32);
    for (let y = 0; y < 32; y++)
      for (let x = 0; x < 32; x++) {
        const i = (y * 32 + x) * 4;
        im.data.set([255, 255, 255, (37 * x + 71 * y + 11 * x * y) % 256], i);
      }
    c.putImageData(im, 0, 0);
    const bytes = Uint8Array.from(atob(input.toDataURL("image/png").split(",")[1]), c => c.charCodeAt(0)), doc = api.engine.newDocument(66, 66, false);
    api.engine.importImage(doc, bytes, "Alpha grid", { x: 16, y: 16 });
    const id = api.engine.state(doc).activeLayerId, t = api.engine.state(doc).layers[0].transform;
    api.engine.execute(doc, { type: "SetLayerTransform", id, transform: { ...t, origin: [16, 16], size: [34, 34], sampling: "High quality" } });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
    const comparisons = [];
    for (const sampling of ["High quality", "Smooth"]) {
      api.engine.execute(doc, { type: "SetLayerTransform", id, transform: { ...t, origin: [16, 16], size: [34, 34], sampling } });
      api.store.getState().refresh(doc);
      await new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)));
      const s = api.store.getState(), vp = s.viewports[doc], rect = vp.documentRect({ width: 66, height: 66 }), dpr = window.devicePixelRatio || 1;
      vp.translate({ width: (Math.round(rect.x * dpr) - rect.x * dpr) / dpr, height: (Math.round(rect.y * dpr) - rect.y * dpr) / dpr });
      api.renderer.render(api.engine, s.documents[doc], vp, dpr, { checkerboard: false }, null);
      const gpu = api.readDocumentPixels(), cpu = api.engine.composite(doc, { x: 0, y: 0, width: 66, height: 66 }, 66, 66);
      const crop = [];
      for (let y = 16; y < 50; y++)
        for (let x = 16; x < 50; x++)
          crop.push(cpu[(y * 66 + x) * 4 + 3]);
      comparisons.push({ sampling, savedSampling: JSON.parse(api.engine.savePackage(doc).manifest).layers[0].transform.sampling, alpha: crop, length: gpu.length, expectedLength: cpu.length, max: gpu.reduce((m: number, v: number, i: number) => Math.max(m, Math.abs(v - cpu[i])), 0) });
    }
    return { kind: api.renderer.kind, comparisons };
  });
  expect(result.kind).toBe("gl");
  for (const r of result.comparisons) {
    expect(r.savedSampling).toBe(r.sampling);
    expect(r.length).toBe(r.expectedLength);
    expect(r.max).toBeLessThanOrEqual(2);
    expect(r.alpha).toEqual(oracle.alpha);
  }
});
