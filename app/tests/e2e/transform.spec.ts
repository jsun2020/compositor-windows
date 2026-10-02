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

/**
 * Keyboard shortcuts stay live while the pointer is captured, so a bare opacity digit can
 * commit the drag from under the pointer handler. The drag must then end: the session is
 * dropped and the snap guides cleared, instead of guides being recomputed and redrawn for a
 * gesture that is already over.
 */
test("a command that commits mid-drag ends the drag and clears the snap guides", async ({ page }) => {
  await setup(page);
  const guides = () => page.evaluate(() => (window as any).__compositor.store.getState().snapGuides);
  const a = await viewPoint(page, 200, 150);
  // Drag towards the canvas centre, which is where a guide appears.
  await page.mouse.move(a.x, a.y);
  await page.mouse.down();
  await page.mouse.move(a.x - 100, a.y);
  await page.mouse.move(a.x - 168, a.y);
  expect(await page.evaluate(() => !!(window as any).__compositor.store.getState().transformEdit)).toBe(true);
  const during = await guides();
  expect(during.xs.length + during.ys.length).toBeGreaterThan(0);
  // The digit commits the pending transform through `run`.
  await page.keyboard.press("5");
  expect(await page.evaluate(() => !!(window as any).__compositor.store.getState().transformEdit)).toBe(false);
  // Moving again must not resurrect the guides.
  await page.mouse.move(a.x - 160, a.y + 20);
  await page.mouse.move(a.x - 150, a.y + 40);
  expect(await guides()).toEqual({ xs: [], ys: [] });
  await page.mouse.up();
  expect(await guides()).toEqual({ xs: [], ys: [] });
  expect((await layer(page)).opacity).toBeCloseTo(0.5, 5);
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
  const revision = (await layer(page)).pixelsRevision;
  await drag(page, { x: 250, y: 100 }, { x: 300, y: 100 }, ["Control"]); // top-right corner pulled right
  expect(await page.evaluate(() => (window as any).__compositor.store.getState().transformEdit?.corners?.[1])).toEqual([300, 100]);
  await expect(page.getByTestId("transform-apply")).toBeVisible();
  await page.keyboard.press("Enter");
  let l = await layer(page);
  expect(l.transform.rotation).toBe(0);
  expect(l.pixelsRevision, "the distortion redraws the pixels").toBeGreaterThan(revision);
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

test("a resize handle snaps the edge it drags to another layer's edge, as a move snaps (Crop.swift:139-182 at v1.4.5)", async ({ page }) => {
  await setup(page);
  const b64 = await page.evaluate(redSquarePngBase64);
  // A second layer whose left edge, x 300, is the target; the red layer's right edge starts at 250.
  await page.evaluate(async (b64) => {
    const api = (window as any).__compositor; const s = api.store.getState(); const doc = s.activeId;
    const red = api.engine.state(doc).activeLayerId;
    api.engine.importImage(doc, Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0)), "target", { x: 325, y: 45 });
    const id = api.engine.state(doc).activeLayerId; const t = api.engine.state(doc).layers.find((l: any) => l.id === id).transform;
    api.engine.execute(doc, { type: "SetLayerTransform", id, transform: { ...t, origin: [300, 20], size: [50, 50] } });
    api.engine.execute(doc, { type: "SetActiveLayer", id: red });
    s.refresh(); s.selectLayers([red], red);
  }, b64);
  // The right-middle handle dragged to x 294, 6 short of 300 and inside the 10 px reach: the edge lands on 300.
  const a = await viewPoint(page, 250, 150), b = await viewPoint(page, 294, 150);
  await page.mouse.move(a.x, a.y); await page.mouse.down(); await page.mouse.move((a.x + b.x) / 2, a.y); await page.mouse.move(b.x, b.y);
  expect((await page.evaluate(() => (window as any).__compositor.store.getState().snapGuides)).xs).toEqual([300]);
  await page.mouse.up();
  const l = (await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].layers; })).find((x: any) => x.name === "red");
  expect(l.transform.origin[0] + l.transform.size[0]).toBe(300);
  // The aspect lock is on (the Mac's default): the height follows the width, about the left-middle handle.
  expect(l.transform.size).toEqual([150, 150]);
  expect(l.transform.origin).toEqual([150, 75]);
  // Out of reach (x 280, 20 short) it does not snap.
  const c = await viewPoint(page, 300, 150), d = await viewPoint(page, 280, 150);
  await page.mouse.move(c.x, c.y); await page.mouse.down(); await page.mouse.move((c.x + d.x) / 2, c.y); await page.mouse.move(d.x, d.y); await page.mouse.up();
  const m = (await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].layers; })).find((x: any) => x.name === "red");
  expect(m.transform.origin[0] + m.transform.size[0]).toBe(280);
});

test("the Move bar's W and H resize from the origin, and the lock keeps the ratio there and on a handle (TransformInspector.swift:24-29 at v1.4.5)", async ({ page }) => {
  await setup(page);
  const lock = page.getByTestId("transform-lock");
  await expect(lock).toHaveAttribute("aria-pressed", "true");
  await page.getByLabel("W", { exact: true }).fill("200");
  await page.keyboard.press("Enter");
  let l = await layer(page);
  expect(l.transform.size).toEqual([200, 200]);
  expect(l.transform.origin).toEqual([150, 100]);
  await lock.click();
  await expect(lock).toHaveAttribute("aria-pressed", "false");
  await page.getByLabel("H", { exact: true }).fill("50");
  await page.keyboard.press("Enter");
  l = await layer(page);
  expect(l.transform.size).toEqual([200, 50]);
  // Unlocked, a corner handle changes each side on its own: the bottom-right corner, at (350, 150), dragged
  // to (370, 190) makes the layer 220 x 90.
  await drag(page, { x: 350, y: 150 }, { x: 370, y: 190 });
  expect((await layer(page)).transform.size).toEqual([220, 90]);
  // Shift turns the lock the other way while held, and the button shows it turned.
  await page.keyboard.down("Shift");
  await expect(lock).toHaveAttribute("aria-pressed", "true");
  await page.keyboard.up("Shift");
  await expect(lock).toHaveAttribute("aria-pressed", "false");
});
