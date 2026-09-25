import { test, expect } from "@playwright/test";
import { claimedPngBase64, clickMenu } from "./helpers";

test("opening a project over 100 megapixels says so in the error banner", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const png = await page.evaluate(claimedPngBase64, { width: 10_000, height: 6_000 });
  const ids = ["0B6C6B1E-4F1B-4B4E-9E0A-CCCCCCCCCCC1", "0B6C6B1E-4F1B-4B4E-9E0A-CCCCCCCCCCC2"];
  const manifest = {
    format: "com.compositor.project", version: 9, colorSpace: "sRGB",
    documentID: "0B6C6B1E-4F1B-4B4E-9E0A-CCCCCCCCCCC0", width: 100, height: 100,
    layers: ids.map((id) => ({ id, name: "L", isVisible: true, imageFile: `${id}.png`,
      transform: { origin: [0, 0], size: [100, 100], rotation: 0, flipX: false, flipY: false, sampling: "High quality" } })),
  };
  await page.evaluate(async ({ manifest, png, ids }) => {
    const bridge = (window as any).__compositor.store.getState().bridge;
    const bytes = Uint8Array.from(atob(png), (c: string) => c.charCodeAt(0));
    await bridge.writePackage("C:/big.comp", { manifest: JSON.stringify(manifest), images: ids.map((id: string) => ({ name: `${id}.png`, bytes })) });
    bridge.setNextPick("C:/big.comp");
  }, { manifest, png, ids });
  await clickMenu(page, "File", "open");
  const banner = page.getByTestId("error-banner");
  await expect(banner).toContainText("larger than Compositor for Windows supports");
  await expect(banner).toContainText("100 megapixels");
  expect(await page.evaluate(() => Object.keys((window as any).__compositor.store.getState().documents).length)).toBe(0);
});

test("importing an image over 100 megapixels says so in the error banner, before decoding it", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  // 10,000 x 10,001 = 100,010,000 pixels, just over budget; the claimed body is one empty zlib
  // block, so decoding it fully would fail differently - the banner proves the header check runs
  // first (fix round 1, task-5-review.md finding 1: decode_image, not just package open/save).
  const png = await page.evaluate(claimedPngBase64, { width: 10_000, height: 10_001 });
  await page.evaluate(async ({ png }) => {
    const bridge = (window as any).__compositor.bridge;
    const bytes = Uint8Array.from(atob(png), (c: string) => c.charCodeAt(0));
    bridge.seedFile("C:/big-import.png", bytes);
    bridge.setNextPick("C:/big-import.png");
  }, { png });
  await clickMenu(page, "File", "import");
  const banner = page.getByTestId("error-banner");
  await expect(banner).toContainText("larger than Compositor for Windows supports");
  await expect(banner).toContainText("100 megapixels");
  expect(await page.evaluate(() => Object.keys((window as any).__compositor.store.getState().documents).length)).toBe(0);
});
