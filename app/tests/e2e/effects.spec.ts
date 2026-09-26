import { test, expect, type Page } from "@playwright/test";
import { hueSafeNoisePngBase64, softBlobPngBase64 } from "./helpers";

// Layer effects (Phase 3.5c). The engine makes one image per styled layer and both renderers
// draw it: the CPU compositor samples it, the GPU uploads it as the layer's texture.

const DOC = "0B6C6B1E-4F1B-4B4E-9E0A-EFEFEFEFEF00";
const BACK = "0B6C6B1E-4F1B-4B4E-9E0A-EFEFEFEFEF01";
const STYLED = "0B6C6B1E-4F1B-4B4E-9E0A-EFEFEFEFEF02";
const CHILD = "0B6C6B1E-4F1B-4B4E-9E0A-EFEFEFEFEF03";

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
