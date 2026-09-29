import { test, expect, type Page } from "@playwright/test";
import { solidPngBase64 } from "./helpers";

// Phase 4b-1: the Gradient tool through the real canvas and keys (the engine's painting is pinned in
// engine/tests/raster_edits.rs and its previews in gradient_preview.rs).

type Pt = [number, number];

/** A 64 x 48 grey layer over its whole canvas, at 4 CSS px a pixel; red foreground, blue background. */
async function setup(page: Page) {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const grey = await page.evaluate(solidPngBase64, { width: 64, height: 48, color: "#808080" });
  await page.evaluate(async (g) => {
    const api = (window as any).__compositor;
    const bytes = (data: string) => Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 48, false);
    api.engine.importImage(doc, bytes(g), "Grey", { x: 32, y: 24 });
    api.store.getState().openDocument(doc);
    await api.setZoom(4);
    const s = api.store.getState();
    s.setPaletteColor({ red: 1, green: 0, blue: 0 }, false);
    s.setPaletteColor({ red: 0, green: 0, blue: 1 }, true);
  }, grey);
}
const client = (page: Page, p: Pt) => page.evaluate(([x, y]) => {
  const s = (window as any).__compositor.store.getState(); const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
  const v = vp.viewPoint({ x, y }, { width: d.width, height: d.height });
  const r = document.querySelector('[data-testid="canvas-view"]')!.getBoundingClientRect();
  return { x: r.left + v.x, y: r.top + v.y };
}, p);
async function drag(page: Page, from: Pt, to: Pt, shift = false) {
  const a = await client(page, from), b = await client(page, to);
  await page.mouse.move(a.x, a.y); await page.mouse.down();
  if (shift) await page.keyboard.down("Shift");
  await page.mouse.move(b.x, b.y, { steps: 6 }); await page.mouse.up();
  if (shift) await page.keyboard.up("Shift");
}
const store = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return { edit: s.gradientEdit, options: s.gradientOptions, depth: s.documents[s.activeId].undoDepth, fg: s.palette.foreground }; });
const pixel = (page: Page, x: number, y: number) => page.evaluate(([x, y]) => {
  const api = (window as any).__compositor; const s = api.store.getState();
  return Array.from(api.engine.composite(s.activeId, { x, y, width: 1, height: 1 }, 1, 1) as Uint8Array);
}, [x, y]);
/** What a linear red-to-transparent gradient from `start` to `end` leaves over opaque grey 128 at
 * pixel (x, y)'s centre: straight red at alpha 1 - t, source-over, each channel rounded half up. */
function expected(start: { x: number; y: number }, end: { x: number; y: number }, x: number, y: number): number[] {
  const dx = end.x - start.x, dy = end.y - start.y;
  const t = Math.min(1, Math.max(0, ((x + 0.5 - start.x) * dx + (y + 0.5 - start.y) * dy) / (dx * dx + dy * dy)));
  const s = 1 - t;
  return [Math.floor(255 * s + 128 * (1 - s) + 0.5), Math.floor(128 * (1 - s) + 0.5), Math.floor(128 * (1 - s) + 0.5), 255];
}

test("a dragged line previews without recording; Return applies it as the preview showed, as one undo step", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("g");
  await expect(page.getByTestId("gradient-options")).toBeVisible();
  const before = await store(page);
  await drag(page, [8, 20], [56, 28]);
  const pending = await store(page);
  expect(pending.edit).not.toBeNull();
  expect(pending.depth, "a pending gradient records nothing").toBe(before.depth);
  const { start, end } = pending.edit;
  const shown = [];
  for (const [x, y] of [[4, 4], [30, 24], [40, 13], [60, 44]]) {
    const p = await pixel(page, x, y);
    expect(p, `(${x}, ${y})`).toEqual(expected(start, end, x, y));
    shown.push(p);
  }
  await expect(page.getByTestId("gradient-apply")).toBeVisible();
  await page.keyboard.press("Enter");
  const applied = await store(page);
  expect([applied.edit, applied.depth]).toEqual([null, before.depth + 1]);
  let i = 0;
  for (const [x, y] of [[4, 4], [30, 24], [40, 13], [60, 44]]) expect(await pixel(page, x, y)).toEqual(shown[i++]);
});

test("the ends drag as handles, Shift holds 45 degrees; Escape drops it and the first Undo discards it", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("g");
  await drag(page, [10, 24], [40, 24]);
  const first = (await store(page)).edit;
  // Grab the end (within 10 view px of it) and move it; the start stays.
  await drag(page, [40.5, 24.5], [50, 10]);
  const moved = (await store(page)).edit;
  expect(moved.start).toEqual(first.start);
  expect(Math.abs(moved.end.x - 50) < 0.3 && Math.abs(moved.end.y - 10) < 0.3).toBe(true);
  // Grab the start (within 10 view px of it) and move it; the end stays (fix round 1, M-4).
  await drag(page, [10.5, 24.5], [5, 15]);
  const movedStart = (await store(page)).edit;
  expect(movedStart.end).toEqual(moved.end);
  expect(Math.abs(movedStart.start.x - 5) < 0.3 && Math.abs(movedStart.start.y - 15) < 0.3).toBe(true);
  await drag(page, [5, 15], [10, 24]); // put the start back for the Shift step below
  // Shift: the end goes onto the nearest eighth of a turn about the start, at the same distance.
  await drag(page, [50, 10], [30, 5], true);
  const snapped = (await store(page)).edit;
  const dx = snapped.end.x - snapped.start.x, dy = snapped.end.y - snapped.start.y;
  expect(Math.abs(Math.abs(dx) - Math.abs(dy)) < 1e-9 || Math.abs(dx) < 1e-9 || Math.abs(dy) < 1e-9, `${dx}, ${dy}`).toBe(true);
  const grey = [128, 128, 128, 255];
  await page.keyboard.press("Escape");
  expect((await store(page)).edit).toBeNull();
  expect(await pixel(page, 30, 24)).toEqual(grey);
  const depth = (await store(page)).depth;
  await drag(page, [10, 24], [40, 24]);
  await page.keyboard.press("Control+z");
  expect((await store(page)).edit).toBeNull();
  expect((await store(page)).depth, "the first Undo only discarded the gradient").toBe(depth);
  expect(await pixel(page, 30, 24)).toEqual(grey);
});

test("Tab switches Radial, the digits set the opacity, and Alt-click samples the foreground", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("g");
  await page.keyboard.press("Tab");
  await expect(page.getByTestId("gradient-radial")).toHaveAttribute("aria-pressed", "true");
  await page.keyboard.press("5");
  await expect(page.getByTestId("gradient-opacity")).toHaveValue("50");
  const at = await client(page, [20, 20]);
  await page.keyboard.down("Alt");
  await page.mouse.click(at.x, at.y);
  await page.keyboard.up("Alt");
  const s = await store(page);
  expect(s.fg).toEqual({ red: 128 / 255, green: 128 / 255, blue: 128 / 255 });
  expect(s.edit, "Alt-click starts no line").toBeNull();
  // A radial red gradient at half opacity: at pixel (32, 24)'s centre, 0.707 px from the start and
  // 20 px to the rim, red at (1 - 0.707 / 20) x 0.5 over grey.
  await page.evaluate(() => (window as any).__compositor.store.getState().setPaletteColor({ red: 1, green: 0, blue: 0 }, false));
  await drag(page, [32, 24], [32, 44]);
  const edit = (await store(page)).edit;
  expect([edit.start, edit.end]).toEqual([{ x: 32, y: 24 }, { x: 32, y: 44 }]);
  await page.keyboard.press("Enter");
  const s2 = (1 - Math.SQRT1_2 / 20) * 0.5;
  expect(await pixel(page, 32, 24)).toEqual([Math.floor(255 * s2 + 128 * (1 - s2) + 0.5), Math.floor(128 * (1 - s2) + 0.5), Math.floor(128 * (1 - s2) + 0.5), 255]);
});

test("with a mask targeted the gradient paints the mask in black and white, as Gradient Mask", async ({ page }) => {
  await setup(page);
  await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); s.run({ type: "AddMask", id: s.documents[s.activeId].activeLayerId, revealing: true }); s.setMaskSelected(true); });
  await page.keyboard.press("g");
  const depth = (await store(page)).depth;
  await drag(page, [0, 24], [64, 24]);
  await page.keyboard.press("Enter");
  expect((await store(page)).depth).toBe(depth + 1);
  // Black (hide) at the left fading to nothing: the layer shows more to the right. The mask starts
  // white (255, `AddMask revealing: true`); source-over black (grey 0) at alpha 1 - t leaves
  // floor(255 * t + 0.5) (fix round 1, M-5: the exact formula, not a loose bound). A fully opaque
  // grey layer's composited alpha equals the mask's value directly.
  const left = await pixel(page, 2, 24), right = await pixel(page, 60, 24);
  expect(left[3]).toBe(10);
  expect(right[3]).toBe(241);
});
