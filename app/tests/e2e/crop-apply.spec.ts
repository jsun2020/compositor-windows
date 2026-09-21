import { test, expect, type Page } from "@playwright/test";
import { clickMenu } from "./helpers";

// Regression for "Apply does nothing": the crop tool used to treat a missing rectangle as an
// implicit full-canvas frame, so Apply cropped the canvas to its own size and the frame and
// the buttons stayed exactly as they were. Now choosing the tool seeds an explicit rectangle,
// Apply and Cancel clear it, and without one the frame is gone and the buttons are disabled.

async function fresh(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await clickMenu(page, "File", "new");
  await page.getByRole("button", { name: "Create" }).click();
}
const store = (page: Page) => page.evaluate(() => {
  const s = (window as any).__compositor.store.getState();
  const d = s.documents[s.activeId];
  return { tool: s.tool, cropRect: s.cropRect, width: d.width, height: d.height };
});

test("choosing the crop tool seeds a full-canvas rectangle and Apply clears it", async ({ page }) => {
  await fresh(page);
  await page.getByTestId("tool-crop").click();
  let s = await store(page);
  expect(s.cropRect).toEqual({ x: 0, y: 0, width: s.width, height: s.height });
  await expect(page.getByTestId("crop-size")).toHaveText(`${s.width} x ${s.height}`);
  await expect(page.getByTestId("crop-apply")).toBeEnabled();
  await page.getByTestId("crop-apply").click();
  s = await store(page);
  expect(s.tool).toBe("crop");
  expect(s.cropRect).toBeNull();
  await expect(page.getByTestId("crop-size")).toHaveText("no selection");
  await expect(page.getByTestId("crop-apply")).toBeDisabled();
  await expect(page.getByTestId("crop-cancel")).toBeDisabled();
  // Enter with no rectangle is a no-op rather than a full-canvas crop.
  const before = await page.evaluate(() => (window as any).__compositor.store.getState().documents[(window as any).__compositor.store.getState().activeId].canUndo);
  await page.keyboard.press("Enter");
  const after = await page.evaluate(() => (window as any).__compositor.store.getState().documents[(window as any).__compositor.store.getState().activeId].canUndo);
  expect(after).toBe(before);
});

test("Cancel clears the rectangle and re-entering the tool seeds it again", async ({ page }) => {
  await fresh(page);
  await page.getByTestId("tool-crop").click();
  await page.getByTestId("crop-cancel").click();
  expect((await store(page)).cropRect).toBeNull();
  await expect(page.getByTestId("crop-apply")).toBeDisabled();
  await page.getByTestId("tool-move").click();
  await page.getByTestId("tool-crop").click();
  const s = await store(page);
  expect(s.cropRect).toEqual({ x: 0, y: 0, width: s.width, height: s.height });
});
