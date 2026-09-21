import { test, expect, type Page } from "@playwright/test";
import { redSquarePngBase64 } from "./helpers";

async function setup(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(redSquarePngBase64);
  await page.evaluate(async (b64) => {
    const api = (window as any).__compositor;
    const png = Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(400, 300, false);
    api.engine.importImage(doc, png, "red", { x: 200, y: 150 });
    const id = api.engine.state(doc).activeLayerId;
    const t = api.engine.state(doc).layers[0].transform;
    api.engine.execute(doc, { type: "SetLayerTransform", id, transform: { ...t, origin: [150, 100], size: [100, 100] } });
    api.store.getState().openDocument(doc);
    api.store.getState().setTool("move");
    await api.setZoom(1);
  }, b64);
}
const layer = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].layers[0]; });
const viewPoint = (page: Page, x: number, y: number) => page.evaluate(({ x, y }) => {
  const s = (window as any).__compositor.store.getState(); const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
  const p = vp.viewPoint({ x, y }, { width: d.width, height: d.height }); const r = document.querySelector('[data-testid="canvas-view"]')!.getBoundingClientRect();
  return { x: r.left + p.x, y: r.top + p.y };
}, { x, y });
async function drag(page: Page, from: { x: number; y: number }, to: { x: number; y: number }, mods: string[] = []) {
  const a = await viewPoint(page, from.x, from.y), b = await viewPoint(page, to.x, to.y);
  for (const m of mods) await page.keyboard.down(m);
  await page.mouse.move(a.x, a.y); await page.mouse.down(); await page.mouse.move((a.x + b.x) / 2, (a.y + b.y) / 2); await page.mouse.move(b.x, b.y); await page.mouse.up();
  for (const m of mods) await page.keyboard.up(m);
}

test("dragging moves the layer, even from outside it, with snapping to the canvas edge", async ({ page }) => {
  await setup(page);
  // dy 15, not 10: keeps the moved centre (y 165) 15px from the canvas centre (150),
  // clear of the inclusive 10px snap radius, so this move should not snap.
  await drag(page, { x: 20, y: 20 }, { x: 40, y: 35 });
  expect((await layer(page)).transform.origin).toEqual([170, 115]);
  await drag(page, { x: 200, y: 150 }, { x: 32, y: 150 }); // origin would be 2: snaps to 0
  expect((await layer(page)).transform.origin[0]).toBe(0);
});

test("handles resize and rotate; alt-drag duplicates; arrows nudge", async ({ page }) => {
  await setup(page);
  await drag(page, { x: 250, y: 200 }, { x: 300, y: 250 }); // bottom-right handle
  let l = await layer(page);
  expect(l.transform.size).toEqual([150, 150]);
  await drag(page, { x: 225, y: 72 }, { x: 350, y: 175 }, ["Shift"]); // rotation handle 28px above the top middle
  l = await layer(page);
  expect(l.transform.rotation % 15).toBe(0);
  expect(l.transform.rotation).not.toBe(0);
  await page.keyboard.press("ArrowRight");
  expect((await layer(page)).transform.origin[0]).toBe(l.transform.origin[0] + 1);
  await page.keyboard.press("Shift+ArrowDown");
  expect((await layer(page)).transform.origin[1]).toBe(l.transform.origin[1] + 10);
  await drag(page, { x: 20, y: 20 }, { x: 60, y: 20 }, ["Alt"]);
  const d = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
  expect(d.layers.length).toBe(2);
  expect(d.layers[1].name).toBe("red copy");
});

test("ctrl-drag on a corner distorts; Enter applies; the inspector edits values", async ({ page }) => {
  await setup(page);
  await drag(page, { x: 250, y: 100 }, { x: 300, y: 100 }, ["Control"]); // top-right corner pulled right
  expect(await page.evaluate(() => (window as any).__compositor.store.getState().transformEdit?.corners?.[1])).toEqual([300, 100]);
  await expect(page.getByTestId("transform-apply")).toBeVisible();
  await page.keyboard.press("Enter");
  let l = await layer(page);
  expect(l.transform.rotation).toBe(0);
  expect(l.pixelsRevision).toBe(2);
  expect(l.transform.size[0]).toBeGreaterThan(100);
  await page.getByLabel("X").fill("10");
  await page.keyboard.press("Enter");
  l = await layer(page);
  expect(l.transform.origin[0]).toBe(10);
  await page.getByLabel("Angle").fill("45");
  await page.keyboard.press("Enter");
  expect((await layer(page)).transform.rotation).toBe(45);
});

test("an unlinked mask alone: arrow nudges and Alt-drag move the mask, not the layer", async ({ page }) => {
  await setup(page);
  const id = (await layer(page)).id;
  await page.evaluate((id) => {
    const api = (window as any).__compositor;
    const s = api.store.getState(); const docId = s.activeId;
    api.engine.execute(docId, { type: "AddMask", id, revealing: true });
    api.engine.execute(docId, { type: "SetMaskLinked", id, linked: false });
    s.refresh();
    s.setMaskSelected(true);
  }, id);
  // "Transform Mask" shows as soon as the unlinked mask chip is selected, before any drag.
  await expect(page.getByText("Transform Mask")).toBeVisible();
  const before = await layer(page);
  expect(before.maskPlacement).toBeNull(); // unmoved: still following the layer's own transform
  await page.keyboard.press("ArrowRight");
  const afterNudge = await layer(page);
  expect(afterNudge.maskPlacement.origin).toEqual([before.transform.origin[0] + 1, before.transform.origin[1]]);
  expect(afterNudge.transform.origin).toEqual(before.transform.origin); // the layer itself didn't move
  await drag(page, { x: 20, y: 20 }, { x: 60, y: 20 }, ["Alt"]); // outside the layer, Alt held
  const d = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
  expect(d.layers.length).toBe(1); // no duplicate: Alt-drag on an unlinked mask moves it instead
  expect(d.layers[0].maskPlacement.origin).not.toEqual(afterNudge.maskPlacement.origin);
  expect(d.layers[0].transform.origin).toEqual(before.transform.origin);
});
