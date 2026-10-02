import { test, expect, type Page } from "@playwright/test";
import { clickMenu, solidPngBase64 } from "./helpers";

// Phase 4a: the selection tools and the edits a selection limits, driven through the real canvas,
// keys, menus and panels (engine behaviour is pinned in engine/tests/selection_*.rs).

type Pt = [number, number];

/** A 64 x 48 document with one opaque layer over it: red on the left 32 columns, blue on the right,
 * shown at 4 CSS px per document pixel. */
async function setup(page: Page) {
  // Pin the viewport where this file reads pixels (the composite, the GPU readback and the
  // overlay canvas's ants) -- see blend.spec.ts's setup for why an unpinned viewport can land
  // the document rect on a rasterisation tie.
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const red = await page.evaluate(solidPngBase64, { width: 32, height: 48, color: "#ff0000" });
  const blue = await page.evaluate(solidPngBase64, { width: 32, height: 48, color: "#0000ff" });
  await page.evaluate(async ([r, b]) => {
    const api = (window as any).__compositor;
    const bytes = (data: string) => Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 48, false);
    api.engine.importImage(doc, bytes(r), "Red", { x: 16, y: 24 });
    api.engine.importImage(doc, bytes(b), "Blue", { x: 48, y: 24 });
    api.engine.execute(doc, { type: "MergeLayers", ids: api.engine.state(doc).layers.map((l: any) => l.id) });
    api.store.getState().openDocument(doc);
    await api.setZoom(4);
  }, [red, blue]);
}
const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
const selection = async (page: Page) => (await state(page)).selection;
const undoDepth = async (page: Page) => (await state(page)).undoDepth;
/** Where document point `p` is on screen. */
const client = (page: Page, p: Pt) => page.evaluate(([x, y]) => {
  const s = (window as any).__compositor.store.getState(); const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
  const v = vp.viewPoint({ x, y }, { width: d.width, height: d.height });
  const r = document.querySelector('[data-testid="canvas-view"]')!.getBoundingClientRect();
  return { x: r.left + v.x, y: r.top + v.y };
}, p);
async function drag(page: Page, from: Pt, to: Pt, steps = 6) {
  const a = await client(page, from), b = await client(page, to);
  await page.mouse.move(a.x, a.y); await page.mouse.down();
  await page.mouse.move(b.x, b.y, { steps }); await page.mouse.up();
}
async function click(page: Page, p: Pt) { const c = await client(page, p); await page.mouse.click(c.x, c.y); }
/** The document's own composite at one pixel, premultiplied RGBA. */
const pixel = (page: Page, p: Pt) => page.evaluate(([x, y]) => {
  const api = (window as any).__compositor; const s = api.store.getState();
  return Array.from(api.engine.composite(s.activeId, { x, y, width: 1, height: 1 }, 1, 1) as Uint8Array);
}, p);
/** The overlay's device pixels along document row `y` from column `x0` to `x1`, as one string. */
const overlayRow = (page: Page, y: number, x0: number, x1: number) => page.evaluate(([y, x0, x1]) => {
  const s = (window as any).__compositor.store.getState(); const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
  const dpr = window.devicePixelRatio || 1;
  const a = vp.viewPoint({ x: x0, y }, { width: d.width, height: d.height }), b = vp.viewPoint({ x: x1, y }, { width: d.width, height: d.height });
  const overlay = document.querySelector('[data-testid="overlay"]') as HTMLCanvasElement;
  const data = overlay.getContext("2d")!.getImageData(Math.round(a.x * dpr), Math.round(a.y * dpr) - 1, Math.round((b.x - a.x) * dpr), 3).data;
  return Array.from(data).join(",");
}, [y, x0, x1]);
const painted = (row: string) => row.split(",").some((v, i) => i % 4 === 3 && Number(v) > 0);

test("a Marquee drag selects a whole-pixel box, the ants march around it, and Ctrl+D deselects", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("m");
  await expect(page.getByTestId("selection-options")).toBeVisible();
  const before = await undoDepth(page);
  await drag(page, [10.3, 12.2], [29.8, 27.6]);
  const s = await selection(page);
  expect(s.bounds).toEqual({ x: 10, y: 12, width: 20, height: 16 });
  expect(await undoDepth(page)).toBe(before + 1);
  // The ants lie on the box's top edge and march: the dash has moved a few ticks later.
  const first = await overlayRow(page, 12, 12, 28);
  expect(painted(first)).toBe(true);
  await page.waitForTimeout(400);
  expect(await overlayRow(page, 12, 12, 28)).not.toBe(first);
  await page.keyboard.press("Control+d");
  expect(await selection(page)).toBeNull();
  expect(painted(await overlayRow(page, 12, 12, 28))).toBe(false);
});

test("Shift adds, Alt subtracts, Tab makes the Marquee an ellipse", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("m");
  await drag(page, [4, 4], [20, 20]);
  await page.keyboard.down("Shift"); await drag(page, [30, 4], [44, 20]); await page.keyboard.up("Shift");
  expect((await selection(page)).bounds).toEqual({ x: 4, y: 4, width: 40, height: 16 });
  await page.keyboard.down("Alt"); await drag(page, [4, 4], [12, 20]); await page.keyboard.up("Alt");
  expect((await selection(page)).bounds).toEqual({ x: 12, y: 4, width: 32, height: 16 });
  await page.keyboard.press("Tab");
  await expect(page.getByTestId("marquee-ellipse")).toHaveAttribute("aria-pressed", "true");
  await drag(page, [10, 10], [50, 40]);
  const b = (await selection(page)).bounds;
  expect(Math.abs(b.x - 10) < 0.5 && Math.abs(b.y - 10) < 0.5 && Math.abs(b.width - 40) < 0.5 && Math.abs(b.height - 30) < 0.5).toBe(true);
});

test("the Polygonal Lasso adds corners, drops one with Backspace, closes on its first corner, and Escape cancels", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("l");
  await page.getByTestId("lasso-polygonal").click();
  const before = await undoDepth(page);
  await click(page, [10, 10]); await click(page, [50, 10]); await click(page, [30, 30]);
  await page.keyboard.press("Backspace");
  await click(page, [50, 40]); await click(page, [10, 40]);
  expect(await selection(page)).toBeNull();
  await click(page, [10.5, 10.5]); // within 8 view px of the first corner
  const s = await selection(page);
  expect(s.bounds).toEqual({ x: 10, y: 10, width: 40, height: 30 });
  expect(await undoDepth(page)).toBe(before + 1);
  await click(page, [5, 5]); await click(page, [20, 5]);
  await page.keyboard.press("Escape");
  expect((await selection(page)).bounds).toEqual({ x: 10, y: 10, width: 40, height: 30 });
  expect(await page.evaluate(() => (window as any).__compositor.store.getState().selectionDraft)).toBeNull();
});

test("dragging inside the selection moves its outline, arrows nudge it, a click deselects", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("m");
  await drag(page, [10, 10], [20, 20]);
  const before = await undoDepth(page);
  await drag(page, [15, 15], [25.4, 19.6]);
  expect((await selection(page)).bounds).toEqual({ x: 20, y: 15, width: 10, height: 10 });
  expect(await undoDepth(page)).toBe(before + 1);
  await page.keyboard.press("ArrowLeft"); await page.keyboard.press("Shift+ArrowDown");
  expect((await selection(page)).bounds).toEqual({ x: 19, y: 25, width: 10, height: 10 });
  await click(page, [24, 30]);
  expect(await selection(page)).toBeNull();
});

test("the Magic Wand selects the clicked colour on this layer, contiguous or everywhere", async ({ page }) => {
  await setup(page);
  // Ruling I7: the clicked (red) colour is otherwise one connected region, so Contiguous on and
  // off would select exactly the same pixels and this test could never tell them apart. A lone
  // 4x4 red island, disconnected from the main red block and placed inside the blue half, is what
  // a wrongly-ignored Contiguous setting pulls in (or a wrongly-honoured one leaves out).
  const island = await page.evaluate(solidPngBase64, { width: 4, height: 4, color: "#ff0000" });
  await page.evaluate(async (b64) => {
    const api = (window as any).__compositor;
    const bytes = Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0));
    const s = api.store.getState();
    api.engine.importImage(s.activeId, bytes, "Island", { x: 50, y: 10 });
    api.engine.execute(s.activeId, { type: "MergeLayers", ids: api.engine.state(s.activeId).layers.map((l: any) => l.id) });
  }, island);
  await page.keyboard.press("w");
  await click(page, [5, 5]);
  expect((await selection(page)).bounds).toEqual({ x: 0, y: 0, width: 32, height: 48 });
  await page.keyboard.press("Control+i");
  expect(await pixel(page, [50, 10])).toEqual([255, 0, 0, 255]); // the island: outside with Contiguous on, untouched
  expect(await pixel(page, [5, 5])).toEqual([0, 255, 255, 255]); // the clicked region: inside, inverted
  await page.keyboard.press("Control+z");
  await page.keyboard.press("Control+d");
  await page.getByTestId("wand-contiguous").uncheck();
  await click(page, [5, 5]);
  expect((await selection(page)).bounds).toEqual({ x: 0, y: 0, width: 52, height: 48 }); // the main block and the island together
  await page.keyboard.press("Control+i");
  expect(await pixel(page, [50, 10])).toEqual([0, 255, 255, 255]); // the island: inside with Contiguous off, inverted too
});

test("Invert, Levels and Delete stay inside the selection; an adjustment layer ignores it", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("m");
  await drag(page, [0, 0], [16, 48]);
  await page.keyboard.press("Control+i");
  expect(await pixel(page, [4, 4])).toEqual([0, 255, 255, 255]);
  expect(await pixel(page, [20, 4])).toEqual([255, 0, 0, 255]);
  // The GPU draws what the CPU composites.
  const gpu = await page.evaluate(async () => {
    const api = (window as any).__compositor; await api.setZoom(1);
    const px = api.readDocumentPixels() as Uint8Array; const s = api.store.getState(); const w = s.documents[s.activeId].width;
    const at = (x: number, y: number) => Array.from(px.slice((y * w + x) * 4, (y * w + x) * 4 + 4));
    const shown = [at(4, 4), at(20, 4)];
    await api.setZoom(4);
    return shown;
  });
  expect(gpu).toEqual([[0, 255, 255, 255], [255, 0, 0, 255]]);
  await page.keyboard.press("Delete");
  expect((await pixel(page, [4, 4]))[3]).toBe(0);
  expect(await pixel(page, [20, 4])).toEqual([255, 0, 0, 255]);
  expect((await state(page)).layers.length).toBe(1);
  // Ruling M9: a test named for Levels has to exercise Levels too, through the real dialog, and
  // show it stays inside a selection like Invert and Delete just did. A fresh selection on the
  // untouched right side of the blue half keeps this independent of the edits above.
  await drag(page, [48, 0], [64, 48]);
  await clickMenu(page, "Image", "image-levels");
  await expect(page.getByTestId("adjust-title")).toHaveText("Levels");
  await page.getByLabel("Output white").fill("128");
  await page.getByLabel("Output white").press("Enter");
  await page.getByTestId("adjust-ok").click();
  // The master range's formula (LevelRange::apply, engine/src/adjust/settings.rs): with the
  // per-channel range left at its identity default, input black 0, gamma 1, input white 255,
  // output black 0, output white 128 -- out = outputBlack + ((v - black) / (white - black))
  // ^(1/gamma) * (outputWhite - outputBlack), applied unpremultiplied then re-premultiplied
  // (levels.rs::apply_tables). Blue's B channel (255, i.e. normalised 1.0) maps to
  // 0 + (255 - 0) / (255 - 0) * (128 - 0) = 128 exactly; R and G (0) stay 0. `pixel()` reads
  // the engine's own composite (api.engine.composite), not the GPU canvas, so this is exact --
  // no GPU/CPU parity tolerance applies here.
  expect(await pixel(page, [56, 4])).toEqual([0, 0, 128, 255]); // inside the new selection: changed
  expect(await pixel(page, [40, 4])).toEqual([0, 0, 255, 255]); // outside it: untouched
  await clickMenu(page, "Layer", "layer-adjustment-invert");
  expect(await pixel(page, [40, 4])).toEqual([255, 255, 0, 255]);
});

test("Add Mask with a selection reveals it (Alt-click hides it) and uses it up; the Crop tool starts at the selection", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("m");
  await drag(page, [8, 6], [24, 30]);
  await page.keyboard.press("c");
  expect(await page.evaluate(() => (window as any).__compositor.store.getState().cropRect)).toEqual({ x: 8, y: 6, width: 16, height: 24 });
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("layer-add-mask")).toHaveAttribute("title", /revealing the selection/);
  await page.getByTestId("layer-add-mask").click();
  const s = await state(page);
  expect(s.selection).toBeNull();
  expect(s.layers[0].hasMask).toBe(true);
  // Reveal Selection (LayerMask.swift:237-267 at v1.4.5): the selection shows, the rest is hidden.
  expect((await pixel(page, [12, 12]))[3]).toBe(255);
  expect((await pixel(page, [40, 40]))[3]).toBe(0);
  await page.keyboard.press("Control+z");
  expect((await state(page)).selection).not.toBeNull();
  await page.getByTestId("layer-add-mask").click({ modifiers: ["Alt"] });
  expect((await state(page)).layers[0].hasMask).toBe(true);
  expect((await pixel(page, [12, 12]))[3]).toBe(0);
  expect((await pixel(page, [40, 40]))[3]).toBe(255);
});

test("Inverse of Select All leaves nothing selected, so Fill reaches the whole layer again; undo brings the selection back", async ({ page }) => {
  // Selection.swift:354-362 at v1.4.5 (Phase 4.5).
  await setup(page);
  await page.keyboard.press("Control+a");
  const depth = await undoDepth(page);
  await page.keyboard.press("Control+Shift+i");
  expect(await selection(page)).toBeNull();
  expect(await undoDepth(page)).toBe(depth + 1);
  // With an empty selection left instead, Fill would be refused; with none it fills everything.
  await page.keyboard.press("Alt+Backspace");
  await expect.poll(() => pixel(page, [60, 40])).toEqual([0, 0, 0, 255]);
  await page.keyboard.press("Control+z");
  await page.keyboard.press("Control+z");
  expect((await selection(page)).bounds).toEqual({ x: 0, y: 0, width: 64, height: 48 });
});

test("the Select menu: All, Inverse, Expand and Feather with their amount, the layer's pixels, and Ctrl-click on a thumbnail", async ({ page }) => {
  await setup(page);
  await clickMenu(page, "Select", "select-all");
  expect((await selection(page)).bounds).toEqual({ x: 0, y: 0, width: 64, height: 48 });
  await page.keyboard.press("m");
  // A New-mode drag inside the selection would move it, so deselect first.
  await page.keyboard.press("Control+d");
  await drag(page, [10, 10], [20, 20]);
  await clickMenu(page, "Select", "select-expand");
  await page.getByTestId("selection-amount").fill("3");
  await page.keyboard.press("Enter");
  expect((await selection(page)).bounds).toEqual({ x: 7, y: 7, width: 16, height: 16 });
  await clickMenu(page, "Select", "select-feather");
  await expect(page.getByRole("button", { name: "OK" })).toBeEnabled();
  await page.getByTestId("selection-amount").fill("251");
  await expect(page.getByRole("button", { name: "OK" })).toBeDisabled();
  await page.getByTestId("selection-amount").fill("4");
  await page.getByRole("button", { name: "OK" }).click();
  expect((await selection(page)).feather).toBe(4);
  await page.keyboard.press("Control+Shift+i");
  expect((await selection(page)).bounds).toEqual({ x: 0, y: 0, width: 64, height: 48 });
  // Ruling M10: that bounds check is exactly what a plain Select All already produces, so on its
  // own it cannot tell "the layer's own opaque pixels" apart from "the whole canvas". Punching a
  // hole along the left edge first gives Layer's Pixels a concrete, fixture-derived shape to load,
  // while the feather assertion below still shows the load replaced the old (feathered) selection.
  await page.keyboard.press("Control+d");
  await drag(page, [0, 0], [8, 48]);
  await page.keyboard.press("Delete");
  const id = (await state(page)).layers[0].id;
  await page.keyboard.down("Control"); await page.getByTestId(`target-pixels-${id}`).click(); await page.keyboard.up("Control");
  expect((await selection(page)).feather).toBe(0);
  expect((await selection(page)).bounds).toEqual({ x: 8, y: 0, width: 56, height: 48 });
});

test("an empty selection says so and refuses the edits", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("m");
  await drag(page, [10, 10], [20, 20]);
  await page.getByTestId("selection-contract-amount").fill("6");
  await page.getByTestId("selection-contract").click();
  await expect(page.getByTestId("selection-empty")).toBeVisible();
  await page.getByRole("button", { name: "Image", exact: true }).click();
  await expect(page.getByTestId("menu-image-levels")).toBeDisabled();
  await expect(page.getByTestId("menu-image-invert")).toBeDisabled();
});

test("a drag that loses its pointerup leaves no outline move behind", async ({ page }) => {
  // Final review F4: after a pointercancel, or a press whose release never came, the next press
  // used to drag the outline and record a wrong Move Selection.
  await setup(page);
  await page.keyboard.press("m");
  await drag(page, [10, 10], [20, 20]);
  const before = await undoDepth(page);
  const outlineMove = () => page.evaluate(() => (window as any).__compositor.store.getState().outlineMove);
  const canvasEvent = (type: string, at: { x: number; y: number }) => page.evaluate(([type, x, y]) => {
    document.querySelector('[data-testid="canvas-view"]')!.dispatchEvent(new PointerEvent(type as string, { pointerId: 1, button: 0, clientX: x as number, clientY: y as number }));
  }, [type, at.x, at.y]);
  // Cancelled mid-drag: the outline stops following, and the release commits nothing.
  const a = await client(page, [15, 15]), b = await client(page, [18, 15]);
  await page.mouse.move(a.x, a.y); await page.mouse.down(); await page.mouse.move(b.x, b.y, { steps: 3 });
  expect(await outlineMove()).not.toBeNull();
  await canvasEvent("pointercancel", b);
  expect(await outlineMove()).toBeNull();
  await page.mouse.up();
  expect(await undoDepth(page)).toBe(before);
  expect((await selection(page)).bounds).toEqual({ x: 10, y: 10, width: 10, height: 10 });
  // A press inside the selection whose release the page never sees: the next drag, outside it,
  // draws a new box instead of dragging the outline.
  await canvasEvent("pointerdown", a);
  expect(await outlineMove()).toEqual({ dx: 0, dy: 0 });
  await drag(page, [40, 30], [50, 40]);
  expect((await selection(page)).bounds).toEqual({ x: 40, y: 30, width: 10, height: 10 });
  expect(await undoDepth(page)).toBe(before + 1);
});

test("the selection options give the keys back once used", async ({ page }) => {
  // Final review F5: a checkbox, a select or a number field kept the focus, so Delete, Ctrl+D, the
  // tool letters and the arrows went to it instead of the canvas.
  await setup(page);
  await page.keyboard.press("m");
  await drag(page, [10, 10], [20, 20]);
  await page.keyboard.press("w");
  await page.getByTestId("wand-contiguous").click();
  await page.keyboard.press("Control+d");
  expect(await selection(page)).toBeNull();
  await page.keyboard.press("m");
  await drag(page, [10, 10], [20, 20]);
  await page.keyboard.press("w");
  const size = page.getByTestId("wand-sample-size");
  await size.focus();
  await size.selectOption("1");
  await page.keyboard.press("ArrowLeft");
  expect((await selection(page)).bounds).toEqual({ x: 9, y: 10, width: 10, height: 10 });
  await expect(size).toHaveValue("1");
  // A number field keeps the focus while typed into and gives it up on Enter.
  await page.getByTestId("wand-tolerance").fill("40");
  await page.getByTestId("wand-tolerance").press("Enter");
  await page.keyboard.press("Control+d");
  expect(await selection(page)).toBeNull();
});

test("Ctrl-click on a folder's thumbnail loads nothing and says nothing", async ({ page }) => {
  // Final review M3: the engine's refusal used to show in the banner (the Mac beeps). The click is
  // still taken: the folder does not become the active layer.
  await setup(page);
  await page.keyboard.press("m");
  await drag(page, [10, 10], [20, 20]);
  await page.getByTestId("layer-add-folder").click();
  const layers = (await state(page)).layers;
  const folder = layers.find((l: any) => l.isGroup).id, pixels = layers.find((l: any) => !l.isGroup).id;
  await page.getByTestId(`target-pixels-${pixels}`).click();
  const before = await state(page);
  expect(before.activeLayerId).toBe(pixels);
  await page.keyboard.down("Control"); await page.getByTestId(`target-pixels-${folder}`).click(); await page.keyboard.up("Control");
  await expect(page.getByTestId("error-banner")).toHaveCount(0);
  const after = await state(page);
  expect([after.activeLayerId, after.undoDepth, after.selection.bounds]).toEqual([pixels, before.undoDepth, { x: 10, y: 10, width: 10, height: 10 }]);
});