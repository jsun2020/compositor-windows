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

test("save as failure shows the error banner", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await clickMenu(page, "File", "new");
  await page.getByRole("button", { name: "Create" }).click();
  await page.getByTestId("layer-add").click();
  await expect(page.getByTestId("project-tab").first()).toContainText("\u2022");
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/projects/Fail.comp"));
  await page.evaluate(() => (window as any).__compositor.bridge.failNextWrite("disk is full"));
  await clickMenu(page, "File", "save-as");
  await expect(page.getByTestId("error-banner")).toContainText("disk is full");
  await expect(page.getByTestId("project-tab").first()).toContainText("Untitled");
  await expect(page.getByTestId("project-tab").first()).toContainText("\u2022");
});

test("two documents from one package do not share textures", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();

  // Build a small canvas with an imported layer, then Save As twice from the same session
  // so both files on disk carry the same layer id.
  await clickMenu(page, "File", "new");
  await page.getByLabel("Width").fill("300");
  await page.getByLabel("Height").fill("200");
  await page.getByRole("button", { name: "Create" }).click();

  await seedImage(page, "C:/p/red.png");
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/p/red.png"));
  await clickMenu(page, "File", "import");
  await expect(page.getByTestId("layer-row")).toHaveCount(2);

  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/p/A.comp"));
  await clickMenu(page, "File", "save-as");
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/p/B.comp"));
  await clickMenu(page, "File", "save-as");
  await expect(page.getByTestId("project-tab")).toHaveCount(1);
  await expect(page.getByTestId("project-tab").first()).toContainText("B");

  // Open A fresh: the active tab's path is now B.comp, so `openProject`'s dedupe does not
  // match and this is a genuinely new session -- one that happens to carry the same layer id
  // A.comp was saved with, since both files came from the same original session.
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/p/A.comp"));
  await clickMenu(page, "File", "open");
  await expect(page.getByTestId("project-tab")).toHaveCount(2);
  await expect(page.getByTestId("project-tab").last()).toContainText("A");

  // Resize the newly opened A tab so its layer's pixels_revision and pixel dimensions
  // diverge from B's, then render it so its (colliding, pre-fix) layer id occupies the
  // shared texture cache.
  await clickMenu(page, "Image", "image-size");
  await page.getByLabel("Constrain proportions").uncheck();
  await page.getByLabel("Width").fill("8");
  await page.getByLabel("Height").fill("8");
  await page.getByRole("button", { name: "Apply" }).click();
  await page.evaluate(() => (window as any).__compositor.setZoom(1));

  // Switch to B, the untouched tab, and confirm it still renders at its own, unresized size
  // instead of picking up A's stale-but-matching-key texture.
  await page.getByTestId("project-tab").first().click();
  const bState = await page.evaluate(() => {
    const s = (window as any).__compositor.store.getState();
    return s.documents[s.activeId];
  });
  await page.evaluate(() => (window as any).__compositor.setZoom(1));
  await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
  const pixelsLength = await page.evaluate(() => (window as any).__compositor.readDocumentPixels().length);
  expect(pixelsLength).toBe(bState.width * bState.height * 4);
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
