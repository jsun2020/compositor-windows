import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { test, expect, type Page } from "@playwright/test";
import { claimedPngBase64, clickMenu, grayRampMaskPngBase64, hueSafeNoisePngBase64, noisePngBase64, redSquarePngBase64 } from "./helpers";

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

const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
const field = (page: Page, name: string) => page.getByRole("spinbutton", { name, exact: true });

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

/** Whether the document's device rect starts on whole device pixels (LL-065(6)): checked, not assumed. */
async function onWholePixels(page: Page): Promise<boolean> {
  return page.evaluate(async () => {
    const api = (window as any).__compositor;
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const s = api.store.getState(); const d = s.documents[s.activeId]; const dpr = window.devicePixelRatio || 1;
    const rect = s.viewports[s.activeId].documentRect({ width: d.width, height: d.height });
    return Number.isInteger(rect.x * dpr) && Number.isInteger(rect.y * dpr);
  });
}

test("the blend-greys probe draws on the GPU as the Mac exported it, Soft Light included", async ({ page }) => {
  await openProbe(page, "blend-greys");
  expect(await onWholePixels(page), "the document sits on whole device pixels").toBe(true);
  // The CPU is within 1 of the Mac (mac_1_2_10.rs); W3C's Soft Light was 14 off at the 75% grey.
  // Measured 1 on p4a-scratch (2026-09-27).
  expect(worstOf(await glPixels(page), await macPixels(page, "blend-greys"))).toBeLessThanOrEqual(1);
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

/** How much a tone belongs to the shadows, midtones and highlights, transcribed independently
 * from `tonal_weights` in Compositor-1.2.10/Compositor/Rendering/AdjustPixels.c:156-167 (not from
 * the Rust port under review): three overlapping curves that sum to about one across the range. */
function macTonalWeights(v: number): [number, number, number] {
  const a = 0.25, b = 0.333, scale = 0.7;
  const clamp01 = (x: number) => Math.min(1, Math.max(0, x));
  const s = clamp01((v - b) / -a + 0.5);
  const h = clamp01((v + b - 1) / a + 0.5);
  const m1 = clamp01((v - b) / a + 0.5);
  const m2 = clamp01((v + b - 1) / -a + 0.5);
  return [s * scale, m1 * m2 * scale, h * scale];
}

type MacColorBalanceSettings = {
  shadowCyanRed: number; shadowMagentaGreen: number; shadowYellowBlue: number;
  midCyanRed: number; midMagentaGreen: number; midYellowBlue: number;
  highlightCyanRed: number; highlightMagentaGreen: number; highlightYellowBlue: number;
  preserveLuminosity: boolean;
};
/** One premultiplied pixel through `adjust_color_balance`, transcribed independently from
 * Compositor-1.2.10/Compositor/Rendering/AdjustPixels.c:169-196 (not from the Rust port under
 * review): an oracle the GPU/CPU parity check alone does not provide. */
function macColorBalancePixel(p: [number, number, number, number], s: MacColorBalanceSettings): [number, number, number] {
  const alpha = p[3];
  if (alpha === 0) return [p[0], p[1], p[2]];
  const clamp01 = (x: number) => Math.min(1, Math.max(0, x));
  const c = [0, 1, 2].map((i) => Math.min(255, (p[i] * 255) / alpha) / 255);
  const before = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
  const shadows = [s.shadowCyanRed / 100, s.shadowMagentaGreen / 100, s.shadowYellowBlue / 100];
  const midtones = [s.midCyanRed / 100, s.midMagentaGreen / 100, s.midYellowBlue / 100];
  const highlights = [s.highlightCyanRed / 100, s.highlightMagentaGreen / 100, s.highlightYellowBlue / 100];
  for (let i = 0; i < 3; i++) {
    const [sw, mw, hw] = macTonalWeights(c[i]);
    c[i] = clamp01(c[i] + shadows[i] * sw + midtones[i] * mw + highlights[i] * hw);
  }
  if (s.preserveLuminosity) {
    const after = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
    if (after > 0.0001) {
      const ratio = before / after;
      for (let i = 0; i < 3; i++) c[i] = clamp01(c[i] * ratio);
    }
  }
  return c.map((v) => Math.min(alpha, Math.max(0, Math.round(v * alpha)))) as [number, number, number];
}

test("a Color Balance layer matches AdjustPixels.c's color balance, transcribed independently, at sample pixels", async ({ page }) => {
  const doc = await setupNoise(page, 721);
  // The noise layer's own stored pixels: an expectation that does not come from either renderer.
  const noise = await page.evaluate((doc) => {
    const api = (window as any).__compositor;
    const id = api.engine.state(doc).layers[0].id;
    return Array.from(api.engine.layerPixels(doc, id, 0)) as number[];
  }, doc);
  // Asymmetric, non-default indices across the 64x64 fixture (4096 pixels).
  const samples = [3, 777, 2010, 4095];
  const base = { shadowCyanRed: 40, shadowMagentaGreen: -20, shadowYellowBlue: 30, midCyanRed: -35, midMagentaGreen: 25, midYellowBlue: -15, highlightCyanRed: 20, highlightMagentaGreen: 45, highlightYellowBlue: -50 };
  for (const preserveLuminosity of [true, false]) {
    const settings: MacColorBalanceSettings = { ...base, preserveLuminosity };
    const id = await addAdjustment(page, "Color Balance", { colorBalanceSettings: settings });
    const gl = await glPixels(page);
    for (const i of samples) {
      const off = i * 4;
      const p: [number, number, number, number] = [noise[off], noise[off + 1], noise[off + 2], noise[off + 3]];
      const expected = macColorBalancePixel(p, settings);
      for (let c = 0; c < 3; c++) {
        expect(Math.abs(gl[off + c] - expected[c]), `preserveLuminosity ${preserveLuminosity}, pixel ${i}, channel ${c}`).toBeLessThanOrEqual(2);
      }
      expect(gl[off + 3], `preserveLuminosity ${preserveLuminosity}, pixel ${i}, alpha`).toBe(p[3]);
    }
    await run(page, { type: "DeleteLayers", ids: [id], bake: false });
  }
});

/** A 64x64 fully-opaque mid-grey PNG: a flat, known baseline so Add Noise's statistics can be
 * checked against a formula instead of against pre-existing per-pixel variation. Must be run via
 * `page.evaluate(grayPngBase64)` as a standalone page function, same rule as helpers.ts's
 * `noisePngBase64`. */
async function grayPngBase64(): Promise<string> {
  const canvas = document.createElement("canvas");
  canvas.width = 64;
  canvas.height = 64;
  const ctx = canvas.getContext("2d")!;
  const image = ctx.createImageData(64, 64);
  for (let i = 0; i < 64 * 64; i++) {
    image.data[i * 4] = 128;
    image.data[i * 4 + 1] = 128;
    image.data[i * 4 + 2] = 128;
    image.data[i * 4 + 3] = 255;
  }
  ctx.putImageData(image, 0, 0);
  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("toBlob failed"))), "image/png");
  });
  const bytes = new Uint8Array(await blob.arrayBuffer());
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary);
}

/** A 64x64 fully-opaque mid-grey document at zoom 1, its own adjustment layer selected (721 pinning). */
async function setupGray(page: Page): Promise<string> {
  await page.setViewportSize({ width: 1280, height: 721 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(grayPngBase64);
  return page.evaluate(async (data) => {
    const api = (window as any).__compositor;
    const doc = api.engine.newDocument(64, 64, false);
    api.engine.importImage(doc, Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0)), "gray", { x: 32, y: 32 });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
    return doc as string;
  }, b64);
}

test("an Add Noise layer's statistics match AdjustPixels.c's noise_add_at, transcribed independently", async ({ page }) => {
  await setupGray(page);
  const GREY = 128;
  const mean = (values: number[]) => values.reduce((a, v) => a + v, 0) / values.length;
  const sd = (values: number[]) => { const m = mean(values); return Math.sqrt(mean(values.map((v) => (v - m) ** 2))); };
  const channelOf = (pixels: number[], c: number) => { const out: number[] = []; for (let i = 0; i < pixels.length / 4; i++) out.push(pixels[i * 4 + c]); return out; };

  // Uniform: amount 40 -> spread 51.0 (amount / 100 * 127.5, NoisePixels.c:23); the delta is
  // uniform on [-spread, spread), whose standard deviation is spread / sqrt(3) (variance of a
  // continuous uniform(-s, s) is (2s)^2 / 12 = s^2 / 3). Tolerance 3 (about 10% of 29.44): over
  // 4096 pixels the standard error of the estimated sd is about spread / sqrt(2 * 4095) =~ 0.33,
  // so 3 is generous slack for byte quantization and the hash's own structure while still catching
  // a materially wrong scale (a missing /100, a missing *127.5, or an off-by-2 spread).
  {
    const id = await addAdjustment(page, "Add Noise", { noiseAmount: 40, noiseGaussian: false, noiseMonochromatic: false, noiseSeed: 999 });
    const gl = await glPixels(page);
    const spread = (40 / 100) * 127.5;
    const expectedSd = spread / Math.sqrt(3);
    for (let c = 0; c < 3; c++) {
      const channel = channelOf(gl, c);
      expect(Math.abs(mean(channel) - GREY), `uniform channel ${c} mean`).toBeLessThanOrEqual(1.5);
      expect(Math.abs(sd(channel) - expectedSd), `uniform channel ${c} sd (expected ${expectedSd.toFixed(2)})`).toBeLessThanOrEqual(3);
    }
    await run(page, { type: "DeleteLayers", ids: [id], bake: false });
  }

  // Gaussian, monochromatic: amount 20 -> spread 25.5, and Box-Muller's normal is scaled by 2/3
  // (NoisePixels.c:39), so its standard deviation is spread * 2/3 = 17.0. A smaller amount than
  // the uniform case keeps 3 standard deviations inside 0..255 around grey 128 (128 +/- 51), so
  // clamping at the channel's ends does not compress the measured spread. Tolerance 2 for the same
  // sampling-error reasoning as above (a smaller expected sd has a smaller absolute standard
  // error too).
  {
    const id = await addAdjustment(page, "Add Noise", { noiseAmount: 20, noiseGaussian: true, noiseMonochromatic: true, noiseSeed: 4242 });
    const gl = await glPixels(page);
    const spread = (20 / 100) * 127.5;
    const expectedSd = spread * (2 / 3);
    for (let c = 0; c < 3; c++) {
      const channel = channelOf(gl, c);
      expect(Math.abs(mean(channel) - GREY), `gaussian channel ${c} mean`).toBeLessThanOrEqual(1.5);
      expect(Math.abs(sd(channel) - expectedSd), `gaussian channel ${c} sd (expected ${expectedSd.toFixed(2)})`).toBeLessThanOrEqual(2);
    }
    // Monochromatic: one key drives every channel, and every input pixel is the same grey, so R,
    // G and B must land on exactly the same byte at every pixel, not just statistically close.
    let maxChannelSpread = 0;
    for (let i = 0; i < gl.length / 4; i++) {
      const off = i * 4;
      maxChannelSpread = Math.max(maxChannelSpread, Math.abs(gl[off] - gl[off + 1]), Math.abs(gl[off + 1] - gl[off + 2]));
    }
    expect(maxChannelSpread, "monochromatic: R, G, B equal at every pixel").toBe(0);
    await run(page, { type: "DeleteLayers", ids: [id], bake: false });
  }
});

/** The sizes the engine gives a blur layer at the current zoom (`spatial_blur` through wasm). */
async function blurOf(page: Page, id: string): Promise<{ level: number; sigma: number; distance: number; angle: number }> {
  return page.evaluate((id) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    const layer = api.engine.state(s.activeId).layers.find((l: any) => l.id === id);
    return api.engine.spatialBlur(layer.adjustment, s.viewports[s.activeId].pointsPerPixel * (window.devicePixelRatio || 1));
  }, id);
}
const blurLevel = async (page: Page, id: string) => (await blurOf(page, id)).level;

test("blur adjustment layers draw on the GPU as on the CPU, reduced or not, dimmed, blended and clipped", async ({ page }) => {
  await setupNoise(page, 721);
  // At one output px per document px: radius 3 reaches 9 px (exact kernel); radius 20 reaches 60,
  // past SPATIAL_REACH_LIMIT 48 (one halving); a 21-px streak reaches 10.5, within
  // MOTION_REACH_LIMIT 12 (exact); a 150-px streak reaches 75, which halves to 37.5, 18.75 and
  // 9.375 (three halvings). A halved case may differ by one more level.
  const cases: [string, object, number][] = [
    ["Gaussian Blur", { blurRadius: 3 }, 0], ["Gaussian Blur", { blurRadius: 20 }, 1],
    ["Motion Blur", { motionAngle: 30, motionDistance: 21 }, 0], ["Motion Blur", { motionAngle: -60, motionDistance: 150 }, 3],
  ];
  for (const [kind, settings, level] of cases) {
    const id = await addAdjustment(page, kind, settings);
    expect(await blurLevel(page, id), `${kind} ${JSON.stringify(settings)}: the engine's level`).toBe(level);
    await expectMatchesCpu(page, `${kind} ${JSON.stringify(settings)}`, level > 0 ? 3 : 2);
    await run(page, { type: "DeleteLayers", ids: [id], bake: false });
  }
  const id = await addAdjustment(page, "Gaussian Blur", { blurRadius: 3 });
  await run(page, { type: "SetLayerOpacity", id, opacity: 0.6 });
  await expectMatchesCpu(page, "opacity");
  await run(page, { type: "SetLayerBlendMode", id, mode: "Multiply" });
  await expectMatchesCpu(page, "blend mode");
  // A Core-Image-only mode: Normal at full coverage with the original alpha kept (keepsAlpha).
  await run(page, { type: "SetLayerBlendMode", id, mode: "Linear Burn" });
  await expectMatchesCpu(page, "Linear Burn");
  await run(page, { type: "SetLayerBlendMode", id, mode: "Normal" });
  await run(page, { type: "ToggleClipping", id });
  await expectMatchesCpu(page, "clipped to the layer below");
});

test("a blur never reads what lies off the canvas, so its edge fades as the Mac's does", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 721 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const red = await page.evaluate(redSquarePngBase64);
  const blurId = await page.evaluate(async (b64) => {
    const api = (window as any).__compositor;
    const doc = api.engine.newDocument(64, 64, false);
    api.engine.importImage(doc, Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0)), "block", { x: 1, y: 1 });
    // An opaque red block from x = -16 to 32: its left third is off the canvas.
    const block = api.engine.state(doc).activeLayerId;
    api.engine.execute(doc, { type: "SetLayerTransform", id: block, transform: { origin: [-16, 0], size: [48, 64], rotation: 0, flipX: false, flipY: false, sampling: "Nearest" } });
    api.engine.execute(doc, { type: "AddAdjustmentLayer", kind: "Gaussian Blur", seed: 0, shadows: null, highlights: null });
    const layer = api.engine.state(doc).layers.find((l: any) => l.adjustment);
    api.engine.execute(doc, { type: "SetAdjustment", id: layer.id, adjustment: { ...layer.adjustment, blurRadius: 4 } });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
    return layer.id as string;
  }, red);
  const gl = await glPixels(page);
  // The CPU kernel, written out: sigma 4, radius 12, over the 32 on-canvas columns; the block spans
  // every row, so at row 32 the vertical pass keeps all of it.
  const weight = (i: number) => Math.exp(-(i * i) / 32);
  let total = 0; for (let i = -12; i <= 12; i++) total += weight(i);
  for (const x of [0, 5, 20, 31, 32, 40]) {
    let covered = 0; for (let i = -12; i <= 12; i++) if (x + i >= 0 && x + i < 32) covered += weight(i);
    const alpha = gl[(32 * 64 + x) * 4 + 3];
    expect(Math.abs(alpha - 255 * covered / total), `alpha at x ${x}: ${alpha}`).toBeLessThanOrEqual(2);
  }
  await expectMatchesCpu(page, "off-canvas block");
  // Linear Burn (keepsAlpha): the original alpha comes back, so the canvas edge does not fade and
  // nothing spreads past the block (LiveMaskRenderer.swift:24-46; Task 4's CPU test).
  await run(page, { type: "SetLayerBlendMode", id: blurId, mode: "Linear Burn" });
  const kept = await glPixels(page);
  for (const x of [0, 31]) expect(kept[(32 * 64 + x) * 4 + 3], `alpha at x ${x}, Linear Burn`).toBe(255);
  expect(kept[(32 * 64 + 40) * 4 + 3], "alpha at x 40, Linear Burn").toBe(0);
  await expectMatchesCpu(page, "off-canvas block, Linear Burn");
  // Clipped to the block, in Multiply: beyond the block the stack surface is opaque black, and the
  // blur spreads it inward (FRAG_OPAQUE, as draw_stack). Multiply, not Screen: over pure red,
  // Screen gives red whether the blur spread black or transparency, and the base alpha put back
  // hides the difference in alpha. The blurred red, made opaque, is the share of the kernel on the
  // block (all 25 taps lie on the canvas at x 20 and 31), and Multiply by red keeps it.
  await run(page, { type: "SetLayerBlendMode", id: blurId, mode: "Multiply" });
  await run(page, { type: "ToggleClipping", id: blurId });
  const clipped = await glPixels(page);
  for (const x of [20, 31]) {
    let onBlock = 0; for (let i = -12; i <= 12; i++) if (x + i < 32) onBlock += weight(i);
    const red = clipped[(32 * 64 + x) * 4];
    expect(Math.abs(red - 255 * onBlock / total), `red at x ${x}, clipped in Multiply: ${red}`).toBeLessThanOrEqual(2);
  }
  await expectMatchesCpu(page, "clipped to the block, in Multiply");
});

test("a blur layer's soft, placed mask limits where the blur lands, as on the CPU", async ({ page }) => {
  // Audit D-I1: AddMask plus BlurMask makes a uniform 1 x 1 mask (ops/masks.rs:15, :46), which a
  // pass that ignored its coverage texture would also pass. A gray ramp placed off-centre and
  // rotated, as blend.spec.ts:89-129 does, varies across the layer and falls back to its
  // background outside the placement. The package is how such a mask arrives (from a Mac file).
  await page.setViewportSize({ width: 1280, height: 721 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const noise = await page.evaluate(hueSafeNoisePngBase64);
  const mask = await page.evaluate(grayRampMaskPngBase64);
  const layer = await page.evaluate(async ({ noise, mask }) => {
    const api = (window as any).__compositor;
    const decode = (b: string) => Uint8Array.from(atob(b), (c: string) => c.charCodeAt(0));
    const [p, b] = ["B2000000-0000-4000-8000-0000000000A1", "B2000000-0000-4000-8000-0000000000A2"];
    const range = { black: 0, gamma: 1, white: 255, outputBlack: 0, outputWhite: 255 };
    const line = [{ x: 0, y: 0 }, { x: 255, y: 255 }];
    const transform = { origin: [0, 0], size: [64, 64], rotation: 0, flipX: false, flipY: false, sampling: "High quality" };
    const manifest = {
      format: "com.compositor.project", version: 9, colorSpace: "sRGB",
      // The adjustment layer is selected on open, the chrome the 721 viewport is pinned for.
      documentID: "B2000000-0000-4000-8000-0000000000D1", width: 64, height: 64, activeLayerID: b,
      layers: [
        { id: p, name: "noise", isVisible: true, imageFile: `${p}.png`, transform },
        { id: b, name: "Gaussian Blur", isVisible: true, transform,
          maskFile: `${b}.mask.png`, maskEnabled: true, maskLinked: false,
          maskPlacement: { origin: [12, 8], size: [36, 44], rotation: 12, flipX: false, flipY: false, sampling: "High quality" },
          adjustment: { kind: "Gaussian Blur", hue: 0, saturation: 0, lightness: 0, colorize: false,
            levels: { channel: "RGB", ranges: [range, range, range, range] },
            curves: { channel: "RGB", channels: [line, line, line, line] }, blurRadius: 3 } },
      ],
    };
    const doc = api.engine.openPackage({ manifest: JSON.stringify(manifest), images: [{ name: `${p}.png`, bytes: decode(noise) }, { name: `${b}.mask.png`, bytes: decode(mask) }] }, null);
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
    return api.engine.state(doc).layers[1];
  }, { noise, mask });
  // The package really carried the 32 x 32 ramp, not a uniform mask silently substituted.
  expect(layer.hasMask).toBe(true);
  expect([layer.maskWidth, layer.maskHeight]).toEqual([32, 32]);
  await expectMatchesCpu(page, "soft placed mask");
});

test("on an odd-sized canvas the GPU halves the same canvas-anchored blocks as the CPU", async ({ page }) => {
  // Audit D-I2. 63 x 61 is a multiple of no cell: both renderers round the far edges out to the
  // lattice with transparency (spatial_span), so a radius-20 blur (one halving) matches over the
  // whole canvas. The noise layer is 64 x 64, so it also reaches past the canvas where the frame's
  // ring must be cleared. 1281 x 722 puts the document on whole device pixels with an adjustment
  // layer selected (canvas area 977 x 641, from the 304 x 81 chrome adjust-render.spec.ts measured).
  await page.setViewportSize({ width: 1281, height: 722 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(hueSafeNoisePngBase64);
  const onWholePixels = await page.evaluate(async (data) => {
    const api = (window as any).__compositor;
    const doc = api.engine.newDocument(63, 61, false);
    api.engine.importImage(doc, Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0)), "noise", { x: 32, y: 32 });
    api.engine.execute(doc, { type: "AddAdjustmentLayer", kind: "Gaussian Blur", seed: 0, shadows: null, highlights: null });
    const layer = api.engine.state(doc).layers.find((l: any) => l.adjustment);
    api.engine.execute(doc, { type: "SetAdjustment", id: layer.id, adjustment: { ...layer.adjustment, blurRadius: 20 } });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
    // Read the placement once the view has taken its settled size (setZoom waits one frame only).
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const s = api.store.getState();
    const rect = s.viewports[s.activeId].documentRect({ width: 63, height: 61 });
    const dpr = window.devicePixelRatio || 1;
    return Number.isInteger(rect.x * dpr) && Number.isInteger(rect.y * dpr);
  }, b64);
  expect(onWholePixels, "the document sits on whole device pixels").toBe(true);
  await expectMatchesCpu(page, "63 x 61, radius 20", 3);
});

test("zoomed in, a blur near the window's edge still sees the canvas beyond it", async ({ page }) => {
  test.setTimeout(90_000);
  // 800 x 601 with the adjustment layer selected leaves a 496 x 520 canvas area (the 304 x 81
  // chrome adjust-render.spec.ts measured), which puts the 640 x 640 zoomed document at (-72, -60):
  // whole device pixels. The test asserts that rather than trusting it (LL-065(6)).
  await page.setViewportSize({ width: 800, height: 601 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(hueSafeNoisePngBase64);
  await page.evaluate(async (data) => {
    const api = (window as any).__compositor;
    const doc = api.engine.newDocument(64, 64, false);
    api.engine.importImage(doc, Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0)), "noise", { x: 32, y: 32 });
    api.engine.execute(doc, { type: "AddAdjustmentLayer", kind: "Gaussian Blur", seed: 0, shadows: null, highlights: null });
    const layer = api.engine.state(doc).layers.find((l: any) => l.adjustment);
    api.engine.execute(doc, { type: "SetAdjustment", id: layer.id, adjustment: { ...layer.adjustment, blurRadius: 1 } });
    api.store.getState().openDocument(doc);
    await api.setZoom(10);
    api.setCheckerboard(false);
  }, b64);
  const r = await page.evaluate(async () => {
    const api = (window as any).__compositor; const s = api.store.getState();
    const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const dpr = window.devicePixelRatio || 1;
    const W = Math.round(vp.viewSize.width * dpr), H = Math.round(vp.viewSize.height * dpr);
    const gl = Array.from(api.renderer.readPixels()) as number[];
    const rect = vp.documentRect({ width: d.width, height: d.height }); const ppp = vp.pointsPerPixel;
    // The doc region under the whole window, at device resolution: the CPU pads it itself.
    const region = { x: -rect.x / ppp, y: -rect.y / ppp, width: W / (dpr * ppp), height: H / (dpr * ppp) };
    const cpu = Array.from(api.engine.compositeEdit(d.id, null, region, W, H)) as number[];
    const covers = rect.x < 0 && rect.y < 0 && rect.x + rect.width > vp.viewSize.width && rect.y + rect.height > vp.viewSize.height;
    return { gl, cpu, covers, kind: s.rendererKind, whole: Number.isInteger(rect.x * dpr) && Number.isInteger(rect.y * dpr) };
  });
  expect(r.kind).toBe("gl");
  expect(r.whole, "the document sits on whole device pixels").toBe(true);
  expect(r.covers, "the document covers the whole window, so every pixel is inside the canvas").toBe(true);
  // An unpadded frame loses the blur's reach (30 device px) along every window edge.
  expect(worstOf(r.gl, r.cpu)).toBeLessThanOrEqual(3);
});
test("a step edge on the lattice blurs on the GPU to the closed form, halved or not", async ({ page }) => {
  test.setTimeout(90_000);
  // Review I1: every case above but the level-0 Gaussian is parity only, and both renderers read
  // the same spatialBlur sizes. An opaque red block over x < 128 of a 256 x 192 canvas puts its edge
  // on every lattice up to a cell of 128, so the halved copy is an exact step and the whole path
  // (reduce, blur, enlarge) has a closed form. 1280 x 721 with an adjustment layer selected leaves
  // a 976 x 640 canvas area: the document at (360, 224), asserted below (LL-065(6)).
  await page.setViewportSize({ width: 1280, height: 721 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const red = await page.evaluate(redSquarePngBase64);
  await page.evaluate(async (b64) => {
    const api = (window as any).__compositor;
    const doc = api.engine.newDocument(256, 192, false);
    api.engine.importImage(doc, Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0)), "block", { x: 1, y: 1 });
    const block = api.engine.state(doc).activeLayerId;
    // From x = -16 to 128 and past every row: only the edge at x = 128 lies on the canvas.
    api.engine.execute(doc, { type: "SetLayerTransform", id: block, transform: { origin: [-16, -16], size: [144, 224], rotation: 0, flipX: false, flipY: false, sampling: "Nearest" } });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
  }, red);
  const EDGE = 128, ROW = 96;
  /** spatial.rs `enlarged`: the reduced row sampled bilinearly at output column x's centre. */
  const enlarged = (b: (k: number) => number, level: number, x: number) => {
    const q = (x + 0.5) / 2 ** level - 0.5, k = Math.floor(q), t = q - k;
    return b(k) + (b(k + 1) - b(k)) * t;
  };
  const cases: [string, object, number][] = [
    ["Gaussian Blur", { blurRadius: 20 }, 1],
    ["Motion Blur", { motionAngle: 0, motionDistance: 150 }, 3],
    ["Motion Blur", { motionAngle: 0, motionDistance: 21 }, 0],
  ];
  for (const [kind, settings, level] of cases) {
    const label = `${kind} ${JSON.stringify(settings)}`;
    const id = await addAdjustment(page, kind, settings);
    const blur = await blurOf(page, id);
    expect(blur.level, `${label}: the engine's level`).toBe(level);
    const f = 2 ** blur.level, edge = EDGE / f;   // the step's column in the reduced copy
    let b: (k: number) => number;
    if (kind === "Gaussian Blur") {
      // gaussian_blur at the reduced sigma: the share of the kernel's weight on the block. Row 96
      // lies beyond every vertical reach from the canvas's top and bottom, so that pass keeps it.
      const s = blur.sigma / f, radius = Math.ceil(s * 3);
      const w = (j: number) => Math.exp(-(j * j) / (2 * s * s));
      let total = 0; for (let j = -radius; j <= radius; j++) total += w(j);
      b = (k) => { let on = 0; for (let j = -radius; j <= radius; j++) if (k + j >= 0 && k + j < edge) on += w(j); return 255 * on / total; };
    } else {
      // motion_blur at angle 0 over the reduced copy: an odd count of steps lands every sample on a
      // texel centre, so each column is the share of the streak's samples on the block.
      const steps = Math.max(1, Math.round(blur.distance / f)), mid = (steps - 1) / 2;
      expect(steps % 2, `${label}: an odd streak of ${steps}`).toBe(1);
      b = (k) => { let on = 0; for (let i = 0; i < steps; i++) { const c = k + i - mid; if (c >= 0 && c < edge) on++; } return 255 * on / steps; };
    }
    const r = await page.evaluate(async () => {
      const api = (window as any).__compositor; const s = api.store.getState();
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const rect = s.viewports[s.activeId].documentRect({ width: 256, height: 192 }); const dpr = window.devicePixelRatio || 1;
      return { gl: Array.from(api.readDocumentPixels()) as number[], whole: Number.isInteger(rect.x * dpr) && Number.isInteger(rect.y * dpr) };
    });
    expect(r.whole, `${label}: the document sits on whole device pixels`).toBe(true);
    const alpha = (x: number) => r.gl[(ROW * 256 + x) * 4 + 3];
    const tolerance = blur.level > 0 ? 3 : 2;
    for (const x of [120, 127, 128, 135]) {
      const want = enlarged(b, blur.level, x);
      expect(Math.abs(alpha(x) - want), `${label}: alpha at x ${x} is ${alpha(x)}, want ${want.toFixed(2)}`).toBeLessThanOrEqual(tolerance);
    }
    // Symmetric about the edge on the lattice: the two columns beside it sum to opaque.
    expect(Math.abs(alpha(127) + alpha(128) - 255), `${label}: alpha at 127 + 128`).toBeLessThanOrEqual(tolerance);
    await expectMatchesCpu(page, label, tolerance);
    await run(page, { type: "DeleteLayers", ids: [id], bake: false });
  }
});

test("Layer > New Adjustment lists all twelve kinds in the Mac's order, with no ellipsis for Invert", async ({ page }) => {
  await setupNoise(page, 720);
  await page.getByRole("button", { name: "Layer", exact: true }).click();
  expect(await page.locator("[data-testid^='menu-layer-adjustment-']").allTextContents()).toEqual([
    "New Hue/Saturation Adjustment...", "New Levels Adjustment...", "New Curves Adjustment...", "New Exposure Adjustment...",
    "New Gradient Map Adjustment...", "New Grain Adjustment...", "New Add Noise Adjustment...", "New Gaussian Blur Adjustment...",
    "New Motion Blur Adjustment...", "New Invert Adjustment", "New Black & White Adjustment...", "New Color Balance Adjustment...",
  ]);
});

test("a new Black & White layer opens its panel, previews live, and OK records one step", async ({ page }) => {
  await setupNoise(page, 720);
  await clickMenu(page, "Layer", "layer-adjustment-black-white");
  await expect(page.getByTestId("adjust-title")).toHaveText("Black & White");
  const depth = (await state(page)).undoDepth;   // the New Black & White Adjustment step is already in
  const before = await glPixels(page);
  await expect(field(page, "Reds")).toHaveValue("40");
  await field(page, "Reds").fill("150");
  expect(await glPixels(page), "the canvas shows the edit before OK").not.toEqual(before);
  expect((await state(page)).undoDepth).toBe(depth);
  await expect(field(page, "Hue")).toHaveCount(0);
  await page.getByRole("checkbox", { name: "Tint", exact: true }).check();
  await expect(field(page, "Hue")).toHaveValue("40");
  await page.getByTestId("adjust-ok").click();
  const d = await state(page);
  expect(d.undoDepth).toBe(depth + 1);
  const bw = d.layers.find((l: any) => l.adjustment?.kind === "Black & White").adjustment.blackWhiteSettings;
  expect([bw.reds, bw.yellows, bw.tint]).toEqual([150, 60, true]);
});

test("Color Balance names each field by its tone, and Preserve Luminosity is a checkbox", async ({ page }) => {
  await setupNoise(page, 720);
  await clickMenu(page, "Layer", "layer-adjustment-color-balance");
  for (const tone of ["Shadows", "Midtones", "Highlights"]) for (const pair of ["Cyan / Red", "Magenta / Green", "Yellow / Blue"]) {
    await expect(field(page, `${tone} ${pair}`)).toHaveValue("0");
  }
  const depth = (await state(page)).undoDepth;
  await field(page, "Midtones Yellow / Blue").fill("-40");
  await page.getByRole("checkbox", { name: "Preserve Luminosity", exact: true }).uncheck();
  await page.getByTestId("adjust-ok").click();
  const d = await state(page);
  expect(d.undoDepth).toBe(depth + 1);
  const cb = d.layers.find((l: any) => l.adjustment?.kind === "Color Balance").adjustment.colorBalanceSettings;
  expect(cb).toMatchObject({ midYellowBlue: -40, shadowCyanRed: 0, highlightYellowBlue: 0, preserveLuminosity: false });
});

test("the blur and noise editors show the Mac's defaults, preview live, and OK records one step writing their own keys", async ({ page }) => {
  await setupNoise(page, 720);
  const layerOf = async (kind: string) => (await state(page)).layers.find((l: any) => l.adjustment?.kind === kind).adjustment;
  // The New ... Adjustment step is already in when the panel opens. The edit must reach the canvas
  // before OK (for the blurs, the spatial `PreviewEdit::Adjustment` path), record nothing, and OK
  // must record exactly one step.
  const previewsThenCommits = async (kind: string, edit: () => Promise<void>) => {
    const depth = (await state(page)).undoDepth;
    const before = await glPixels(page);
    await edit();
    expect(await glPixels(page), `${kind}: the canvas shows the edit before OK`).not.toEqual(before);
    expect((await state(page)).undoDepth, `${kind}: nothing recorded before OK`).toBe(depth);
    await page.getByTestId("adjust-ok").click();
    expect((await state(page)).undoDepth, `${kind}: OK records one step`).toBe(depth + 1);
  };
  await clickMenu(page, "Layer", "layer-adjustment-gaussian-blur");
  await expect(field(page, "Radius")).toHaveValue("10");
  await previewsThenCommits("Gaussian Blur", () => field(page, "Radius").fill("3.5"));
  expect((await layerOf("Gaussian Blur")).blurRadius).toBe(3.5);
  await clickMenu(page, "Layer", "layer-adjustment-motion-blur");
  await expect(field(page, "Angle")).toHaveValue("0");
  await expect(field(page, "Distance")).toHaveValue("10");
  await previewsThenCommits("Motion Blur", async () => {
    await field(page, "Angle").fill("-35");
    await field(page, "Distance").fill("48");
  });
  expect(await layerOf("Motion Blur")).toMatchObject({ motionAngle: -35, motionDistance: 48 });
  await clickMenu(page, "Layer", "layer-adjustment-add-noise");
  const seed = (await layerOf("Add Noise")).noiseSeed;
  expect(typeof seed).toBe("number");
  await expect(field(page, "Amount")).toHaveValue("10");
  await previewsThenCommits("Add Noise", async () => {
    await field(page, "Amount").fill("33.5");
    await page.getByRole("combobox", { name: "Distribution", exact: true }).selectOption("Gaussian");
    await page.getByRole("checkbox", { name: "Monochromatic", exact: true }).check();
  });
  expect(await layerOf("Add Noise")).toMatchObject({ noiseAmount: 33.5, noiseGaussian: true, noiseMonochromatic: true, noiseSeed: seed });
});

test("a new Invert layer applies at once, opens nothing, and cannot be edited", async ({ page }) => {
  await setupNoise(page, 720);
  const depth = (await state(page)).undoDepth;
  await clickMenu(page, "Layer", "layer-adjustment-invert");
  await expect(page.getByTestId("adjust-panel")).toHaveCount(0);
  expect((await state(page)).undoDepth).toBe(depth + 1);
  await page.getByRole("button", { name: "Layer", exact: true }).click();
  await expect(page.getByTestId("menu-layer-edit-adjustment")).toBeDisabled();
});

test("Cancel on an edited Black & White layer puts its settings back and records nothing", async ({ page }) => {
  await setupNoise(page, 720);
  await clickMenu(page, "Layer", "layer-adjustment-black-white");
  await field(page, "Greens").fill("-120");
  await page.getByTestId("adjust-ok").click();
  const d0 = await state(page);
  await page.getByTestId("layer-row").nth(0).dblclick();
  await expect(page.getByTestId("adjust-title")).toHaveText("Black & White");
  await field(page, "Greens").fill("250");
  await page.getByTestId("adjust-cancel").click();
  const d = await state(page);
  expect(d.undoDepth).toBe(d0.undoDepth);
  expect(d.layers.find((l: any) => l.adjustment).adjustment.blackWhiteSettings.greens).toBe(-120);
});

test("Image > Black & White and Color Balance preview on the layer and OK changes its own pixels, one step each", async ({ page }) => {
  const doc = await setupNoise(page, 720);
  // The layer's pixels as the engine renders them (a preview substituted while one is open,
  // engine.rs:345-347), read here only before the panel opens and after OK.
  const stored = () => page.evaluate((doc) => {
    const api = (window as any).__compositor;
    const id = api.engine.state(doc).layers[0].id;
    return Array.from(api.engine.layerPixels(doc, id, 0)) as number[];
  }, doc);
  for (const [id, title, name, value] of [["image-black-white", "Black & White", "Reds", "150"], ["image-color-balance", "Color Balance", "Midtones Yellow / Blue", "-40"]] as const) {
    const depth = (await state(page)).undoDepth;
    const before = await stored();
    const shown = await glPixels(page);
    await clickMenu(page, "Image", id);
    await expect(page.getByTestId("adjust-title")).toHaveText(title);
    await field(page, name).fill(value);
    await expect.poll(() => glPixels(page), { message: `${title}: the canvas previews the edit` }).not.toEqual(shown);
    expect((await state(page)).undoDepth, `${title}: nothing recorded before OK`).toBe(depth);
    await page.getByTestId("adjust-ok").click();
    await expect(page.getByTestId("adjust-panel")).toHaveCount(0);
    expect((await state(page)).undoDepth, `${title}: OK records one step`).toBe(depth + 1);
    expect(await stored(), `${title}: the layer's own pixels changed`).not.toEqual(before);
  }
});
