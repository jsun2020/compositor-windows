import { test, expect, type Page } from "@playwright/test";
import { redSquarePngBase64 } from "./helpers";

async function seedImage(page: Page, path: string): Promise<void> {
  const b64 = await page.evaluate(redSquarePngBase64);
  await page.evaluate(({ path, b64 }) => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    api.bridge.seedFile(path, Uint8Array.from(atob(b64), (c) => c.charCodeAt(0)));
  }, { path, b64 });
}

test("dropping an image with no document creates one; dropping onto a document adds a layer", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await seedImage(page, "C:/pics/red.png");
  await page.evaluate(() => (window as any).__compositor.bridge.simulateDrop(["C:/pics/red.png"], null));
  await expect(page.getByTestId("project-tab")).toHaveCount(1);
  await expect(page.getByTestId("layer-row")).toHaveCount(1);
  await page.evaluate(() => (window as any).__compositor.bridge.simulateDrop(["C:/pics/red.png"], { x: 10, y: 10 }));
  await expect(page.getByTestId("layer-row")).toHaveCount(2);
});

test("keyboard shortcuts drive tools and undo", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.keyboard.press("Control+n");
  await page.getByRole("button", { name: "Create" }).click();
  await page.keyboard.press("c");
  await expect(page.getByTestId("crop-options")).toBeVisible();
  await page.keyboard.press("v");
  await expect(page.getByTestId("crop-options")).toHaveCount(0);
  await page.keyboard.press("Control+Shift+n");
  await expect(page.getByTestId("layer-row")).toHaveCount(2);
  await page.keyboard.press("Control+z");
  await expect(page.getByTestId("layer-row")).toHaveCount(1);
});
