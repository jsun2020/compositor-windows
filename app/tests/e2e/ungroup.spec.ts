import { test, expect, type Page } from "@playwright/test";
import { clickMenu, solidPngBase64 } from "./helpers";

// Ungroup Layers (Compositor 1.4.5, LayerGroups.swift:214-239): the Layer menu and Shift+Ctrl+G
// (CompositorApp.swift:312-313), and a folder's context menu (NativeLayerList.swift:159-165).

/** A 64 x 48 document: "Below", a folder holding "A" and "B" at half opacity, and "Above". */
async function setup(page: Page): Promise<{ below: string; folder: string; a: string; b: string; above: string }> {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const red = await page.evaluate(solidPngBase64, { width: 8, height: 8, color: "#ff0000" });
  return page.evaluate(async (red) => {
    const api = (window as any).__compositor;
    const png = Uint8Array.from(atob(red), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 48, false);
    const add = (name: string) => { api.engine.importImage(doc, png, name, { x: 32, y: 24 }); return api.engine.state(doc).activeLayerId as string; };
    const below = add("Below"), a = add("A"), b = add("B"), above = add("Above");
    api.engine.execute(doc, { type: "GroupLayers", ids: [a, b] });
    const folder = api.engine.state(doc).activeLayerId as string;
    api.engine.execute(doc, { type: "SetLayerOpacity", id: folder, opacity: 0.5 });
    api.store.getState().openDocument(doc);
    return { below, folder, a, b, above };
  }, red);
}
const doc = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return { d: s.documents[s.activeId], selected: s.selectedLayerIds as string[] }; });
const row = (page: Page, id: string) => page.locator(`[data-testid="layer-row"][data-layer-id="${id}"]`);

test("a folder's context menu ungroups it: its layers take its place, selected, and the folder goes as one step", async ({ page }) => {
  const ids = await setup(page);
  // A plain layer's menu has no Ungroup.
  await row(page, ids.above).click({ button: "right" });
  await expect(page.getByTestId("ctx-ungroup")).toHaveCount(0);
  await page.keyboard.press("Escape");
  await row(page, ids.folder).click({ button: "right" });
  const depth = (await doc(page)).d.undoDepth;
  await page.getByTestId("ctx-ungroup").click();
  const { d, selected } = await doc(page);
  expect(d.layers.map((l: any) => l.id)).toEqual([ids.below, ids.a, ids.b, ids.above]);
  expect(d.layers.every((l: any) => l.parentId === null)).toBe(true);
  expect(d.layers.map((l: any) => l.opacity)).toEqual([1, 1, 1, 1]);
  expect(selected).toEqual([ids.a, ids.b]);
  expect(d.activeLayerId).toBe(ids.a);
  expect(d.undoDepth).toBe(depth + 1);
});

test("Shift+Ctrl+G and Layer > Ungroup Layers ungroup the active folder, and only a folder", async ({ page }) => {
  const ids = await setup(page);
  await row(page, ids.above).click();
  await page.getByRole("button", { name: "Layer", exact: true }).click();
  await expect(page.getByTestId("menu-layer-ungroup")).toBeDisabled();
  await page.keyboard.press("Escape");
  await page.keyboard.press("Shift+Control+g");
  expect((await doc(page)).d.layers.length).toBe(5);
  await row(page, ids.folder).click();
  await page.keyboard.press("Shift+Control+g");
  expect((await doc(page)).d.layers.map((l: any) => l.id)).toEqual([ids.below, ids.a, ids.b, ids.above]);
  await page.keyboard.press("Control+z");
  const back = (await doc(page)).d;
  expect(back.layers.find((l: any) => l.id === ids.folder)?.isGroup).toBe(true);
  expect(back.layers.filter((l: any) => l.parentId === ids.folder).map((l: any) => l.id)).toEqual([ids.a, ids.b]);
  await row(page, ids.folder).click();
  await clickMenu(page, "Layer", "layer-ungroup");
  expect((await doc(page)).d.layers.map((l: any) => l.id)).toEqual([ids.below, ids.a, ids.b, ids.above]);
});
