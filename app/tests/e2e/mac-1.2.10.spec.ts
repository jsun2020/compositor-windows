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

test("the engine hands the GPU its blur sizes, the plan's reach and the canvas-anchored lattice", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const r = await page.evaluate(() => {
    const api = (window as any).__compositor;
    const doc = api.engine.newDocument(32, 32, true);
    const before = api.engine.renderPlan(doc, null).spatialMargin;
    api.engine.execute(doc, { type: "AddAdjustmentLayer", kind: "Gaussian Blur", seed: 0, shadows: null, highlights: null });
    const layer = api.engine.state(doc).layers.find((l: any) => l.adjustment);
    const blurs = [
      api.engine.spatialBlur(layer.adjustment, 2),                                  // radius absent: 10, so sigma 20, reach 60
      api.engine.spatialBlur({ ...layer.adjustment, blurRadius: 16 }, 1),           // reach 48, the limit: no halving
      api.engine.spatialBlur({ ...layer.adjustment, blurRadius: 250 }, 1),          // reach 750: four halvings
      api.engine.spatialBlur({ ...layer.adjustment, kind: "Motion Blur", motionAngle: 30, motionDistance: 97 }, 1),   // reach 48.5, halved 3x past the Motion Blur limit of 12 (48.5 -> 24.25 -> 12.125 -> 6.0625)
    ];
    api.engine.execute(doc, { type: "SetAdjustment", id: layer.id, adjustment: { ...layer.adjustment, blurRadius: 6 } });
    return {
      before, after: api.engine.renderPlan(doc, null).spatialMargin, blurs,
      grids: [1, 4, 100].map((s: number) => api.engine.spatialGrid(doc, null, s)),
      span: api.engine.spatialSpan(77, 117, 200, { cell: 4, pad: 10 }),
    };
  });
  expect(r.before).toBe(0);
  expect(r.after, "3 x 6 + 2 document pixels").toBe(20);
  expect(r.blurs).toEqual([
    { level: 1, sigma: 20, distance: 0, angle: 0 }, { level: 0, sigma: 16, distance: 0, angle: 0 },
    { level: 4, sigma: 250, distance: 0, angle: 0 }, { level: 3, sigma: 0, distance: 97, angle: 30 },
  ]);
  // Radius 6 at 1 output px per document px: reach 18, no halving, pad 20 + 3 cells of 1. At 4:
  // reach 72, one halving, pad 80 + 3 x 2. At 100: six halvings, and the pad stops at 1024.
  expect(r.grids).toEqual([{ cell: 1, pad: 23 }, { cell: 2, pad: 86 }, { cell: 64, pad: 1024 }]);
  expect(r.span, "77..117 grown by 10, then out to the lattice of 4").toEqual([64, 128]);
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
