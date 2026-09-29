import { test, expect, type Page } from "@playwright/test";

// Phase 4b-1: a gradient's preview on the GPU (engine/tests/gradient_preview.rs pins the engine). A
// patch inside a small selection reaches the texture as its rectangle alone; a preview small enough
// not to be reduced draws exactly what applying the gradient draws; clearing it goes back the same way.

/** Counts texture uploads and the texels `texSubImage2D` sends, as partial-upload.spec.ts does. */
async function countUploads(page: Page) {
  await page.addInitScript(() => {
    const log = { image: 0, sub: 0, subArea: 0 };
    (window as any).__uploads = log;
    const proto = WebGL2RenderingContext.prototype as any;
    const image = proto.texImage2D, sub = proto.texSubImage2D;
    proto.texImage2D = function (...args: unknown[]) { log.image++; return image.apply(this, args); };
    proto.texSubImage2D = function (...args: unknown[]) { log.sub++; log.subArea += (args[4] as number) * (args[5] as number); return sub.apply(this, args); };
  });
}
const frames = (page: Page) => page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
const uploads = (page: Page) => page.evaluate(() => ({ ...(window as any).__uploads }));
const resetUploads = (page: Page) => page.evaluate(() => { const u = (window as any).__uploads; u.image = 0; u.sub = 0; u.subArea = 0; });
const keep = (page: Page, name: string) => page.evaluate((name) => { (window as any)[name] = (window as any).__compositor.readDocumentPixels(); }, name);
/** The largest channel difference between the GPU's picture now and a kept one. */
const worstAgainst = (page: Page, name: string) => page.evaluate((name) => {
  const now = (window as any).__compositor.readDocumentPixels() as Uint8Array, kept = (window as any)[name] as Uint8Array;
  if (now.length !== kept.length) return -1;
  let worst = 0;
  for (let i = 0; i < now.length; i++) worst = Math.max(worst, Math.abs(now[i] - kept[i]));
  return worst;
}, name);
const setPreview = (page: Page, request: unknown) => page.evaluate((request) => {
  const api = (window as any).__compositor; const s = api.store.getState();
  api.engine.setPreview(s.activeId, request); s.refresh(s.activeId);
}, request);

/** A 600 x 401 noise layer that covers its canvas, active, at 1:1. */
async function setup(page: Page) {
  await page.setViewportSize({ width: 1280, height: 720 });
  await countUploads(page);
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const ids = await page.evaluate(async () => {
    const api = (window as any).__compositor;
    const doc = api.engine.newDocument(10, 10, false);
    api.engine.execute(doc, { type: "CanvasSize", width: 600, height: 401, anchor: 4, fill: [0.5, 0.4, 0.3] });
    const layer = api.engine.state(doc).layers[0].id;
    api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
    api.engine.execute(doc, { type: "ApplyFilter", id: layer, params: { filter: "AddNoise", amount: 30, gaussian: false, monochromatic: false, seed: 3 } });
    api.store.getState().openDocument(doc);
    api.setCheckerboard(false);
    await api.setZoom(1);
    return { doc, layer };
  });
  await frames(page);
  return ids;
}
const gradient = { shape: "Radial", start: [220, 190], end: [300, 240], from: [1, 0.2, 0, 1], to: [0, 0, 1, 0.3], opacity: 0.9 };

test("a gradient inside a small selection previews as a patch taken as its rectangle, draws what applying it draws, and clears the same way", async ({ page }) => {
  const { layer } = await setup(page);
  await page.evaluate(() => (window as any).__compositor.store.getState().run({ type: "SelectShape", kind: "Rectangle", points: [[193, 161], [393, 161], [393, 301], [193, 301]], mode: "Replace", antialiased: true }));
  await frames(page);
  await keep(page, "__before");
  await resetUploads(page);
  await setPreview(page, { preview: "Gradient", layer, mask: false, gradient, dragging: true });
  await frames(page);
  const patched = await uploads(page);
  expect(patched.image, "no texture made afresh").toBe(0);
  expect(patched.sub).toBeGreaterThan(0);
  // The selection's 200 x 140 box plus the clip's pixel and the sampling pixel on each side, at most.
  expect(patched.subArea).toBeLessThanOrEqual(204 * 144 + 16);
  await keep(page, "__previewed");
  await resetUploads(page);
  await setPreview(page, null);
  await frames(page);
  const cleared = await uploads(page);
  expect(cleared.image).toBe(0);
  expect(cleared.subArea).toBeLessThanOrEqual(204 * 144 + 16);
  expect(await worstAgainst(page, "__before"), "clearing the patch restores the layer").toBe(0);
  await page.evaluate(({ layer, gradient }) => (window as any).__compositor.store.getState().run({ type: "Gradient", id: layer, mask: false, gradient }), { layer, gradient });
  await frames(page);
  expect(await worstAgainst(page, "__previewed"), "the patch is the applied gradient").toBe(0);
});

test("a whole-layer gradient and a mask gradient small enough not to be reduced preview as applying them draws", async ({ page }) => {
  const { layer } = await setup(page);
  const linear = { ...gradient, shape: "Linear" };
  await setPreview(page, { preview: "Gradient", layer, mask: false, gradient: linear, dragging: true });
  await frames(page);
  await keep(page, "__previewed");
  await setPreview(page, null);
  await page.evaluate(({ layer, linear }) => (window as any).__compositor.store.getState().run({ type: "Gradient", id: layer, mask: false, gradient: linear }), { layer, linear });
  await frames(page);
  expect(await worstAgainst(page, "__previewed"), "the whole-layer preview is the applied gradient").toBe(0);
  await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); s.undo(); });
  await page.evaluate((layer) => (window as any).__compositor.store.getState().run({ type: "AddMask", id: layer, revealing: true }), layer);
  const grey = { ...gradient, shape: "Linear", start: [40, 0], end: [560, 0], from: [0, 0, 0, 1], to: [1, 1, 1, 1], opacity: 1 };
  await setPreview(page, { preview: "Gradient", layer, mask: true, gradient: grey, dragging: true });
  await frames(page);
  await keep(page, "__masked");
  expect(await worstAgainst(page, "__previewed"), "the mask preview shows").toBeGreaterThan(0);
  await setPreview(page, null);
  await page.evaluate(({ layer, grey }) => (window as any).__compositor.store.getState().run({ type: "Gradient", id: layer, mask: true, gradient: grey }), { layer, grey });
  await frames(page);
  expect(await worstAgainst(page, "__masked"), "the mask preview is the applied mask gradient").toBe(0);
});
