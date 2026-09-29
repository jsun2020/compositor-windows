import { test, expect, type Page } from "@playwright/test";
import { solidPngBase64 } from "./helpers";

// Task 14a: a gradient on a mask grows the mask past its layer to the canvas (Compositor 1.3.7, the
// user's decision of 2026-09-29; engine/tests/mask_grow.rs pins the engine). The GPU must draw what the
// CPU compositor draws while it is previewed, once applied, once its layer moves, and once undone.

/** A 120 x 80 canvas with a 40 x 30 grey layer at (30, 25) under a white 1 x 1 mask, at 1:1. */
async function setup(page: Page) {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const grey = await page.evaluate(solidPngBase64, { width: 40, height: 30, color: "#808080" });
  return page.evaluate(async (g) => {
    const api = (window as any).__compositor;
    const bytes = Uint8Array.from(atob(g), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(120, 80, false);
    api.engine.importImage(doc, bytes, "Grey", { x: 50, y: 40 });
    const layer = api.engine.state(doc).activeLayerId;
    api.engine.execute(doc, { type: "AddMask", id: layer, revealing: true });
    api.store.getState().openDocument(doc);
    api.setCheckerboard(false);
    await api.setZoom(1);
    return { doc, layer };
  }, grey);
}
/** The largest channel difference between the GPU's picture of the document and the CPU compositor's. */
const worstGpuVsCpu = (page: Page) => page.evaluate(async () => {
  const api = (window as any).__compositor; const s = api.store.getState(); const d = s.documents[s.activeId];
  await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
  const gl = api.readDocumentPixels() as Uint8Array;
  const cpu = api.engine.compositeEdit(d.id, null, { x: 0, y: 0, width: d.width, height: d.height }, d.width, d.height) as Uint8Array;
  // Past any channel's reach: a picture of the wrong size fails every `<= 2` below (audit M-6).
  if (gl.length !== cpu.length) return 256;
  let worst = 0;
  for (let i = 0; i < gl.length; i++) worst = Math.max(worst, Math.abs(gl[i] - cpu[i]));
  return worst;
});
const alphaAt = (page: Page, x: number, y: number) => page.evaluate(([x, y]) => {
  const api = (window as any).__compositor; const s = api.store.getState();
  return (api.engine.composite(s.activeId, { x, y, width: 1, height: 1 }, 1, 1) as Uint8Array)[3];
}, [x, y]);
const maskState = (page: Page, doc: string) => page.evaluate((doc) => {
  const s = (window as any).__compositor.engine.state(doc); const l = s.layers[0];
  return { w: l.maskWidth, h: l.maskHeight, placement: l.maskPlacement, depth: s.undoDepth };
}, doc);

test("a mask gradient grows the mask to the canvas: the GPU draws what the CPU draws, previewed, applied, moved and undone", async ({ page }) => {
  const { doc, layer } = await setup(page);
  // Black fading to nothing across the canvas over the white mask: at pixel x (centre x + 0.5) the mask
  // is 255 (x + 0.5) / 120, rounded, and the opaque grey layer shows with that alpha.
  const gradient = { shape: "Linear", start: [0, 40], end: [120, 40], from: [0, 0, 0, 1], to: [0, 0, 0, 0], opacity: 1 };
  const expected = (x: number) => Math.floor(255 * (x + 0.5) / 120 + 0.5);
  // The size the app decides the job worker by is the grown mask's (Engine::edit_pixels, ruling C1).
  expect(await page.evaluate(({ doc, layer }) => (window as any).__compositor.engine.editPixels(doc, layer, true), { doc, layer })).toBe(120 * 80);
  const before = await maskState(page, doc);
  // Previewed (120 x 80 is under GRADIENT_DRAG_LIMIT, so not reduced): the mask shown grown and placed.
  await page.evaluate(({ layer, gradient }) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    api.engine.setPreview(s.activeId, { preview: "Gradient", layer, mask: true, gradient, dragging: true }); s.refresh(s.activeId);
  }, { layer, gradient });
  const previewed = await maskState(page, doc);
  expect([previewed.w, previewed.h]).toEqual([120, 80]);
  expect(previewed.placement).toMatchObject({ origin: [0, 0], size: [120, 80] });
  expect(await worstGpuVsCpu(page), "previewed").toBeLessThanOrEqual(2);
  for (const x of [32, 50, 67]) expect(Math.abs((await alphaAt(page, x, 40)) - expected(x)), `previewed at ${x}`).toBeLessThanOrEqual(1);
  // Applied: one undo step, the mask where the preview showed it.
  await page.evaluate(({ layer, gradient }) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    api.engine.setPreview(s.activeId, null); s.run({ type: "Gradient", id: layer, mask: true, gradient });
  }, { layer, gradient });
  const applied = await maskState(page, doc);
  expect([applied.w, applied.h, applied.depth]).toEqual([120, 80, before.depth + 1]);
  expect(applied.placement).toMatchObject({ origin: [0, 0], size: [120, 80] });
  expect(await worstGpuVsCpu(page), "applied").toBeLessThanOrEqual(2);
  for (const x of [32, 50, 67]) expect(Math.abs((await alphaAt(page, x, 40)) - expected(x)), `applied at ${x}`).toBeLessThanOrEqual(1);
  // Linked, the mask moves with its layer: 10 px right, the layer shows what the mask held 10 px left.
  await page.evaluate(({ layer }) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    const t = api.engine.state(s.activeId).layers[0].transform;
    s.run({ type: "SetLayerTransform", id: layer, transform: { ...t, origin: [t.origin[0] + 10, t.origin[1]] } });
  }, { layer });
  expect((await maskState(page, doc)).placement).toMatchObject({ origin: [10, 0], size: [120, 80] });
  expect(await worstGpuVsCpu(page), "moved").toBeLessThanOrEqual(2);
  for (const x of [42, 60, 77]) expect(Math.abs((await alphaAt(page, x, 40)) - expected(x - 10)), `moved, at ${x}`).toBeLessThanOrEqual(1);
  // Undone twice: the 1 x 1 mask covering its layer again.
  await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); s.undo(); s.undo(); });
  const undone = await maskState(page, doc);
  expect([undone.w, undone.h, undone.placement, undone.depth]).toEqual([1, 1, null, before.depth]);
  expect(await worstGpuVsCpu(page), "undone").toBeLessThanOrEqual(2);
});

test("Alt+Backspace and Delete in a selection on a targeted mask grow it to the canvas too", async ({ page }) => {
  const { doc } = await setup(page);
  // The left half of the layer selected, the mask targeted. The 120 x 80 canvas is less than a 256 px tile
  // across, so a fill inside the selection grows the mask to all of it (SelectionEdits.swift:200-210).
  await page.evaluate(() => {
    const s = (window as any).__compositor.store.getState();
    s.setMaskSelected(true);
    s.run({ type: "SelectShape", kind: "Rectangle", points: [[30, 25], [50, 25], [50, 55], [30, 55]], mode: "Replace", antialiased: false });
  });
  const before = await maskState(page, doc);
  // The mask palette's foreground is black: the selected half hides, the other half still shows.
  await page.keyboard.press("Alt+Backspace");
  const filled = await maskState(page, doc);
  expect([filled.w, filled.h, filled.depth]).toEqual([120, 80, before.depth + 1]);
  expect(filled.placement).toMatchObject({ origin: [0, 0], size: [120, 80] });
  expect([await alphaAt(page, 40, 40), await alphaAt(page, 60, 40)]).toEqual([0, 255]);
  expect(await worstGpuVsCpu(page), "filled").toBeLessThanOrEqual(2);
  // Delete with the selection fills it with the mask's background colour, white (Task 14): a Fill, grown alike.
  await page.evaluate(() => (window as any).__compositor.store.getState().undo());
  expect((await maskState(page, doc)).w).toBe(1);
  await page.keyboard.press("Delete");
  const deleted = await maskState(page, doc);
  expect([deleted.w, deleted.h, deleted.depth]).toEqual([120, 80, before.depth + 1]);
  expect(deleted.placement).toMatchObject({ origin: [0, 0], size: [120, 80] });
  expect([await alphaAt(page, 40, 40), await alphaAt(page, 60, 40)]).toEqual([255, 255]);
  expect(await worstGpuVsCpu(page), "deleted").toBeLessThanOrEqual(2);
});
