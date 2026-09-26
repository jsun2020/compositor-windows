import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { test, expect, type Page } from "@playwright/test";
import { grayRampMaskPngBase64, hueSafeNoisePngBase64, softBlobPngBase64, solidPngBase64 } from "./helpers";

// Layer effects (Phase 3.5c). The engine makes one image per styled layer and both renderers
// draw it: the CPU compositor samples it, the GPU uploads it as the layer's texture.

const PROBES = path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "engine", "tests", "fixtures", "mac-1.2.10-probes");

const DOC = "0B6C6B1E-4F1B-4B4E-9E0A-EFEFEFEFEF00";
const BACK = "0B6C6B1E-4F1B-4B4E-9E0A-EFEFEFEFEF01";
const STYLED = "0B6C6B1E-4F1B-4B4E-9E0A-EFEFEFEFEF02";
const CHILD = "0B6C6B1E-4F1B-4B4E-9E0A-EFEFEFEFEF03";
const BETWEEN = "0B6C6B1E-4F1B-4B4E-9E0A-EFEFEFEFEF04";

type Json = Record<string, unknown>;
const transform = (x: number, y: number, w: number, h: number, extra: Json = {}) =>
  ({ origin: [x, y], size: [w, h], rotation: 0, flipX: false, flipY: false, sampling: "High quality", ...extra });

/** All six, none at its defaults, both angles off the axes. */
const ALL_SIX = {
  stroke: { blue: 0.2, green: 0.8, inside: false, opacity: 0.9, red: 0.1, size: 3 },
  shadow: { angle: 120, blue: 0.3, blur: 6, distance: 7, green: 0.2, opacity: 0.75, red: 0.2 },
  colorOverlay: { blue: 0.4, green: 0.1, opacity: 0.3, red: 0.9 },
  innerShadow: { angle: -35, blue: 0.05, blur: 3, distance: 4, green: 0.05, opacity: 0.6, red: 0.05 },
  outerGlow: { blue: 0.2, green: 0.9, opacity: 0.6, red: 1, size: 5 },
  innerGlow: { blue: 1, green: 1, opacity: 0.5, red: 1, size: 4 },
};
/** `LayerEffectsRenderer.margin` for ALL_SIX: the outside stroke, the shadow's distance plus three
 * blurs, three glow sizes, the largest rounded up, plus 2. */
const ALL_SIX_MARGIN = Math.ceil(Math.max(3, 7 + 3 * 6, 3 * 5)) + 2;

/** A 96 x 72 project: a noise backdrop at (16, 4), 1:1, and the 40 x 24 soft blob at (28, 24) with
 * `styled` merged into its record, then `extra` layers; opened at `zoom` with the checkerboard off
 * and the backdrop (a pixel layer) selected, on the 1280 x 720 viewport. The chrome
 * adjust-render.spec.ts measured (304 wide, 102 tall with a pixel layer selected) puts every zoom
 * used here on whole device pixels; the test checks that rather than trusting it (LL-065(6)). */
async function openStyled(page: Page, styled: Json, extra: Json[] = [], zoom = 1, images: Record<string, string> = {}): Promise<void> {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const noise = await page.evaluate(hueSafeNoisePngBase64);
  const blob = await page.evaluate(softBlobPngBase64);
  const manifest = {
    format: "com.compositor.project", version: 9, colorSpace: "sRGB", documentID: DOC, width: 96, height: 72,
    layers: [
      { id: BACK, name: "Noise", isVisible: true, imageFile: `${BACK}.png`, transform: transform(16, 4, 64, 64) },
      { id: STYLED, name: "Styled", isVisible: true, imageFile: `${STYLED}.png`, transform: transform(28, 24, 40, 24), ...styled },
      ...extra,
    ],
  };
  const files: Record<string, string> = { [`${BACK}.png`]: noise, [`${STYLED}.png`]: blob, [`${CHILD}.png`]: noise, ...images };
  const whole = await page.evaluate(async ({ manifest, files, zoom }) => {
    const api = (window as any).__compositor;
    const used = new Set((manifest.layers as any[]).flatMap((l) => [l.imageFile, l.maskFile].filter(Boolean)));
    const images = Object.entries(files).filter(([name]) => used.has(name))
      .map(([name, b64]) => ({ name, bytes: Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0)) }));
    const doc = api.engine.openPackage({ manifest: JSON.stringify(manifest), images }, null);
    api.store.getState().openDocument(doc);
    await api.setZoom(zoom);
    api.setCheckerboard(false);
    const back = api.engine.state(doc).layers[0].id;
    api.store.getState().selectLayers([back], back);
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const s = api.store.getState(); const d = s.documents[s.activeId]; const dpr = window.devicePixelRatio || 1;
    const rect = s.viewports[s.activeId].documentRect({ width: d.width, height: d.height });
    const onPixel = (v: number) => Math.abs(v - Math.round(v)) < 1e-9;
    return onPixel(rect.x * dpr) && onPixel(rect.y * dpr);
  }, { manifest, files, zoom });
  expect(whole, "the document sits on whole device pixels").toBe(true);
}

/** The GPU's document pixels against the CPU compositor at the same output size (zoom-render.spec.ts). */
async function expectMatchesCpuAtZoom(page: Page, label: string, tolerance = 2) {
  const r = await page.evaluate(async () => {
    const api = (window as any).__compositor;
    const s = api.store.getState(); const d = s.documents[s.activeId];
    const vp = s.viewports[s.activeId]; const dpr = window.devicePixelRatio || 1;
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const gl = Array.from(api.readDocumentPixels()) as number[];
    const rect = vp.documentRect({ width: d.width, height: d.height });
    const w = Math.round((rect.x + rect.width) * dpr) - Math.round(rect.x * dpr);
    const h = Math.round((rect.y + rect.height) * dpr) - Math.round(rect.y * dpr);
    const cpu = Array.from(api.engine.compositeEdit(d.id, s.previewEdit(), { x: 0, y: 0, width: d.width, height: d.height }, w, h)) as number[];
    return { gl, cpu, kind: s.rendererKind, w, h };
  });
  expect(r.kind, "the GPU renderer, not the CPU fallback").toBe("gl");
  expect(r.gl.length, `${label} output size ${r.w}x${r.h}`).toBe(r.cpu.length);
  const worst = r.gl.reduce((m, v, i) => Math.max(m, Math.abs(v - r.cpu[i])), 0);
  expect(worst, `${label} (${r.w}x${r.h}) max byte diff`).toBeLessThanOrEqual(tolerance);
}

/** The draw of the styled layer in the plan's plain nodes. */
function styledDraw(page: Page): Promise<any> {
  return page.evaluate((id) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    return api.engine.renderPlan(s.activeId, null).nodes.map((n: any) => n.draw).find((d: any) => d && d.id === id);
  }, STYLED);
}

test("the plan draws a styled layer from the engine's padded effects image", async ({ page }) => {
  await openStyled(page, { effects: ALL_SIX });
  const m = ALL_SIX_MARGIN;
  const draw = await styledDraw(page);
  expect(draw.effects.inset).toBe(m);
  expect([draw.pixelsWidth, draw.pixelsHeight]).toEqual([40 + 2 * m, 24 + 2 * m]);
  expect([draw.transform.origin, draw.transform.size], "LayerEffectsRenderer.placed: grown about the same centre")
    .toEqual([[28 - m, 24 - m], [40 + 2 * m, 24 + 2 * m]]);
  const lengths = await page.evaluate((id) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    return [api.engine.drawPixels(s.activeId, id, 0, null).length, api.engine.layerPixels(s.activeId, id, 0).length];
  }, STYLED);
  expect(lengths, "drawPixels: the padded image; layerPixels: the layer's own").toEqual([(40 + 2 * m) * (24 + 2 * m) * 4, 40 * 24 * 4]);
  await page.evaluate((id) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    const t = api.engine.state(s.activeId).layers.find((l: any) => l.id === id).transform;
    api.engine.execute(s.activeId, { type: "SetLayerTransform", id, transform: { ...t, origin: [t.origin[0] + 5, t.origin[1] + 2] } });
  }, STYLED);
  expect((await styledDraw(page)).effects.key, "moving the layer keeps its image (and its texture)").toBe(draw.effects.key);
  await page.evaluate((id) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    api.engine.execute(s.activeId, { type: "InvertPixels", id, mask: false });
  }, STYLED);
  expect((await styledDraw(page)).effects.key, "new pixels, a new image").not.toBe(draw.effects.key);
});

test("every effect draws on the GPU as on the CPU, at 1:1 and reduced", async ({ page }) => {
  await openStyled(page, { effects: ALL_SIX, blendMode: "Multiply", opacity: 0.7 });
  await expectMatchesCpuAtZoom(page, "all six at zoom 1");
  for (const zoom of [0.5, 0.25]) {
    await page.evaluate(async (zoom) => { await (window as any).__compositor.setZoom(zoom); }, zoom);
    await expectMatchesCpuAtZoom(page, `all six at zoom ${zoom}`);
  }
});

test("a turned, flipped and masked layer's effects draw on the GPU as on the CPU", async ({ page }) => {
  const mask = await page.evaluate(grayRampMaskPngBase64);
  await openStyled(page, { effects: ALL_SIX, transform: transform(28, 24, 40, 24, { rotation: 30, flipY: true }), maskFile: `${STYLED}.mask.png` },
    [], 1, { [`${STYLED}.mask.png`]: mask });
  await expectMatchesCpuAtZoom(page, "turned 30, flipped, ramp mask");
  // The same pixels without the mask: a new image, which the GPU must upload although the
  // layer's pixel revision has not moved.
  await page.evaluate((id) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    api.engine.execute(s.activeId, { type: "SetMaskEnabled", id, enabled: false }); s.refresh(); s.invalidate();
  }, STYLED);
  await expectMatchesCpuAtZoom(page, "the same with its mask switched off");
});

test("a clipping base's effects are its clipped layer's coverage on the GPU as on the CPU", async ({ page }) => {
  await openStyled(page, { effects: { stroke: ALL_SIX.stroke, shadow: ALL_SIX.shadow } },
    [{ id: CHILD, name: "Clipped", isVisible: true, imageFile: `${CHILD}.png`, transform: transform(10, 10, 64, 64), maskSourceID: STYLED }]);
  await expectMatchesCpuAtZoom(page, "stroke and shadow as a clipping base");
});

test("a layer clipped to a styled layer further down shows through its effects on the GPU as on the CPU", async ({ page }) => {
  // A layer between them, so the clipped layer is drawn on its own through the GPU's clip path,
  // not in a stack (effects_draw.rs has the same document's absolute pixels).
  const between = await page.evaluate(solidPngBase64, { width: 6, height: 6, color: "#c8c800" });
  await openStyled(page, { effects: { stroke: ALL_SIX.stroke, shadow: ALL_SIX.shadow } },
    [{ id: BETWEEN, name: "Between", isVisible: true, imageFile: `${BETWEEN}.png`, transform: transform(84, 2, 6, 6) },
     { id: CHILD, name: "Clipped", isVisible: true, imageFile: `${CHILD}.png`, transform: transform(10, 10, 64, 64), maskSourceID: STYLED }],
    1, { [`${BETWEEN}.png`]: between });
  const plan = await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState();
    return api.engine.renderPlan(s.activeId, null).nodes.map((n: any) => n.kind);
  });
  expect(plan, "no stack: every layer on its own").toEqual(["layer", "layer", "layer", "layer"]);
  await expectMatchesCpuAtZoom(page, "stroke and shadow as the clipping source of a layer further up");
});

/** The document's device rect starts on whole device pixels (LL-065(6)): checked, not assumed. */
async function expectOnWholePixels(page: Page): Promise<void> {
  const whole = await page.evaluate(async () => {
    const api = (window as any).__compositor;
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const s = api.store.getState(); const d = s.documents[s.activeId]; const dpr = window.devicePixelRatio || 1;
    const rect = s.viewports[s.activeId].documentRect({ width: d.width, height: d.height });
    const onPixel = (v: number) => Math.abs(v - Math.round(v)) < 1e-9;
    return onPixel(rect.x * dpr) && onPixel(rect.y * dpr);
  });
  expect(whole, "the document sits on whole device pixels").toBe(true);
}

test("the Mac's own export check holds on the GPU: a 6 px green stroke round a red square", async ({ page }) => {
  // ProjectTests.swift:270-311: a 20 x 20 red square at (20, 20) on 60 x 60, a 6 px green stroke
  // outside at opacity 1, the square (a pixel layer) selected.
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const red = await page.evaluate(solidPngBase64, { width: 20, height: 20, color: "#ff0000" });
  const manifest = {
    format: "com.compositor.project", version: 9, colorSpace: "sRGB", documentID: DOC, width: 60, height: 60,
    layers: [{ id: STYLED, name: "Square", isVisible: true, imageFile: `${STYLED}.png`, transform: transform(20, 20, 20, 20),
      effects: { stroke: { blue: 0, green: 1, inside: false, opacity: 1, red: 0, size: 6 } } }],
  };
  await page.evaluate(async ({ manifest, red, name }) => {
    const api = (window as any).__compositor;
    const doc = api.engine.openPackage({ manifest: JSON.stringify(manifest), images: [{ name, bytes: Uint8Array.from(atob(red), (c: string) => c.charCodeAt(0)) }] }, null);
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
    const square = api.engine.state(doc).layers[0].id;
    api.store.getState().selectLayers([square], square);
  }, { manifest, red, name: `${STYLED}.png` });
  await expectOnWholePixels(page);
  const px = await page.evaluate(() => {
    const api = (window as any).__compositor;
    const all = Array.from(api.readDocumentPixels()) as number[];
    const at = (x: number, y: number) => all.slice((y * 60 + x) * 4, (y * 60 + x) * 4 + 4);
    return { stroke: at(15, 30), past: at(13, 30), square: at(30, 30), kind: api.store.getState().rendererKind };
  });
  expect(px.kind).toBe("gl");
  expect(px.stroke, "5 px left of the square").toEqual([0, 255, 0, 255]);
  expect(px.past, "7 px left, past the stroke").toEqual([0, 0, 0, 0]);
  expect(px.square).toEqual([255, 0, 0, 255]);
});

/** Opens a committed Mac probe at zoom 1 with its first pixel layer selected, on whole device pixels. */
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
    // A pixel layer, by kind: edited-rich-file's first layer is a folder, whose selection shows other chrome.
    const pixel = api.engine.state(doc).layers.find((l: any) => l.hasPixels).id;
    api.store.getState().selectLayers([pixel], pixel);
  }, { manifest, images });
  await expectOnWholePixels(page);
}

test("the edited-rich-file probe's two drop shadows draw on the GPU as the Mac exported them", async ({ page }) => {
  await openProbe(page, "edited-rich-file");
  const b64 = fs.readFileSync(path.join(PROBES, "edited-rich-file.mac-1.2.10.png")).toString("base64");
  const r = await page.evaluate(async (b64) => {
    const api = (window as any).__compositor;
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const gl = Array.from(api.readDocumentPixels()) as number[];
    const doc = api.engine.importImage(null, Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0)), "mac", null);
    const d = api.engine.state(doc);
    const mac = Array.from(api.engine.composite(doc, { x: 0, y: 0, width: d.width, height: d.height }, d.width, d.height)) as number[];
    api.engine.closeDocument(doc);
    return { gl, mac };
  }, b64);
  expect(r.gl.length).toBe(r.mac.length);
  let colour = 0, alpha = 0;
  r.gl.forEach((v, i) => { const d = Math.abs(v - r.mac[i]); if (i % 4 === 3) alpha = Math.max(alpha, d); else colour = Math.max(colour, d); });
  // Measured 2026-09-26 (a scratch run of this spec): colour 3, alpha 4 against the Mac. The CPU
  // is 1 and 3 (mac_1_2_10.rs); the GPU adds the radius-24 blur layer's halved-path parity (3).
  expect(colour, "colour").toBeLessThanOrEqual(3);
  expect(alpha, "alpha").toBeLessThanOrEqual(4);
  await expectMatchesCpuAtZoom(page, "edited-rich-file (a blur layer at level 1)", 3);
});

/** What the GPU shows of the document, after two frames. */
function gpuFrame(page: Page): Promise<number[]> {
  return page.evaluate(async () => {
    const api = (window as any).__compositor;
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    return Array.from(api.readDocumentPixels()) as number[];
  });
}

/** The engine's history depth for the active document: a commit is proved against a baseline. */
function undoDepth(page: Page): Promise<number> {
  return page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].undoDepth; });
}

test("a styled layer's texture is uploaded again for a pixel edit, a mask edit, an undo and a redo", async ({ page }) => {
  // The effects image changes with the pixels and the mask. The texture must follow each change,
  // including an undo, which brings back an earlier pixel revision rather than a newer one.
  const mask = await page.evaluate(grayRampMaskPngBase64);
  await openStyled(page, { effects: ALL_SIX, maskFile: `${STYLED}.mask.png` }, [], 1, { [`${STYLED}.mask.png`]: mask });
  const opened = await gpuFrame(page);
  await expectMatchesCpuAtZoom(page, "as opened");
  const depth = await undoDepth(page);
  const run = (mask: boolean) => page.evaluate(({ id, mask }) => (window as any).__compositor.store.getState().run({ type: "InvertPixels", id, mask }), { id: STYLED, mask });

  expect(await run(false), "the pixel edit is accepted").toBe(true);
  expect(await undoDepth(page), "the pixel edit is one history step").toBe(depth + 1);
  await expectMatchesCpuAtZoom(page, "after a pixel edit");
  const edited = await gpuFrame(page);
  expect(edited, "the pixel edit shows").not.toEqual(opened);

  await page.evaluate(() => (window as any).__compositor.store.getState().undo());
  expect(await undoDepth(page), "undone").toBe(depth);
  await expectMatchesCpuAtZoom(page, "after undoing the pixel edit");
  expect(await gpuFrame(page), "undo shows the very frame the document opened with").toEqual(opened);

  await page.evaluate(() => (window as any).__compositor.store.getState().redo());
  expect(await undoDepth(page), "redone").toBe(depth + 1);
  await expectMatchesCpuAtZoom(page, "after redoing the pixel edit");
  expect(await gpuFrame(page), "redo shows the edited frame again").toEqual(edited);

  expect(await run(true), "the mask edit is accepted").toBe(true);
  expect(await undoDepth(page), "the mask edit is one history step").toBe(depth + 2);
  await expectMatchesCpuAtZoom(page, "after a mask edit");
  expect(await gpuFrame(page), "the mask edit shows").not.toEqual(edited);

  await page.evaluate(() => (window as any).__compositor.store.getState().undo());
  await expectMatchesCpuAtZoom(page, "after undoing the mask edit");
  expect(await gpuFrame(page), "undoing the mask edit shows the edited frame again").toEqual(edited);
});

test("a panel preview of a styled layer is drawn from its own effects image, and cancelling it restores the committed one", async ({ page }) => {
  await openStyled(page, { effects: ALL_SIX });
  const opened = await gpuFrame(page);
  const began = await page.evaluate((id) => {
    const api = (window as any).__compositor;
    if (!api.store.getState().beginAdjust({ kind: "Curves", layerId: id })) return false;
    const next = JSON.parse(JSON.stringify(api.store.getState().adjustEdit.adjustment));
    next.curves.channels[0] = [{ x: 0, y: 40 }, { x: 128, y: 200 }, { x: 255, y: 250 }];
    api.store.getState().updateAdjust({ adjustment: next });
    return true;
  }, STYLED);
  expect(began, "the Curves panel opens on the styled layer").toBe(true);
  await expect.poll(() => page.evaluate(() => (window as any).__compositor.store.getState().previewSettling()), { message: "the full-size preview settles" }).toBe(false);
  await expectOnWholePixels(page);
  await expectMatchesCpuAtZoom(page, "a Curves preview of the styled layer");
  expect(await gpuFrame(page), "the preview shows").not.toEqual(opened);
  await page.evaluate(() => (window as any).__compositor.store.getState().cancelAdjust());
  await expectMatchesCpuAtZoom(page, "the preview cancelled");
  expect(await gpuFrame(page), "cancelling shows the very frame the document opened with").toEqual(opened);
});
