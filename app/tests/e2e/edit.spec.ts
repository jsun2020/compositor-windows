import { test, expect } from "@playwright/test";
import { clickMenu } from "./helpers";

test("crop via the options bar, canvas size and image size sheets", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await clickMenu(page, "File", "new");
  await page.getByLabel("Width").fill("400");
  await page.getByLabel("Height").fill("300");
  await page.getByRole("button", { name: "Create" }).click();
  const state = () => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });

  await page.getByTestId("tool-crop").click();
  await page.evaluate(() => (window as any).__compositor.store.getState().setCropRect({ x: 50, y: 40, width: 200, height: 100 }));
  await page.getByTestId("crop-options").locator("select").focus();
  await page.keyboard.press("Enter");
  let d = await state();
  expect([d.width, d.height]).toEqual([400, 300]);
  await page.getByTestId("crop-apply").click();
  d = await state();
  expect([d.width, d.height]).toEqual([200, 100]);
  expect(d.layers[0].transform.origin).toEqual([-50, -40]);

  await clickMenu(page, "Image", "canvas-size");
  await page.getByLabel("Width").fill("300");
  await page.getByTestId("anchor-0").click();
  await page.getByRole("button", { name: "Apply" }).click();
  d = await state();
  expect([d.width, d.height]).toEqual([300, 100]);
  expect(d.layers[0].transform.origin).toEqual([-50, -40]);

  await clickMenu(page, "Image", "image-size");
  await page.getByLabel("Width").fill("600");
  await page.getByLabel("Resolution").fill("300");
  await page.getByRole("button", { name: "Apply" }).click();
  d = await state();
  expect([d.width, d.height, d.resolution]).toEqual([600, 200, 300]);

  await clickMenu(page, "Image", "flip-h");
  d = await state();
  expect(d.layers[0].transform.flipX).toBe(true);
  await clickMenu(page, "Edit", "undo");
  d = await state();
  expect(d.layers[0].transform.flipX).toBe(false);
});

test("jpeg export sheet previews and writes", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await clickMenu(page, "File", "new");
  await page.getByRole("button", { name: "Create" }).click();
  await clickMenu(page, "File", "export-jpeg");
  await expect(page.getByTestId("jpeg-size")).not.toHaveText("");
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/out/Untitled.jpg"));
  await page.getByRole("button", { name: "Export" }).click();
  const head = await page.evaluate(() => Array.from(((window as any).__compositor.bridge.fileBytes("C:/out/Untitled.jpg") as Uint8Array).subarray(0, 3)));
  expect(head).toEqual([0xff, 0xd8, 0xff]);
});
