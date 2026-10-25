import { test, expect, type Page } from "@playwright/test";
import { clickMenu, noisePngBase64 } from "./helpers";

async function setup(page: Page, canvasSize = 64) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(noisePngBase64);
  await page.evaluate(async ({ data, canvasSize }) => {
    const api = (window as any).__compositor;
    const png = Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(canvasSize, canvasSize, false);
    api.engine.importImage(doc, png, "noise", { x: canvasSize / 2, y: canvasSize / 2 });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
  }, { data: b64, canvasSize });
}
const layer = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].layers[0]; });

test("a gaussian blur grows an interior layer, previews, and applies as one undo step", async ({ page }) => {
  // 1.5.7 extends canvas-touching edges. Keep this growth case inside a larger canvas.
  await setup(page, 128);
  const before = await layer(page);
  const depthBefore = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].undoDepth; });
  await clickMenu(page, "Filter", "filter-gaussian-blur");
  await expect(page.getByTestId("adjust-title")).toHaveText("Gaussian Blur");
  await page.getByLabel("Radius").fill("5");
  await page.getByLabel("Radius").press("Enter");
  const previewed = await layer(page);
  expect(previewed.transform.size[0]).toBeGreaterThan(before.transform.size[0]);
  expect(await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].undoDepth; })).toBe(depthBefore); // preview alone records nothing
  await page.getByTestId("adjust-ok").click();
  const after = await layer(page);
  expect(after.transform.size[0]).toBeGreaterThan(before.transform.size[0]);
  expect(after.transform.origin[0]).toBeLessThan(before.transform.origin[0]);
  await page.keyboard.press("Control+z");
  const undone = await layer(page);
  expect(undone.transform).toEqual(before.transform);
  expect(undone.pixelsWidth).toBe(before.pixelsWidth);
});

test("motion blur, add noise and lens correction each apply once", async ({ page }) => {
  await setup(page);
  const depthBefore = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].undoDepth; });
  for (const [id, label, value] of [["filter-motion-blur", "Distance", "24"], ["filter-add-noise", "Amount", "30"], ["filter-lens-correction", "Remove distortion", "-60"]] as const) {
    const before = (await layer(page)).pixelsRevision;
    await clickMenu(page, "Filter", id);
    await page.getByLabel(label).fill(value);
    await page.getByLabel(label).press("Enter");
    await page.getByTestId("adjust-ok").click();
    expect((await layer(page)).pixelsRevision).toBeGreaterThan(before);
  }
  const depthAfter = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].undoDepth; });
  expect(depthAfter).toBe(depthBefore + 3); // one undo step per filter
});

test("the image menu opens every adjustment and Invert applies at once", async ({ page }) => {
  await setup(page);
  for (const [id, title] of [["image-levels", "Levels"], ["image-curves", "Curves"], ["image-hue-saturation", "Hue/Saturation"], ["image-exposure", "Exposure"], ["image-gradient-map", "Gradient Map"], ["image-grain", "Grain"]] as const) {
    await clickMenu(page, "Image", id);
    await expect(page.getByTestId("adjust-title")).toHaveText(title);
    await page.getByTestId("adjust-cancel").click();
  }
  const before = (await layer(page)).pixelsRevision;
  await clickMenu(page, "Image", "image-invert");
  await expect(page.getByTestId("adjust-panel")).toHaveCount(0); // Invert has no panel
  expect((await layer(page)).pixelsRevision).toBeGreaterThan(before);
});

test("the shortcuts open their panels and the menu items grey out while one is open", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("Control+l");
  await expect(page.getByTestId("adjust-title")).toHaveText("Levels");
  await page.getByRole("button", { name: "Image", exact: true }).click();
  await expect(page.getByTestId("menu-image-curves")).toBeDisabled();
  await page.keyboard.press("Escape");
  await page.keyboard.press("Control+m");
  await expect(page.getByTestId("adjust-title")).toHaveText("Curves");
  await page.keyboard.press("Escape");
  await page.keyboard.press("Control+u");
  await expect(page.getByTestId("adjust-title")).toHaveText("Hue/Saturation");
  await page.keyboard.press("Escape");
  const before = (await layer(page)).pixelsRevision;
  await page.keyboard.press("Control+i");
  expect((await layer(page)).pixelsRevision).toBeGreaterThan(before);
});

test("a canvas-filling gaussian blur extends its edges without growing the layer", async ({page}) => {
  await setup(page);const before=await layer(page);
  await clickMenu(page,"Filter","filter-gaussian-blur");await page.getByLabel("Radius").fill("5");await page.getByLabel("Radius").press("Enter");
  expect((await layer(page)).transform).toEqual(before.transform);
  await page.getByTestId("adjust-ok").click();const after=await layer(page);expect(after.transform).toEqual(before.transform);expect(after.pixelsRevision).not.toBe(before.pixelsRevision);
  await page.keyboard.press("Control+z");expect((await layer(page)).transform).toEqual(before.transform);
});
