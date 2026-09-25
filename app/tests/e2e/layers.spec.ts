import { test, expect, type Page } from "@playwright/test";
import { clickMenu, redSquarePngBase64 } from "./helpers";

async function fresh(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await clickMenu(page, "File", "new");
  await page.getByRole("button", { name: "Create" }).click();
}

/**
 * Like `fresh`, but the base layer has real pixel content (the "New Canvas" sheet's
 * layer never does - it is created with emptyLayer: true, i.e. no pixel buffer at all -
 * so per-layer geometry commands such as FlipLayers, which skip any layer with no
 * pixels, would have nothing to act on). Follows the same engine.newDocument(w, h,
 * false) + importImage + openDocument pattern already used by blend.spec.ts and
 * transform.spec.ts to get a document with actual pixels without going through the UI.
 */
async function freshWithPixels(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(redSquarePngBase64);
  await page.evaluate((b64) => {
    const api = (window as any).__compositor;
    const png = Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(8, 8, false);
    api.engine.importImage(doc, png, "Layer 1", null);
    api.store.getState().openDocument(doc);
  }, b64);
}
const names = (page: Page) => page.getByTestId("layer-row").allInnerTexts();
const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });

test("multi-select, group, collapse, rename and delete", async ({ page }) => {
  await fresh(page);
  await page.getByTestId("layer-add").click();
  await page.getByTestId("layer-add").click();
  expect(await names(page)).toEqual(["Layer 3", "Layer 2", "Layer 1"]);
  await page.getByTestId("layer-row").nth(2).click();
  await page.getByTestId("layer-row").nth(0).click({ modifiers: ["Shift"] });
  const selected = await page.evaluate(() => (window as any).__compositor.store.getState().selectedLayerIds.length);
  expect(selected).toBe(3);
  await page.getByTestId("layer-row").nth(1).click({ modifiers: ["Control"] });
  expect(await page.evaluate(() => (window as any).__compositor.store.getState().selectedLayerIds.length)).toBe(2);
  await page.getByTestId("layer-row").nth(0).click({ button: "right" });
  await page.getByTestId("ctx-group").click();
  expect((await names(page))[0]).toContain("Folder 1");
  const d = await state(page);
  const folder = d.layers.find((l: any) => l.isGroup);
  expect(d.layers.filter((l: any) => l.parentId === folder.id).length).toBe(2);
  await page.getByTestId(`collapse-${folder.id}`).click();
  expect((await names(page)).length).toBe(2);
  await page.getByTestId(`collapse-${folder.id}`).click();
  expect((await names(page)).length).toBe(4);
  await page.getByTestId("layer-row").nth(3).click();
  await page.getByTestId("layer-delete").click();
  expect((await names(page)).length).toBe(3);
});

test("drag reorders and nests, alt-drag copies", async ({ page }) => {
  await fresh(page);
  await page.getByTestId("layer-add").click();
  await page.getByTestId("layer-add-folder").click();
  // Rows: Folder 1, Layer 2, Layer 1. Drag Layer 1 into the folder.
  const rows = page.getByTestId("layer-row");
  await rows.nth(2).dragTo(rows.nth(0), { targetPosition: { x: 60, y: 14 } });
  let d = await state(page);
  const folder = d.layers.find((l: any) => l.isGroup);
  expect(d.layers.find((l: any) => l.name === "Layer 1").parentId).toBe(folder.id);
  // Drag Layer 2 above Layer 1 inside the folder (upper quarter of the row).
  const target = page.locator('[data-testid="layer-row"]', { hasText: "Layer 1" });
  await page.locator('[data-testid="layer-row"]', { hasText: "Layer 2" }).dragTo(target, { targetPosition: { x: 60, y: 3 } });
  d = await state(page);
  expect(d.layers.find((l: any) => l.name === "Layer 2").parentId).toBe(folder.id);
  expect(d.layers.findIndex((l: any) => l.name === "Layer 2")).toBeGreaterThan(d.layers.findIndex((l: any) => l.name === "Layer 1"));
  await page.keyboard.down("Alt");
  await page.locator('[data-testid="layer-row"]', { hasText: "Layer 1" }).dragTo(page.locator('[data-testid="layer-row"]', { hasText: "Folder 1" }), { targetPosition: { x: 60, y: 3 } });
  await page.keyboard.up("Alt");
  d = await state(page);
  expect(d.layers.map((l: any) => l.name)).toContain("Layer 1 copy");
  expect(d.layers.find((l: any) => l.name === "Layer 1 copy").parentId).toBeNull();
});

test("layer properties and masks", async ({ page }) => {
  await fresh(page);
  // The Opacity field commits on Enter or blur, not on every keystroke, so that typing
  // "40" over the selected text is one undo entry rather than two.
  await page.getByLabel("Opacity").fill("40");
  await page.getByLabel("Opacity").press("Enter");
  await page.getByLabel("Blend mode").selectOption("Multiply");
  let d = await state(page);
  expect(d.layers[0].opacity).toBeCloseTo(0.4, 5);
  expect(d.layers[0].blendMode).toBe("Multiply");
  await page.getByTestId("layer-add-mask").click();
  d = await state(page);
  expect(d.layers[0].hasMask).toBe(true);
  expect(await page.evaluate(() => (window as any).__compositor.store.getState().maskSelected)).toBe(true);
  await page.getByTestId(`target-pixels-${d.layers[0].id}`).click();
  expect(await page.evaluate(() => (window as any).__compositor.store.getState().maskSelected)).toBe(false);
  await page.getByRole("button", { name: "Disable mask" }).click();
  d = await state(page);
  expect(d.layers[0].maskEnabled).toBe(false);
  await page.getByRole("button", { name: "Unlink mask" }).click();
  expect((await state(page)).layers[0].maskLinked).toBe(false);
  await page.getByRole("button", { name: "Delete mask" }).click();
  expect((await state(page)).layers[0].hasMask).toBe(false);
});

test("clipping via the context menu asks before deleting a source", async ({ page }) => {
  await fresh(page);
  await page.getByTestId("layer-add").click();
  await page.getByTestId("layer-row").nth(0).click({ button: "right" });
  await page.getByTestId("ctx-clip").click();
  let d = await state(page);
  expect(d.layers[1].maskSourceId).toBe(d.layers[0].id);
  await page.getByTestId("layer-row").nth(1).click();
  page.once("dialog", (dialog) => dialog.dismiss());
  await page.getByTestId("layer-delete").click();
  d = await state(page);
  expect(d.layers.length).toBe(1);
  expect(d.layers[0].maskSourceId).toBeNull();
});

test("layer menu and shortcuts: duplicate, blend cycling, opacity digits, merge, flip", async ({ page }) => {
  await freshWithPixels(page);
  await page.keyboard.press("Control+j");
  expect((await names(page)).length).toBe(2);
  // LayerBlendMode.allCases order: Normal, Darken, Multiply, Color Burn, Linear Burn, Lighten,
  // Screen, Color Dodge, Linear Dodge (Add), Overlay, Soft Light, Hard Light, Vivid Light,
  // Linear Light, Pin Light, Hard Mix, Difference, Exclusion, Subtract, Divide, Hue, Saturation,
  // Color, Luminosity.
  await page.keyboard.press("Shift+=");
  expect((await state(page)).layers[1].blendMode).toBe("Darken");
  await page.keyboard.press("Shift+-");
  await page.keyboard.press("Shift+-");
  expect((await state(page)).layers[1].blendMode).toBe("Luminosity");
  await page.getByTestId("tool-move").click();
  await page.keyboard.press("5");
  expect((await state(page)).layers[1].opacity).toBeCloseTo(0.5, 5);
  // Real gap here must exceed the 600ms digit-combine window: an automated keypress
  // lands only tens of ms after the previous one, which would otherwise still combine
  // with this unrelated "5" press's own buffer (5 then 2 -> 52%) instead of starting
  // the fresh two-digit sequence below (measured gap without this wait: ~34ms).
  await page.waitForTimeout(650);
  await page.keyboard.press("2"); await page.keyboard.press("5");
  expect((await state(page)).layers[1].opacity).toBeCloseTo(0.25, 5);
  await clickMenu(page, "Layer", "layer-flip-h");
  expect((await state(page)).layers[1].transform.flipX).toBe(true);
  await page.keyboard.press("Control+e");
  expect((await names(page)).length).toBe(1);
  await page.keyboard.press("Control+z");
  expect((await names(page)).length).toBe(2);
  // The physical Delete key is now handled in exactly one place (delete-layer in the
  // keymap), since LayersList's own window keydown listener for it was removed: select a
  // row and confirm the keymap-routed path alone deletes it.
  await page.getByTestId("layer-row").first().click();
  await page.keyboard.press("Delete");
  expect((await names(page)).length).toBe(1);
});
