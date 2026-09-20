import { test, expect, type Page } from "@playwright/test";
import { clickMenu, redSquarePngBase64 } from "./helpers";

async function seedImage(page: Page, path: string): Promise<void> {
  const b64 = await page.evaluate(redSquarePngBase64);
  await page.evaluate(({ path, b64 }) => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    api.bridge.seedFile(path, Uint8Array.from(atob(b64), (c) => c.charCodeAt(0)));
  }, { path, b64 });
}

test("new canvas, import, save, close and reopen", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await clickMenu(page, "File", "new");
  await page.getByLabel("Width").fill("300");
  await page.getByLabel("Height").fill("200");
  await page.getByRole("button", { name: "Create" }).click();
  await expect(page.getByTestId("project-tab").first()).toContainText("Untitled");
  await expect(page.getByTestId("layer-row")).toHaveCount(1);

  await seedImage(page, "C:/pics/red.png");
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/pics/red.png"));
  await clickMenu(page, "File", "import");
  await expect(page.getByTestId("layer-row")).toHaveCount(2);
  await expect(page.getByTestId("layer-row").first()).toContainText("red");

  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/projects/Test.comp"));
  await clickMenu(page, "File", "save");
  await expect(page.getByTestId("project-tab").first()).toContainText("Test");
  await expect(page.getByTestId("project-tab").first()).not.toContainText("\u2022");
  const saved = await page.evaluate(() => (window as any).__compositor.bridge.hasPackage("C:/projects/Test.comp"));
  expect(saved).toBe(true);

  await clickMenu(page, "File", "close");
  await expect(page.getByTestId("project-tab")).toHaveCount(0);

  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/projects/Test.comp"));
  await clickMenu(page, "File", "open");
  await expect(page.getByTestId("layer-row")).toHaveCount(2);
  const state = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
  expect([state.width, state.height]).toEqual([300, 200]);
  expect(state.isModified).toBe(false);
});

test("export png writes a file of the canvas size", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await clickMenu(page, "File", "new");
  await page.getByRole("button", { name: "Create" }).click();
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/out/Untitled.png"));
  await clickMenu(page, "File", "export-png");
  const size = await page.evaluate(async () => {
    const bytes = (window as any).__compositor.bridge.fileBytes("C:/out/Untitled.png") as Uint8Array;
    const blob = new Blob([bytes as unknown as BlobPart], { type: "image/png" });
    const bmp = await createImageBitmap(blob);
    return [bmp.width, bmp.height];
  });
  expect(size).toEqual([1920, 1080]);
});

test("closing a modified document asks first", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await clickMenu(page, "File", "new");
  await page.getByRole("button", { name: "Create" }).click();
  await page.getByTestId("layer-add").click();
  await expect(page.getByTestId("project-tab").first()).toContainText("\u2022");
  page.once("dialog", (d) => d.dismiss());
  await clickMenu(page, "File", "close");
  await expect(page.getByTestId("project-tab")).toHaveCount(1);
});
