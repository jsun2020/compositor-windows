import { test, expect, type Page } from "@playwright/test";
import { redSquarePngBase64, ringPngBase64, noisePngBase64, sparseAlphaPngBase64 } from "./helpers";

// Same viewport pinning as blend.spec.ts: it keeps the document rect on an integer device
// pixel, so the comparison exercises the renderer rather than a rasterization tie.
async function ready(page: Page) {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
}

/** Compares the on-screen document pixels with a CPU composite at the same output size. */
async function expectMatchesCpuAtZoom(page: Page, label: string, tolerance = 2) {
  const r = await page.evaluate(async () => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const s = api.store.getState(); const d = s.documents[s.activeId];
    const vp = s.viewports[s.activeId]; const dpr = window.devicePixelRatio || 1;
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const gl = Array.from(api.readDocumentPixels()) as number[];
    // The same independent edge rounding readDocumentPixels and the GL scissor both use.
    const rect = vp.documentRect({ width: d.width, height: d.height });
    const w = Math.round((rect.x + rect.width) * dpr) - Math.round(rect.x * dpr);
    const h = Math.round((rect.y + rect.height) * dpr) - Math.round(rect.y * dpr);
    const cpu = Array.from(api.engine.composite(d.id, { x: 0, y: 0, width: d.width, height: d.height }, w, h)) as number[];
    return { gl, cpu, kind: s.rendererKind, w, h };
  });
  expect(r.gl.length, `${label} output size ${r.w}x${r.h}`).toBe(r.cpu.length);
  const worst = r.gl.reduce((m, v, i) => Math.max(m, Math.abs(v - r.cpu[i])), 0);
  expect(worst, `${label} (${r.kind}, ${r.w}x${r.h}) max byte diff`).toBeLessThanOrEqual(tolerance);
}

/**
 * The divergence I12 described: the GPU mipmapped every non-Nearest layer while the CPU
 * prefiltered only High quality, so a Smooth layer zoomed out disagreed between the canvas and
 * the export. Both now reduce by the same prefilterLevel rule.
 */
for (const sampling of ["Smooth", "High quality"] as const) {
  test(`a ${sampling} layer zoomed out matches the CPU compositor`, async ({ page }) => {
    await ready(page);
    const b64 = await page.evaluate(noisePngBase64);
    await page.evaluate(async ({ b64, sampling }) => {
      const api = (window as unknown as { __compositor: any }).__compositor;
      const png = Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0));
      const doc = api.engine.newDocument(64, 64, false);
      api.engine.importImage(doc, png, "noise", { x: 32, y: 32 });
      const id = api.engine.state(doc).activeLayerId;
      const t = api.engine.state(doc).layers.find((l: any) => l.id === id).transform;
      api.engine.execute(doc, { type: "SetLayerTransform", id, transform: { ...t, sampling } });
      api.store.getState().openDocument(doc);
      // Four source pixels per output pixel: one halving on both sides.
      await api.setZoom(0.25);
      api.setCheckerboard(false);
    }, { b64, sampling });
    await expectMatchesCpuAtZoom(page, `${sampling} at zoom 0.25`);
  });
}

test("zoom 1 still matches, so the prefilter rule does not fire when it should not", async ({ page }) => {
  await ready(page);
  const b64 = await page.evaluate(noisePngBase64);
  await page.evaluate(async (b64) => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const png = Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 64, false);
    api.engine.importImage(doc, png, "noise", { x: 32, y: 32 });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
  }, b64);
  await expectMatchesCpuAtZoom(page, "noise at zoom 1");
});

/**
 * A clipping stack zoomed out. The GL renderer samples the clip source from the reduced
 * texture; the CPU compositor now reduces it the same way, which is what macOS does - a
 * clipping source is painted through the same `drawOwn` closure as any other layer, so it goes
 * through `LayerRenderer.draw` and its sharp halvings.
 *
 * The base is hidden on purpose. A visible base and its clipped layer become a Stack node,
 * which shares the base's alpha directly and never consults the source; hiding it routes the
 * draw through the clip-source path on both renderers, where coverage uses the source's alpha
 * regardless of visibility.
 */
test("a clipping stack zoomed out matches the CPU compositor", async ({ page }) => {
  await ready(page);
  const sparse = await page.evaluate(sparseAlphaPngBase64);
  const noise = await page.evaluate(noisePngBase64);
  await page.evaluate(async ({ sparse, noise }) => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const decode = (b: string) => Uint8Array.from(atob(b), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 64, false);
    // The source's alpha is what becomes coverage, so it is the sparse fixture.
    api.engine.importImage(doc, decode(sparse), "source", { x: 32, y: 32 });
    const source = api.engine.state(doc).activeLayerId;
    api.engine.importImage(doc, decode(noise), "clipped", { x: 32, y: 32 });
    const clipped = api.engine.state(doc).activeLayerId;
    api.engine.execute(doc, { type: "ToggleClipping", id: clipped });
    api.engine.execute(doc, { type: "SetLayerVisible", id: source, visible: false });
    api.store.getState().openDocument(doc);
    await api.setZoom(0.25);
    api.setCheckerboard(false);
  }, { sparse, noise });
  await expectMatchesCpuAtZoom(page, "clipping stack at zoom 0.25");
});

/**
 * A clipping source with a transparent interior. The clip pass composes onto a freshly cleared
 * target with no backdrop bound, which is the fragment that used to read outside the 1x1
 * placeholder texture. ANGLE over D3D11 returns zero there, so this is path coverage on this
 * backend rather than a discriminating test of the portability fix.
 */
test("a clip source with a transparent hole clips the same on GPU and CPU", async ({ page }) => {
  await ready(page);
  const ring = await page.evaluate(ringPngBase64);
  const red = await page.evaluate(redSquarePngBase64);
  await page.evaluate(async ({ ring, red }) => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const decode = (b: string) => Uint8Array.from(atob(b), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(16, 16, false);
    api.engine.importImage(doc, decode(ring), "source", { x: 8, y: 8 });
    const source = api.engine.state(doc).activeLayerId;
    api.engine.importImage(doc, decode(red), "clipped", { x: 8, y: 8 });
    const clipped = api.engine.state(doc).activeLayerId;
    for (const id of [source, clipped]) {
      const t = api.engine.state(doc).layers.find((l: any) => l.id === id).transform;
      api.engine.execute(doc, { type: "SetLayerTransform", id, transform: { ...t, size: [16, 16], origin: [0, 0], sampling: "Nearest" } });
    }
    api.engine.execute(doc, { type: "ToggleClipping", id: clipped });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
  }, { ring, red });
  await expectMatchesCpuAtZoom(page, "clip source with a hole");
});
