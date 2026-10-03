import { test, expect, type Page } from "@playwright/test";
import { grayRampMaskPngBase64, redSquarePngBase64, solidPngBase64 } from "./helpers";

test("fractional pixel-copy edges use Mac's 8-bit coverage with either axis flipped", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const records = await page.evaluate(async () => {
    const api = (window as any).__compositor;
    const canvas = document.createElement("canvas"); canvas.width = 2; canvas.height = 2;
    canvas.getContext("2d")!.putImageData(new ImageData(new Uint8ClampedArray([
      230, 40, 30, 255, 20, 20, 20, 255,
      30, 90, 220, 255, 40, 180, 90, 255,
    ]), 2, 2), 0, 0);
    const bytes = Uint8Array.from(atob(canvas.toDataURL().split(",")[1]), c => c.charCodeAt(0));
    const id = "A66C3B15-160E-41C5-8E7F-1AAB95F51D79", result = [];
    for (const [flipX, flipY] of [[false, false], [true, false], [false, true], [true, true]]) {
      const manifest = { format: "com.compositor.project", version: 11, colorSpace: "sRGB", documentID: id, width: 6, height: 6,
        layers: [{ id, name: "Edge", isVisible: true, imageFile: `${id}.png`, transform: { origin: [1.5, 1.25], size: [2, 2], rotation: 0, flipX, flipY, sampling: "High quality" } }] };
      const doc = api.engine.openPackage({ manifest: JSON.stringify(manifest), images: [{ name: `${id}.png`, bytes }] }, null);
      api.store.getState().openDocument(doc); await api.setZoom(1); api.setCheckerboard(false);
      await new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)));
      const state = api.store.getState(), vp = state.viewports[doc], dpr = window.devicePixelRatio || 1;
      const rect = vp.documentRect({ width: 6, height: 6 });
      vp.translate({ width: (Math.round(rect.x * dpr) - rect.x * dpr) / dpr, height: (Math.round(rect.y * dpr) - rect.y * dpr) / dpr });
      api.renderer.render(api.engine, state.documents[doc], vp, dpr, { checkerboard: false }, null);
      const gpu = Array.from(api.readDocumentPixels()) as number[], cpu = Array.from(api.engine.composite(doc, { x: 0, y: 0, width: 6, height: 6 }, 6, 6)) as number[];
      result.push({ kind: state.rendererKind, worst: gpu.reduce((m, v, i) => Math.max(m, Math.abs(v - cpu[i])), 0), corner: cpu.slice(28, 32) });
      api.store.getState().closeDocument(doc);
    }
    return result;
  });
  records.forEach(record => { expect(record.kind).toBe("gl"); expect(record.worst).toBeLessThanOrEqual(2); });
  // Coverage 0.5 × 0.75 becomes 95/255 before multiplying premultiplied RGBA.
  expect(records.map(record => record.corner)).toEqual([[86, 15, 11, 95], [7, 7, 7, 95], [11, 34, 82, 95], [15, 67, 34, 95]]);
});

test("an upright fractional 1:1 layer copies its pixels on the GPU and export without rewriting its transform", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result = await page.evaluate(async () => {
    const api = (window as any).__compositor;
    const canvas = document.createElement("canvas"); canvas.width = 5; canvas.height = 4;
    const ctx = canvas.getContext("2d")!;
    ctx.putImageData(new ImageData(new Uint8ClampedArray([
      255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 64,
      30, 20, 10, 255, 40, 50, 60, 255, 70, 80, 90, 255,
    ]), 3, 2), 1, 1);
    const bytes = Uint8Array.from(atob(canvas.toDataURL().split(",")[1]), c => c.charCodeAt(0));
    const id = "A66C3B15-160E-41C5-8E7F-1AAB95F51D79", records = [];
    for (const sampling of ["Smooth", "High quality"]) {
      const transform = { origin: [10.5, 20.25], size: [5, 4], rotation: 0, flipX: false, flipY: false, sampling };
      const manifest = { format: "com.compositor.project", version: 11, colorSpace: "sRGB", documentID: id, width: 20, height: 30,
        layers: [{ id, name: "Pattern", isVisible: true, imageFile: `${id}.png`, transform }] };
      const doc = api.engine.openPackage({ manifest: JSON.stringify(manifest), images: [{ name: `${id}.png`, bytes }] }, null);
      api.store.getState().openDocument(doc); await api.setZoom(1); api.setCheckerboard(false);
      await new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)));
      const gpu = Array.from(api.readDocumentPixels()) as number[], cpu = Array.from(api.engine.composite(doc, { x: 0, y: 0, width: 20, height: 30 }, 20, 30)) as number[];
      const crop = [];
      for (let y = 21; y < 23; y++) for (let x = 11; x < 14; x++) crop.push(...cpu.slice((y * 20 + x) * 4, (y * 20 + x + 1) * 4));
      const saved = JSON.parse(api.engine.savePackage(doc).manifest);
      records.push({ kind: api.store.getState().rendererKind, crop, worst: gpu.reduce((m, v, i) => Math.max(m, Math.abs(v - cpu[i])), 0), mismatches: gpu.flatMap((v, i) => Math.abs(v - cpu[i]) > 2 ? [{ x: Math.floor(i / 4) % 20, y: Math.floor(i / 80), channel: i % 4, gpu: v, cpu: cpu[i] }] : []).slice(0, 8), transform: saved.layers[0].transform });
      api.store.getState().closeDocument(doc);
    }
    return records;
  });
  for (const [index, record] of result.entries()) {
    expect(record.kind).toBe("gl");
    expect(record.worst, JSON.stringify(record.mismatches)).toBeLessThanOrEqual(2);
    expect(record.crop).toEqual([255, 0, 0, 255, 0, 128, 0, 128, 0, 0, 64, 64, 30, 20, 10, 255, 40, 50, 60, 255, 70, 80, 90, 255]);
    expect(record.transform.origin).toEqual([10.5, 20.25]);
    expect(record.transform.sampling).toBe(["Smooth", "High quality"][index]);
  }
});

/** Reads the on-screen document pixels and compares them with the CPU compositor within 2/255 per byte. */
async function matchesCpu(page: Page, label: string): Promise<void> {
  const result = await page.evaluate(async () => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const s = api.store.getState(); const d = s.documents[s.activeId];
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const gl = Array.from(api.readDocumentPixels()) as number[];
    const cpu = Array.from(api.engine.composite(d.id, { x: 0, y: 0, width: d.width, height: d.height }, d.width, d.height)) as number[];
    return { gl, cpu, kind: s.rendererKind };
  });
  expect(result.gl.length).toBe(result.cpu.length);
  const worst = result.gl.reduce((m, v, i) => Math.max(m, Math.abs(v - result.cpu[i])), 0);
  expect(worst, `${label} (${result.kind}) max byte diff`).toBeLessThanOrEqual(2);
}

/** A 4x4 document with a red 2x2 layer at (1,1) rendered at zoom 1 must match the engine's CPU composite. */
test("gl renderer matches the CPU compositor", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(redSquarePngBase64);
  await page.evaluate(async (b64) => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const doc = api.engine.newDocument(4, 4, false);
    const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    api.engine.importImage(doc, bytes, "red", { x: 2, y: 2 });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
  }, b64);
  await matchesCpu(page, "red square");
});

test("a flipped layer's mask, redrawn upright by Image Size, shows on the GPU as on the CPU", async ({ page }) => {
  // Final review I2: Image Size redraws a turned or flipped layer and its covering mask upright on
  // the new grid. A mask texture kept from before would lay the old, flipped ramp over the new
  // upright placement: the mask mirrored.
  const DOC = "5A1D2C3B-0000-4000-8000-000000000000", DOT = "5A1D2C3B-0000-4000-8000-000000000001", RAMP = "5A1D2C3B-0000-4000-8000-000000000002";
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const red = await page.evaluate(solidPngBase64, { width: 40, height: 24, color: "#ff0000" });
  const blue = await page.evaluate(solidPngBase64, { width: 4, height: 4, color: "#0000ff" });
  const mask = await page.evaluate(grayRampMaskPngBase64);
  const t = (x: number, y: number, w: number, h: number, flipX = false) => ({ origin: [x, y], size: [w, h], rotation: 0, flipX, flipY: false, sampling: "High quality" });
  const manifest = {
    format: "com.compositor.project", version: 9, colorSpace: "sRGB", documentID: DOC, width: 96, height: 72,
    layers: [
      { id: DOT, name: "Dot", isVisible: true, imageFile: `${DOT}.png`, transform: t(2, 2, 4, 4) },
      { id: RAMP, name: "Ramp", isVisible: true, imageFile: `${RAMP}.png`, maskFile: `${RAMP}.mask.png`, transform: t(28, 24, 40, 24, true) },
    ],
  };
  const files = { [`${DOT}.png`]: blue, [`${RAMP}.png`]: red, [`${RAMP}.mask.png`]: mask };
  await page.evaluate(async ({ manifest, files }) => {
    const api = (window as any).__compositor;
    const images = Object.entries(files).map(([name, b64]) => ({ name, bytes: Uint8Array.from(atob(b64 as string), (c: string) => c.charCodeAt(0)) }));
    const doc = api.engine.openPackage({ manifest: JSON.stringify(manifest), images }, null);
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
    const dot = api.engine.state(doc).layers[0].id;
    api.store.getState().selectLayers([dot], dot);
  }, { manifest, files });
  await matchesCpu(page, "as opened (the frame uploads the flipped mask)");
  const depth = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].undoDepth; });
  const resized = await page.evaluate(() => (window as any).__compositor.store.getState().run({ type: "ImageSize", width: 192, height: 144, resolution: 72, sampling: "High quality" }));
  expect(resized, "Image Size is accepted").toBe(true);
  const r = await page.evaluate(async () => {
    const api = (window as any).__compositor;
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const s = api.store.getState(); const d = s.documents[s.activeId]; const dpr = window.devicePixelRatio || 1;
    const rect = s.viewports[s.activeId].documentRect({ width: d.width, height: d.height });
    const onPixel = (v: number) => Math.abs(v - Math.round(v)) < 1e-9;
    return { depth: d.undoDepth, width: d.width, whole: onPixel(rect.x * dpr) && onPixel(rect.y * dpr) };
  });
  expect(r.depth, "one history step").toBe(depth + 1);
  expect(r.whole, "the document sits on whole device pixels").toBe(true);
  await matchesCpu(page, "after Image Size x2");
  // The mask's own formula: the 32-wide ramp (column i is round(i / 31 * 255)) stretched over the
  // layer and mirrored by its flip, sampled bilinearly at the pixel centre, is the red's alpha.
  const ramp = (i: number) => Math.round((Math.min(31, Math.max(0, i)) / 31) * 255);
  const expected = (x: number) => {
    const u = ((x + 0.5) / 2 - 28) / 40; // across the layer, before the resize
    const m = (1 - u) * 32 - 0.5; const i = Math.floor(m);
    return ramp(i) + (ramp(i + 1) - ramp(i)) * (m - i);
  };
  const columns = [60, 76, 96, 116, 132]; // across the resized layer (56 to 136), row 72
  const alphas = await page.evaluate((columns) => {
    const px = (window as any).__compositor.readDocumentPixels() as Uint8Array;
    return columns.map((x) => px[(72 * 192 + x) * 4 + 3]);
  }, columns);
  // Measured 2026-09-26: at most 0.6 from the formula (the two resamplings round). The mirrored
  // mask a stale texture gives is about 237 off at either end.
  columns.forEach((x, k) => expect(Math.abs(alphas[k] - expected(x)), `alpha at x ${x}: ${alphas[k]}, formula ${expected(x)}`).toBeLessThanOrEqual(1));
});

test("cpu fallback renders when WebGL2 is unavailable", async ({ page }) => {
  await page.addInitScript(() => {
    const original = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (type: string, ...rest: unknown[]) {
      if (type === "webgl2") return null;
      return (original as any).call(this, type, ...rest);
    };
  });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const kind = await page.evaluate(async () => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const doc = api.engine.newDocument(4, 4, true);
    api.store.getState().openDocument(doc);
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    return api.store.getState().rendererKind;
  });
  expect(kind).toBe("cpu");
});
