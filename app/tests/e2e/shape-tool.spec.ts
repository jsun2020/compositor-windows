import { test, expect, type Page } from "@playwright/test";

// Phase 4b-1: the Shape tool through the real canvas, keys and bar (the engine's shapes are pinned
// in engine/tests/shapes.rs, ported from the Mac's ShapeToolTests).

type Pt = [number, number];

/** The Mac test's session: 100 x 80 with one empty layer, red foreground, at 4 CSS px a pixel. */
async function setup(page: Page) {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.evaluate(async () => {
    const api = (window as any).__compositor;
    api.store.getState().openDocument(api.engine.newDocument(100, 80, true));
    await api.setZoom(4);
    api.store.getState().setPaletteColor({ red: 1, green: 0, blue: 0 }, false);
  });
}
const client = (page: Page, p: Pt) => page.evaluate(([x, y]) => {
  const s = (window as any).__compositor.store.getState(); const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
  const v = vp.viewPoint({ x, y }, { width: d.width, height: d.height });
  const r = document.querySelector('[data-testid="canvas-view"]')!.getBoundingClientRect();
  return { x: r.left + v.x, y: r.top + v.y };
}, p);
const doc = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
const pixel = (page: Page, x: number, y: number) => page.evaluate(([x, y]) => {
  const api = (window as any).__compositor; const s = api.store.getState();
  const p = Array.from(api.engine.composite(s.activeId, { x, y, width: 1, height: 1 }, 1, 1) as Uint8Array);
  return [p[0], p[3]];
}, [x, y]);
/** The overlay's colour at document point `p` (straight RGBA). */
const overlayAt = (page: Page, p: Pt) => page.evaluate(([x, y]) => {
  const s = (window as any).__compositor.store.getState(); const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
  const v = vp.viewPoint({ x, y }, { width: d.width, height: d.height }); const dpr = window.devicePixelRatio || 1;
  const overlay = document.querySelector('[data-testid="overlay"]') as HTMLCanvasElement;
  return Array.from(overlay.getContext("2d")!.getImageData(Math.round(v.x * dpr), Math.round(v.y * dpr), 1, 1).data);
}, p);

test("U and a drag make a rectangle on a new layer in the foreground colour, one undo step, keeping the selection", async ({ page }) => {
  await setup(page);
  await page.evaluate(() => (window as any).__compositor.store.getState().run({ type: "SelectAll" }));
  const depth = (await doc(page)).undoDepth;
  await page.keyboard.press("u");
  await expect(page.getByTestId("shape-options")).toBeVisible();
  const a = await client(page, [10, 10]), b = await client(page, [40, 30]), mid = await client(page, [30, 25]);
  await page.mouse.move(a.x, a.y); await page.mouse.down();
  await page.mouse.move(mid.x, mid.y, { steps: 3 });
  // While dragging, the shape shows on the overlay in red, and nothing is in the document yet.
  expect(await overlayAt(page, [20, 20])).toEqual([255, 0, 0, 255]);
  expect((await doc(page)).layers.length).toBe(1);
  await page.mouse.move(b.x, b.y, { steps: 3 }); await page.mouse.up();
  const d = await doc(page);
  expect(d.layers.map((l: any) => l.name)).toEqual(["Layer 1", "Rectangle 1"]);
  const layer = d.layers[1];
  expect([d.activeLayerId, layer.transform.origin, layer.transform.size]).toEqual([layer.id, [10, 10], [30, 20]]);
  expect(d.undoDepth).toBe(depth + 1);
  expect(d.selection, "unlike Paste, drawing a shape keeps the selection").not.toBeNull();
  for (const [x, y] of [[25, 20], [10, 10], [39, 29]]) expect(await pixel(page, x, y)).toEqual([255, 255]);
  for (const [x, y] of [[9, 20], [40, 20], [25, 30]]) expect((await pixel(page, x, y))[1]).toBe(0);
  expect(await overlayAt(page, [20, 20]), "the draft is gone from the overlay").toEqual([0, 0, 0, 0]);
  await page.keyboard.press("Control+z");
  expect((await doc(page)).layers.length).toBe(1);
});

test("Shift+U steps to the Ellipse; Shift and Alt make a circle from its centre; a click and Escape make nothing", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("u");
  await page.keyboard.press("Shift+U");
  await expect(page.getByTestId("shape-ellipse")).toHaveAttribute("aria-pressed", "true");
  const depth = (await doc(page)).undoDepth;
  // A click.
  const c = await client(page, [20, 20]);
  await page.mouse.click(c.x, c.y);
  // Escape mid-drag.
  await page.mouse.move(c.x, c.y); await page.mouse.down();
  const far = await client(page, [50, 50]);
  await page.mouse.move(far.x, far.y, { steps: 3 });
  await page.keyboard.press("Escape");
  await page.mouse.up();
  expect((await doc(page)).undoDepth, "nothing made").toBe(depth);
  // (50, 40) to (60, 45) with Shift and Alt: the box (40, 30, 20, 20).
  const a = await client(page, [50, 40]), b = await client(page, [60, 45]);
  await page.mouse.move(a.x, a.y); await page.mouse.down();
  await page.keyboard.down("Shift"); await page.keyboard.down("Alt");
  await page.mouse.move(b.x, b.y, { steps: 3 }); await page.mouse.up();
  await page.keyboard.up("Alt"); await page.keyboard.up("Shift");
  const layer = (await doc(page)).layers[1];
  expect([layer.name, layer.transform.origin, layer.transform.size]).toEqual(["Ellipse 1", [40, 30], [20, 20]]);
  expect(await pixel(page, 50, 40)).toEqual([255, 255]);
  expect((await pixel(page, 41, 40))[1]).toBeGreaterThan(0);
  expect((await pixel(page, 40, 30))[1]).toBe(0);
});

test("a line takes the bar's width and lands between the points it was dragged between", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("u");
  await page.getByTestId("shape-line").click();
  await page.getByTestId("shape-width").fill("6");
  await page.getByTestId("shape-width").press("Enter");
  const a = await client(page, [20, 30]), b = await client(page, [70, 30]);
  await page.mouse.move(a.x, a.y); await page.mouse.down();
  await page.mouse.move(b.x, b.y, { steps: 4 }); await page.mouse.up();
  const layer = (await doc(page)).layers[1];
  // (20, 30) to (70, 30) grown by 3 on each side: 56 x 6 at (17, 27).
  expect([layer.name, layer.transform.origin, layer.transform.size]).toEqual(["Line 1", [17, 27], [56, 6]]);
  expect(await pixel(page, 45, 29)).toEqual([255, 255]);
  expect((await pixel(page, 45, 33))[1]).toBe(0);
});
