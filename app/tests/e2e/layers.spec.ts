import { test, expect, type Page } from "@playwright/test";
import { clickMenu } from "./helpers";

async function fresh(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await clickMenu(page, "File", "new");
  await page.getByRole("button", { name: "Create" }).click();
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
  await page.getByLabel("Opacity").fill("40");
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
