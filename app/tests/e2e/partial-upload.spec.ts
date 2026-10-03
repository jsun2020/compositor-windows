import { test, expect, type Page } from "@playwright/test";

// Phase 4b-1: an edit that knows where it changed a layer (engine/tests/changed_rects.rs) reaches the
// GPU as that rectangle alone, and draws exactly what a whole upload draws.

/** Counts every texture upload the page makes, and the texels `texSubImage2D` sends. */
async function countUploads(page: Page) {
  await page.addInitScript(() => {
    const log = { image: 0, imageMax: 0, sub: 0, subArea: 0 };
    (window as any).__uploads = log;
    const proto = WebGL2RenderingContext.prototype as any;
    const image = proto.texImage2D, sub = proto.texSubImage2D;
    proto.texImage2D = function (...args: unknown[]) {
      log.image++;
      if (typeof args[3] === "number" && typeof args[4] === "number") log.imageMax = Math.max(log.imageMax, (args[3] as number) * (args[4] as number));
      return image.apply(this, args);
    };
    proto.texSubImage2D = function (...args: unknown[]) { log.sub++; log.subArea += (args[4] as number) * (args[5] as number); return sub.apply(this, args); };
  });
}
const frames = (page: Page) => page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
const uploads = (page: Page) => page.evaluate(() => ({ ...(window as any).__uploads }));
const resetUploads = (page: Page) => page.evaluate(() => { const u = (window as any).__uploads; u.image = 0; u.imageMax = 0; u.sub = 0; u.subArea = 0; });
/** Keeps the GPU's picture of the document in the page (reading it out costs more than the test). */
const keepPicture = (page: Page) => page.evaluate(() => { (window as any).__kept = (window as any).__compositor.readDocumentPixels(); });
/** The largest channel difference between the GPU's picture now and the kept one; -1 for another size. */
const worstAgainstKept = (page: Page) => page.evaluate(() => {
  const now = (window as any).__compositor.readDocumentPixels() as Uint8Array, kept = (window as any).__kept as Uint8Array;
  if (now.length !== kept.length) return -1;
  let worst = 0;
  for (let i = 0; i < now.length; i++) worst = Math.max(worst, Math.abs(now[i] - kept[i]));
  return worst;
});

for (const zoom of ["1:1", "fit"] as const) {
  test(`a clear inside a selection uploads only its rectangle at ${zoom}, and draws what a whole upload draws`, async ({ page }) => {
    test.setTimeout(120_000);
    await page.setViewportSize({ width: 1280, height: 720 });
    await countUploads(page);
    await page.goto("/");
    await expect(page.getByTestId("engine-ready")).toBeVisible();
    const ids = await page.evaluate(async (zoom) => {
      const api = (window as any).__compositor;
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: 2400, height: 1000, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const layer = api.engine.state(doc).layers[0].id;
      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
      api.engine.execute(doc, { type: "ApplyFilter", id: layer, params: { filter: "AddNoise", amount: 30, gaussian: false, monochromatic: false, seed: 5 } });
      api.store.getState().openDocument(doc);
      if (zoom === "1:1") await api.setZoom(1);
      return { doc, layer };
    }, zoom);
    await frames(page);
    // A selection around the middle, which the view shows at either zoom.
    await page.evaluate(() => (window as any).__compositor.store.getState().run({ type: "SelectShape", kind: "Rectangle", points: [[1010, 370], [1390, 370], [1390, 630], [1010, 630]], mode: "Replace", antialiased: false }));
    await frames(page);
    await resetUploads(page);
    await page.evaluate((layer) => (window as any).__compositor.store.getState().run({ type: "ClearSelectedPixels", id: layer, mask: false }), ids.layer);
    await frames(page);
    const partial = await uploads(page);
    expect(partial.image, "no texture made afresh").toBe(0);
    expect(partial.sub).toBeGreaterThan(0);
    // The selection's 380 x 260 box plus the clip's pixel and the sampling pixel on each side, at most.
    expect(partial.subArea).toBeLessThanOrEqual(384 * 264 + 16);
    await keepPicture(page);
    // The same document again after another one: its textures were dropped, so it uploads whole.
    await resetUploads(page);
    await page.evaluate(async (doc) => {
      const api = (window as any).__compositor;
      api.store.getState().openDocument(api.engine.newDocument(10, 10, true));
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      api.store.getState().setActive(doc);
    }, ids.doc);
    await frames(page);
    // At fit the layer is drawn halved once (1200 x 500); at 1:1 in chunks of up to 2048 x 1000.
    expect((await uploads(page)).imageMax, "uploaded whole this time").toBeGreaterThanOrEqual(1200 * 500);
    expect(await worstAgainstKept(page), "a partial upload draws exactly what a whole one does").toBe(0);
  });
}


test("opening a large canvas fits before its first texture upload and draws its pixels", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await countUploads(page);
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.evaluate(() => {
    const api = (window as any).__compositor, doc = api.engine.newDocument(10, 10, false);
    api.engine.execute(doc, { type: "CanvasSize", width: 6000, height: 4000, anchor: 4, fill: [0.5, 0.4, 0.3] });
    api.store.getState().openDocument(doc);
  });
  await frames(page);
  const first = await uploads(page);
  expect(first.image).toBeGreaterThan(0);
  // At this viewport fit is at least two halvings; an accidental initial 1:1
  // upload makes a 2048 x 2048 chunk even though none of it is needed on screen.
  expect(first.imageMax).toBeLessThanOrEqual(1500 * 1000);
  const sample = await page.evaluate(() => {
    const api = (window as any).__compositor, s = api.store.getState(), doc = s.documents[s.activeId];
    const size = s.viewports[s.activeId].documentRect(doc), pixels = api.readDocumentPixels() as Uint8Array;
    const width = Math.round(size.x + size.width) - Math.round(size.x);
    const height = Math.round(size.y + size.height) - Math.round(size.y);
    // CanvasSize retains the original 10 x 10 white centre; sample the
    // newly filled area, well clear of that retained source and the canvas edge.
    const offset = (Math.floor(height / 4) * width + Math.floor(width / 4)) * 4;
    return Array.from(pixels.subarray(offset, offset + 4));
  });
  expect(sample).toEqual([128, 102, 77, 255]);
});
