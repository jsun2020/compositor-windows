import { test, expect, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { clickMenu } from "./helpers";

// Project format 11 through the app (Phase 4.5): a real Compositor for Mac save at format 11 opens from
// File > Open and saves again at 11; a newer format says which versions this build reads. The engine's
// own tests (engine/tests/format_v11.rs) pin the text runs.
const SHAPES = path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "engine", "tests", "fixtures", "mac-4b1-probes", "shapes.mac-1.3.7.comp");

/** Puts the Mac's saved package into the mock bridge at `at`, its manifest changed by `edit`. Its
 * QuickLook folder is left out, as the real bridge reads only manifest.json and images/. */
async function seed(page: Page, at: string, edit: (manifest: string) => string = (m) => m) {
  const manifest = edit(fs.readFileSync(path.join(SHAPES, "manifest.json"), "utf8"));
  const images = fs.readdirSync(path.join(SHAPES, "images")).map((name) => ({ name, bytes: Array.from(fs.readFileSync(path.join(SHAPES, "images", name))) }));
  await page.evaluate(async ({ at, manifest, images }) => {
    const api = (window as any).__compositor;
    await api.bridge.writePackage(at, { manifest, images: images.map((i: { name: string; bytes: number[] }) => ({ name: i.name, bytes: new Uint8Array(i.bytes) })) });
  }, { at, manifest, images });
}

test("a Mac save at format 11 opens from File > Open and saves again at 11, its shapes as the Mac wrote them", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await seed(page, "C:/mac/shapes.comp");
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/mac/shapes.comp"));
  await clickMenu(page, "File", "open");
  await expect(page.getByTestId("layer-row")).toHaveCount(7);
  await expect(page.getByTestId("error-banner")).toHaveCount(0);
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/win/shapes.comp"));
  await clickMenu(page, "File", "save-as");
  const [before, after] = await page.evaluate(async () => {
    const bridge = (window as any).__compositor.bridge;
    return [JSON.parse((await bridge.readPackage("C:/mac/shapes.comp")).manifest), JSON.parse((await bridge.readPackage("C:/win/shapes.comp")).manifest)];
  });
  expect(before.version).toBe(11);
  expect(after.version).toBe(11);
  expect(after.layers.map((l: { shape?: unknown }) => l.shape ?? null)).toEqual(before.layers.map((l: { shape?: unknown }) => l.shape ?? null));
});

test("a project from a newer format says which versions this build reads", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await seed(page, "C:/mac/future.comp", (m) => m.replace('"version" : 11', '"version" : 12'));
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/mac/future.comp"));
  await clickMenu(page, "File", "open");
  await expect(page.getByTestId("error-banner")).toContainText("This project uses format version 12. This app supports versions 1-11, which Compositor for Mac saves up to version 1.4.5.");
  await expect(page.getByTestId("layer-row")).toHaveCount(0);
});
