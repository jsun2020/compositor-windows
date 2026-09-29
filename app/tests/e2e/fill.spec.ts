import { test, expect, type Page } from "@playwright/test";
import { clickMenu, solidPngBase64 } from "./helpers";

// Phase 4b-1: Fill with the foreground or background colour, from the keys and the Edit menu,
// through a selection, on a mask, and through the job worker (the engine's Fill is pinned in
// engine/tests/raster_edits.rs).

/** A 60 x 40 canvas with a 30 x 20 grey layer at (10, 10), active, at 4 CSS px a pixel. */
async function setup(page: Page) {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const grey = await page.evaluate(solidPngBase64, { width: 30, height: 20, color: "#808080" });
  await page.evaluate(async (g) => {
    const api = (window as any).__compositor;
    const bytes = (data: string) => Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(60, 40, false);
    api.engine.importImage(doc, bytes(g), "Grey", { x: 25, y: 20 });
    api.store.getState().openDocument(doc);
    await api.setZoom(4);
    const s = api.store.getState();
    s.setPaletteColor({ red: 1, green: 0, blue: 0 }, false);
    s.setPaletteColor({ red: 0, green: 0, blue: 1 }, true);
  }, grey);
}
const pixel = (page: Page, x: number, y: number) => page.evaluate(([x, y]) => {
  const api = (window as any).__compositor; const s = api.store.getState();
  return Array.from(api.engine.composite(s.activeId, { x, y, width: 1, height: 1 }, 1, 1) as Uint8Array);
}, [x, y]);
const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
const select = (page: Page, x0: number, y0: number, x1: number, y1: number) => page.evaluate(([x0, y0, x1, y1]) =>
  (window as any).__compositor.store.getState().run({ type: "SelectShape", kind: "Rectangle", points: [[x0, y0], [x1, y0], [x1, y1], [x0, y1]], mode: "Replace", antialiased: false }), [x0, y0, x1, y1]);

test("Alt+Backspace fills the selection with the foreground, Ctrl+Delete the layer (grown to the canvas) with the background", async ({ page }) => {
  await setup(page);
  await select(page, 15, 12, 25, 18);
  const depth = (await state(page)).undoDepth;
  await page.keyboard.press("Alt+Backspace");
  expect(await pixel(page, 20, 15)).toEqual([255, 0, 0, 255]);
  expect(await pixel(page, 12, 15), "outside the selection").toEqual([128, 128, 128, 255]);
  expect((await state(page)).undoDepth).toBe(depth + 1);
  await page.evaluate(() => (window as any).__compositor.store.getState().run({ type: "Deselect" }));
  await page.keyboard.press("Control+Delete");
  // No selection: the whole layer, grown to cover the canvas as the Mac's raster edit grows it.
  expect(await pixel(page, 2, 2)).toEqual([0, 0, 255, 255]);
  const layer = (await state(page)).layers[0];
  expect([layer.pixelsWidth, layer.pixelsHeight, layer.transform.origin]).toEqual([60, 40, [0, 0]]);
  // Backspace alone still deletes: with no selection, the layer.
  await page.keyboard.press("Backspace");
  expect((await state(page)).layers.length).toBe(0);
});

test("on a targeted mask the fill is its black or white, and the Edit menu offers both fills only when they can paint", async ({ page }) => {
  await setup(page);
  await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); s.run({ type: "AddMask", id: s.documents[s.activeId].activeLayerId, revealing: true }); s.setMaskSelected(true); });
  await select(page, 10, 10, 20, 30);
  await clickMenu(page, "Edit", "fill-foreground");
  expect(await pixel(page, 15, 15), "black hides").toEqual([0, 0, 0, 0]);
  expect(await pixel(page, 25, 15)).toEqual([128, 128, 128, 255]);
  // Hidden, the layer takes no fill: both items grey out.
  await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); s.run({ type: "SetLayerVisible", id: s.documents[s.activeId].activeLayerId, visible: false }); });
  await page.getByRole("button", { name: "Edit", exact: true }).click();
  await expect(page.getByTestId("menu-fill-foreground")).toBeDisabled();
  await expect(page.getByTestId("menu-fill-background")).toBeDisabled();
});

test("a large layer is filled by the job worker, exactly as in place", async ({ page }) => {
  await setup(page);
  await select(page, 15, 12, 45, 38);
  const inPlace = await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState(); const id = s.documents[s.activeId].activeLayerId;
    api.engine.execute(s.activeId, { type: "Fill", id, mask: false, color: [1, 0, 0] });
    const bytes = Array.from(api.engine.layerPixels(s.activeId, id, 0) as Uint8Array);
    api.engine.undo(s.activeId); s.refresh(s.activeId);
    return bytes;
  });
  await page.evaluate(() => {
    const api = (window as any).__compositor; const jobs = api.store.getState().jobs; const run = jobs.run.bind(jobs);
    (window as any).__jobKinds = [];
    jobs.run = (channel: string, request: { kind: string }) => { (window as any).__jobKinds.push(request.kind); return run(channel, request); };
    api.store.setState({ jobPixels: 0 });
  });
  const depth = (await state(page)).undoDepth;
  await page.keyboard.press("Alt+Backspace");
  // Put back as one undo step once the worker is done.
  await expect.poll(async () => (await state(page)).undoDepth).toBe(depth + 1);
  // Ruling I5: the worker made it, not the UI thread.
  expect(await page.evaluate(() => (window as any).__jobKinds), "the fill ran in the job worker").toEqual(["edit"]);
  const viaJob = await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState(); const id = s.documents[s.activeId].activeLayerId;
    return Array.from(api.engine.layerPixels(s.activeId, id, 0) as Uint8Array);
  });
  expect(viaJob.length).toBe(inPlace.length);
  expect(viaJob).toEqual(inPlace);
});
