import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { test, expect, type Page } from "@playwright/test";
import { claimedPngBase64, clickMenu, hueSafeNoisePngBase64, noisePngBase64 } from "./helpers";

const PROBES = path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "engine", "tests", "fixtures", "mac-1.2.10-probes");

/** adjust-render.spec.ts's setup: a 64 x 64 noise layer at zoom 1, checkerboard off, on a pinned
 * viewport: 720 tall while a pixel layer is selected, 721 while an adjustment layer is (the chrome
 * differs by 21 px, adjust-render.spec.ts:14-29), so the document sits on whole device pixels. */
async function setupNoise(page: Page, height: 720 | 721): Promise<string> {
  await page.setViewportSize({ width: 1280, height });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(hueSafeNoisePngBase64);
  return page.evaluate(async (data) => {
    const api = (window as any).__compositor;
    const doc = api.engine.newDocument(64, 64, false);
    api.engine.importImage(doc, Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0)), "noise", { x: 32, y: 32 });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
    return doc as string;
  }, b64);
}

const run = (page: Page, cmd: unknown) => page.evaluate((cmd) => {
  const api = (window as any).__compositor; const s = api.store.getState();
  api.engine.execute(s.activeId, cmd); s.refresh(); s.invalidate();
}, cmd);

/** What the GPU drew for the document, premultiplied, after two frames. */
async function glPixels(page: Page): Promise<number[]> {
  return page.evaluate(async () => {
    const api = (window as any).__compositor;
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    return Array.from(api.readDocumentPixels()) as number[];
  });
}

/** The GPU against the CPU compositor for the whole document, through the open panel's edit. */
async function expectMatchesCpu(page: Page, label: string, tolerance = 2) {
  const r = await page.evaluate(async () => {
    const api = (window as any).__compositor;
    const s = api.store.getState(); const d = s.documents[s.activeId];
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const gl = Array.from(api.readDocumentPixels()) as number[];
    const cpu = Array.from(api.engine.compositeEdit(d.id, s.previewEdit(), { x: 0, y: 0, width: d.width, height: d.height }, d.width, d.height)) as number[];
    return { gl, cpu, kind: s.rendererKind };
  });
  expect(r.kind, "the GPU renderer, not the CPU fallback").toBe("gl");
  expect(r.gl.length).toBe(r.cpu.length);
  const worst = r.gl.reduce((m, v, i) => Math.max(m, Math.abs(v - r.cpu[i])), 0);
  expect(worst, `${label}: max byte diff`).toBeLessThanOrEqual(tolerance);
}

/** Opens a committed Mac probe and selects its bottom layer, a pixel layer, for the 720 pinning. */
async function openProbe(page: Page, name: string): Promise<void> {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const dir = path.join(PROBES, `${name}.comp`);
  const manifest = fs.readFileSync(path.join(dir, "manifest.json"), "utf8");
  const images = fs.readdirSync(path.join(dir, "images")).map((file) => ({ name: file, b64: fs.readFileSync(path.join(dir, "images", file)).toString("base64") }));
  await page.evaluate(async ({ manifest, images }) => {
    const api = (window as any).__compositor;
    const files = { manifest, images: images.map((i: { name: string; b64: string }) => ({ name: i.name, bytes: Uint8Array.from(atob(i.b64), (c: string) => c.charCodeAt(0)) })) };
    const doc = api.engine.openPackage(files, null);
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
    const bottom = api.engine.state(doc).layers[0].id;
    api.store.getState().selectLayers([bottom], bottom);
  }, { manifest, images });
}

/** The Mac's export of a probe, premultiplied, decoded by this build's importer into a scratch document. */
async function macPixels(page: Page, name: string): Promise<number[]> {
  const b64 = fs.readFileSync(path.join(PROBES, `${name}.mac-1.2.10.png`)).toString("base64");
  return page.evaluate((b64) => {
    const api = (window as any).__compositor;
    const doc = api.engine.importImage(null, Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0)), "mac", null);
    const d = api.engine.state(doc);
    const px = Array.from(api.engine.composite(doc, { x: 0, y: 0, width: d.width, height: d.height }, d.width, d.height)) as number[];
    api.engine.closeDocument(doc);
    return px;
  }, b64);
}

const worstOf = (a: number[], b: number[]) => { expect(a.length).toBe(b.length); return a.reduce((m, v, i) => Math.max(m, Math.abs(v - b[i])), 0); };

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

const NEW_MODES = ["Linear Burn", "Linear Dodge (Add)", "Soft Light", "Hard Light", "Vivid Light", "Linear Light", "Pin Light", "Hard Mix", "Exclusion", "Subtract", "Divide"];

test("the eleven blend modes Mac 1.2.6 added draw on the GPU as on the CPU, through a translucent layer", async ({ page }) => {
  await setupNoise(page, 720);
  // A second, different noise on top at 60%: every mode meets many backdrop and source values.
  const b64 = await page.evaluate(noisePngBase64);
  const top = await page.evaluate((data) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    api.engine.importImage(s.activeId, Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0)), "top", { x: 32, y: 32 });
    const id = api.engine.state(s.activeId).activeLayerId;
    api.engine.execute(s.activeId, { type: "SetLayerOpacity", id, opacity: 0.6 });
    s.refresh(); s.invalidate();
    return id as string;
  }, b64);
  for (const mode of NEW_MODES) {
    await run(page, { type: "SetLayerBlendMode", id: top, mode });
    await expectMatchesCpu(page, mode);
  }
});

test("the new-blend-modes probe draws on the GPU as the Mac exported it", async ({ page }) => {
  await openProbe(page, "new-blend-modes");
  expect(worstOf(await glPixels(page), await macPixels(page, "new-blend-modes"))).toBeLessThanOrEqual(2);
});

/** A new adjustment layer of `kind` on the active document, with `settings` merged into it; it becomes the active layer. */
async function addAdjustment(page: Page, kind: string, settings: object | null): Promise<string> {
  return page.evaluate(({ kind, settings }) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    api.engine.execute(s.activeId, { type: "AddAdjustmentLayer", kind, seed: 7, shadows: null, highlights: null });
    const layer = api.engine.state(s.activeId).layers.filter((l: any) => l.adjustment).at(-1);
    if (settings) api.engine.execute(s.activeId, { type: "SetAdjustment", id: layer.id, adjustment: { ...layer.adjustment, ...settings } });
    s.refresh(); s.invalidate();
    return layer.id as string;
  }, { kind, settings });
}

test("the four per-pixel 1.2.6 kinds draw on the GPU as on the CPU", async ({ page }) => {
  await setupNoise(page, 721);
  const cases: [string, object | null, number][] = [
    ["Invert", null, 2],
    ["Black & White", null, 2],
    ["Black & White", { blackWhiteSettings: { reds: 115, yellows: -40, greens: 70, cyans: 180, blues: -90, magentas: 20, tint: true, tintHue: 205, tintSaturation: 45 } }, 2],
    ["Color Balance", { colorBalanceSettings: { shadowCyanRed: 40, shadowMagentaGreen: -20, shadowYellowBlue: 30, midCyanRed: -35, midMagentaGreen: 25, midYellowBlue: -15, highlightCyanRed: 20, highlightMagentaGreen: 45, highlightYellowBlue: -50, preserveLuminosity: true } }, 2],
    ["Color Balance", { colorBalanceSettings: { shadowCyanRed: 40, shadowMagentaGreen: -20, shadowYellowBlue: 30, midCyanRed: -35, midMagentaGreen: 25, midYellowBlue: -15, highlightCyanRed: 20, highlightMagentaGreen: 45, highlightYellowBlue: -50, preserveLuminosity: false } }, 2],
    ["Add Noise", { noiseAmount: 30, noiseGaussian: false, noiseMonochromatic: false, noiseSeed: 12345 }, 2],
    // Box-Muller's log and cos round differently in GLSL: one more level.
    ["Add Noise", { noiseAmount: 45, noiseGaussian: true, noiseMonochromatic: true, noiseSeed: 777 }, 3],
  ];
  for (const [kind, settings, tolerance] of cases) {
    const id = await addAdjustment(page, kind, settings);
    await expectMatchesCpu(page, `${kind} ${JSON.stringify(settings)}`, tolerance);
    await run(page, { type: "DeleteLayers", ids: [id], bake: false });
  }
});

test("an Invert layer turns each opaque pixel into 255 minus itself on the GPU", async ({ page }) => {
  const doc = await setupNoise(page, 721);
  // The noise layer's own stored pixels: an expectation that does not come from either renderer.
  const noise = await page.evaluate((doc) => {
    const api = (window as any).__compositor;
    const id = api.engine.state(doc).layers[0].id;
    return Array.from(api.engine.layerPixels(doc, id, 0)) as number[];
  }, doc);
  await addAdjustment(page, "Invert", null);
  const gl = await glPixels(page);
  const want = noise.map((v, i) => (i % 4 === 3 ? v : 255 - v));
  expect(worstOf(gl, want)).toBeLessThanOrEqual(1);
});

test("the Black & White probe draws on the GPU as the Mac exported it", async ({ page }) => {
  await openProbe(page, "new-adjustment-layers");
  expect(worstOf(await glPixels(page), await macPixels(page, "new-adjustment-layers"))).toBeLessThanOrEqual(2);
});
