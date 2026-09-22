import { test, expect, type Page } from "@playwright/test";
import { noisePngBase64 } from "./helpers";
import { curveValue } from "../../src/tools/curves-editor";

async function setup(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(noisePngBase64);
  await page.evaluate(async (data) => {
    const api = (window as any).__compositor;
    const png = Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 64, false);
    api.engine.importImage(doc, png, "noise", { x: 32, y: 32 });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
  }, b64);
}
const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
const open = (page: Page, kind: string) => page.evaluate((kind) => (window as any).__compositor.store.getState().beginAdjust({ kind }), kind);

/**
 * Deliberately confined to a middle band (60..179), never touching 0 or 255: `noisePngBase64`
 * (used by `setup` below) is a uniform 0..255 spread, and with 4096 pixels its bin 0 and bin 255
 * alone already clear the auto-levels 0.1% clip threshold, so "Contrast" on it computes an
 * identity range and could never demonstrate a stretch. This fixture's extremes sit well inside
 * 0..255, so Auto Contrast has an actual gap to close.
 */
async function narrowRangeNoisePngBase64(): Promise<string> {
  const canvas = document.createElement("canvas");
  canvas.width = 64;
  canvas.height = 64;
  const ctx = canvas.getContext("2d")!;
  const image = ctx.createImageData(64, 64);
  let seed = 12345;
  const next = () => { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return (seed >> 16) & 0xff; };
  for (let i = 0; i < 64 * 64; i++) {
    image.data[i * 4] = 60 + (next() % 120);
    image.data[i * 4 + 1] = 60 + (next() % 120);
    image.data[i * 4 + 2] = 60 + (next() % 120);
    image.data[i * 4 + 3] = 255;
  }
  ctx.putImageData(image, 0, 0);
  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("toBlob failed"))), "image/png");
  });
  const bytes = new Uint8Array(await blob.arrayBuffer());
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary);
}

async function setupWith(page: Page, generate: () => Promise<string>) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(generate);
  await page.evaluate(async (data) => {
    const api = (window as any).__compositor;
    const png = Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 64, false);
    api.engine.importImage(doc, png, "noise", { x: 32, y: 32 });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
  }, b64);
}

test("the levels panel previews, applies once, and leaves the pixels alone until OK", async ({ page }) => {
  await setup(page);
  const before = (await state(page)).layers[0].pixelsRevision;
  // `setup` imports a noise layer into an existing document, which is itself an undoable
  // command: `canUndo` starts true and `undoDepth` starts above zero before any panel opens.
  // "recorded nothing" has to mean "the depth did not move from here", not "canUndo is false".
  const depthBefore = (await state(page)).undoDepth;
  const sampleAt = { x: 32, y: 32 };
  // `sampleColor` reads the composite through `render_document`, which substitutes the preview
  // (unlike `sampleLayerColor`, which the histogram/eyedroppers deliberately read straight off
  // the stored document so they never chase the preview they are about to change) -- so this is
  // the same read a screenshot comparison would be, without one.
  const sample = () => page.evaluate((at) => {
    const api = (window as any).__compositor;
    return api.engine.sampleColor(api.store.getState().activeId, at);
  }, sampleAt);
  const original = await sample();
  await open(page, "Levels");
  await expect(page.getByTestId("adjust-panel")).toBeVisible();
  await expect(page.getByTestId("adjust-title")).toHaveText("Levels");
  await expect(page.getByTestId("levels-histogram")).toBeVisible();
  await page.getByLabel("White point").fill("128");
  await page.getByLabel("White point").press("Enter");
  // The preview is on screen, but the document has recorded nothing.
  let d = await state(page);
  expect(d.undoDepth).toBe(depthBefore);
  expect(await sample()).not.toEqual(original);   // the preview is genuinely showing something
  // Preview off shows the original again -- tested, not just claimed in a comment.
  await page.getByTestId("adjust-preview").uncheck();
  expect(await sample()).toEqual(original);
  await page.getByTestId("adjust-preview").check();
  expect(await sample()).not.toEqual(original);
  await page.getByTestId("adjust-ok").click();
  d = await state(page);
  expect(d.canUndo).toBe(true);
  expect(d.undoDepth).toBe(depthBefore + 1);   // applied exactly once
  expect(d.layers[0].pixelsRevision).toBeGreaterThan(before);
  await expect(page.getByTestId("adjust-panel")).toHaveCount(0);
  await page.keyboard.press("Control+z");
  expect((await state(page)).undoDepth).toBe(depthBefore);
});

test("cancel and escape record nothing", async ({ page }) => {
  await setup(page);
  const depthBefore = (await state(page)).undoDepth;
  await open(page, "Levels");
  await page.getByLabel("Gamma").fill("2");
  await page.getByLabel("Gamma").press("Enter");
  await page.getByTestId("adjust-cancel").click();
  expect((await state(page)).undoDepth).toBe(depthBefore);
  await open(page, "Curves");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("adjust-panel")).toHaveCount(0);
  expect((await state(page)).undoDepth).toBe(depthBefore);
});

test("auto levels and the reset button", async ({ page }) => {
  await setupWith(page, narrowRangeNoisePngBase64);
  const depthBefore = (await state(page)).undoDepth;
  await open(page, "Levels");
  await page.getByTestId("levels-auto").selectOption("Contrast");
  const stretched = await page.evaluate(() => (window as any).__compositor.store.getState().adjustEdit.adjustment.levels.ranges[0]);
  expect(stretched.black > 0 || stretched.white < 255).toBe(true);
  await page.getByTestId("adjust-reset").click();
  const reset = await page.evaluate(() => (window as any).__compositor.store.getState().adjustEdit.adjustment.levels.ranges[0]);
  expect(reset).toEqual({ black: 0, gamma: 1, white: 255, outputBlack: 0, outputWhite: 255 });
  await page.getByTestId("adjust-ok").click();
  expect((await state(page)).undoDepth).toBe(depthBefore);   // an identity adjustment records nothing
});

test("the curves editor adds a point by clicking and applies it", async ({ page }) => {
  await setup(page);
  // A boolean `canUndo` can't tell "this test's Curves edit was recorded" from "was already true
  // from setup's import into an existing document" -- undoDepth proves a step was actually
  // pushed. That alone only proves *a* step was pushed, not the RIGHT one, so this also samples
  // one pixel before and after OK and checks the committed colour against the exact curve drawn
  // (via `curveValue`, the same Hermite port the engine uses, already unit-tested against it in
  // curves-editor.test.ts) -- proving the committed pixels are what this curve produces, not just
  // "some" pixels changed.
  const depthBefore = (await state(page)).undoDepth;
  const layerId = (await state(page)).layers[0].id;
  const sampleAt = { x: 32, y: 32 };
  const original = await page.evaluate(({ id, at }) => {
    const api = (window as any).__compositor;
    return api.engine.sampleLayerColor(api.store.getState().activeId, id, at);
  }, { id: layerId, at: sampleAt });
  await open(page, "Curves");
  const editor = page.getByTestId("curves-editor");
  const box = (await editor.boundingBox())!;
  await page.mouse.click(box.x + box.width * 0.5, box.y + box.height * 0.25);
  const points = await page.evaluate(() => (window as any).__compositor.store.getState().adjustEdit.adjustment.curves.channels[0]);
  expect(points.length).toBe(3);
  expect(points[1].y).toBeGreaterThan(points[1].x);   // clicking above the diagonal brightens
  await page.getByTestId("adjust-ok").click();
  const d = await state(page);
  expect(d.undoDepth).toBe(depthBefore + 1);   // exactly one step recorded
  expect(d.canUndo).toBe(true);
  const committed = await page.evaluate(({ id, at }) => {
    const api = (window as any).__compositor;
    return api.engine.sampleLayerColor(api.store.getState().activeId, id, at);
  }, { id: layerId, at: sampleAt });
  // Only the RGB channel was drawn on (channels[0]); the per-channel R/G/B curves stay identity,
  // so the engine's `curves_tables` (composite then per-channel, both curves.rs and this ported
  // `curveValue`) reduces to applying the drawn points straight to each original byte.
  for (let c = 0; c < 3; c++) {
    const originalByte = Math.round(original[c] * 255);
    const expectedByte = Math.round(curveValue(points, originalByte));
    expect(Math.round(committed[c] * 255)).toBe(expectedByte);
  }
});

test("switching curves channel does not strand the point fields on a stale index", async ({ page }) => {
  // Regression for a reachable crash: `selected` pointed at index 2 of a 4-point RGB channel;
  // switching to a channel with only the default 2 points left it pointing past the end, and the
  // Input/Output fields' onChange handlers indexed `points[selected]` with no guard.
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await setup(page);
  await open(page, "Curves");
  const editor = page.getByTestId("curves-editor");
  const box = (await editor.boundingBox())!;
  await page.mouse.click(box.x + box.width * 0.3, box.y + box.height * 0.7);
  await page.mouse.click(box.x + box.width * 0.7, box.y + box.height * 0.3);
  const rgbPoints = await page.evaluate(() => (window as any).__compositor.store.getState().adjustEdit.adjustment.curves.channels[0]);
  expect(rgbPoints.length).toBe(4);
  await page.getByTestId("curves-channel").selectOption("Red");
  await page.getByTestId("curves-point-x").fill("50");
  await page.getByTestId("curves-point-y").fill("60");
  expect(errors).toEqual([]);
  await expect(page.getByTestId("adjust-panel")).toBeVisible();
});

test("clicking empty canvas space at the 32-point cap does not leave a stale selection to drag", async ({ page }) => {
  // Regression for a reachable crash: an insert refused by the 32-point cap left `nearestPoint`
  // and `insertPoint`'s combined result unable to identify a selected point, so `selected` became
  // -1; the following drag called `movePoint(points, -1, ...)`, which threw.
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await setup(page);
  await open(page, "Curves");
  await page.evaluate(() => {
    const api = (window as any).__compositor;
    const edit = api.store.getState().adjustEdit;
    // 32 points: the two endpoints plus a tight cluster far from x=50, so a click near x=50 is
    // more than 8 units (the hit tolerance) from every point, and the cap forces the insert to
    // be refused rather than merely finding "a point already there".
    const clustered = [
      { x: 0, y: 0 },
      ...Array.from({ length: 30 }, (_, i) => ({ x: 100 + i, y: 100 })),
      { x: 255, y: 255 },
    ];
    const curves = { ...edit.adjustment.curves, channels: edit.adjustment.curves.channels.map((c: any, i: number) => (i === 0 ? clustered : c)) };
    api.store.getState().updateAdjust({ adjustment: { ...edit.adjustment, curves } });
  });
  const editor = page.getByTestId("curves-editor");
  const box = (await editor.boundingBox())!;
  const emptyX = box.x + box.width * (50 / 255);
  const y = box.y + box.height * 0.5;
  await page.mouse.move(emptyX, y);
  await page.mouse.down();
  await page.mouse.move(emptyX + 10, y - 10);
  await page.mouse.up();
  expect(errors).toEqual([]);
  await expect(page.getByTestId("adjust-panel")).toBeVisible();
});

test("Enter on a dropdown does not commit the panel (the field owns its own Enter)", async ({ page }) => {
  // Regression: the Enter handler excluded <input> but not <select>; Levels and Curves both have
  // a Channel dropdown (and Levels has Auto), so focusing one and pressing Enter closed the panel.
  await setup(page);
  await open(page, "Levels");
  await page.getByTestId("levels-channel").focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("adjust-panel")).toBeVisible();
  await page.getByTestId("levels-auto").focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("adjust-panel")).toBeVisible();
});

test("a panel owns the document while it is open", async ({ page }) => {
  await setup(page);
  await open(page, "Levels");
  await page.getByTestId("layer-add").click();
  expect((await state(page)).layers.length).toBe(1);
  await expect(page.getByTestId("error-banner")).toContainText("Apply or cancel");
  await page.getByTestId("adjust-cancel").click();
  await page.getByTestId("layer-add").click();
  expect((await state(page)).layers.length).toBe(2);
});
