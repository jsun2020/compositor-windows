import { test, expect, type Page } from "@playwright/test";
import { grayRampMaskPngBase64, redSquarePngBase64 } from "./helpers";

async function setup(page: Page): Promise<{ doc: string; a: string; b: string }> {
  // Pin the viewport so the document rect (centred in the CanvasView element) lands on an
  // integer device pixel. With Playwright's default 1280x720 viewport this app's chrome
  // leaves an odd-height CanvasView, which centres an even-sized test document at a y
  // ending in .5 -- every document row then straddles two device rows exactly evenly, a
  // rasterization tie that is resolved arbitrarily (and inconsistently with the CPU
  // compositor's own point sampling) for any edge that isn't axis-aligned, e.g. a
  // distorted quad's diagonal side. A pinned, chrome-parity-matched viewport keeps the
  // rect integer so the comparison exercises the renderer, not this unrelated tie.
  // (Height 720, not 721: the move tool's Transform inspector now occupies the
  // tool-options row by default, one CSS px taller than the chrome this was tuned
  // against, which flips the parity that keeps the rect on an integer pixel.)
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(redSquarePngBase64);
  return page.evaluate(async (b64) => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const png = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    const doc = api.engine.newDocument(8, 8, false);
    api.engine.importImage(doc, png, "a", { x: 2, y: 2 });
    const a = api.engine.state(doc).activeLayerId;
    api.engine.importImage(doc, png, "b", { x: 3, y: 3 });
    const b = api.engine.state(doc).activeLayerId;
    for (const id of [a, b]) { const t = api.engine.state(doc).layers.find((l: any) => l.id === id).transform; api.engine.execute(doc, { type: "SetLayerTransform", id, transform: { ...t, sampling: "Nearest" } }); }
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
    return { doc, a, b };
  }, b64);
}

async function expectMatchesCpu(page: Page, label: string, edit: unknown = null) {
  const r = await page.evaluate(async (edit) => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const s = api.store.getState(); const d = s.documents[s.activeId];
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const gl = Array.from(api.readDocumentPixels()) as number[];
    const cpu = Array.from(api.engine.compositeEdit(d.id, edit, { x: 0, y: 0, width: d.width, height: d.height }, d.width, d.height)) as number[];
    return { gl, cpu, kind: s.rendererKind };
  }, edit);
  expect(r.gl.length).toBe(r.cpu.length);
  const worst = r.gl.reduce((m, v, i) => Math.max(m, Math.abs(v - r.cpu[i])), 0);
  expect(worst, `${label} (${r.kind}) max byte diff`).toBeLessThanOrEqual(2);
}

test("blend modes, opacity, masks, folder masks and clipping match the CPU compositor", async ({ page }) => {
  const { a, b } = await setup(page);
  const run = (cmd: unknown) => page.evaluate(({ cmd }) => { const api = (window as any).__compositor; api.engine.execute(api.store.getState().activeId, cmd); api.store.getState().refresh(); }, { cmd });
  // All thirteen modes are compared. Darken and Lighten are separable; Saturation and Color are
  // the two non-separable HSL modes that had no coverage, where a setSat/setLum mistake in the
  // shader would not show up through Hue or Luminosity alone.
  for (const mode of ["Multiply", "Screen", "Overlay", "Darken", "Lighten", "Difference", "Color Dodge", "Color Burn", "Hue", "Saturation", "Color", "Luminosity"]) {
    await run({ type: "SetLayerBlendMode", id: b, mode });
    await expectMatchesCpu(page, mode);
  }
  await run({ type: "SetLayerBlendMode", id: b, mode: "Normal" });
  await run({ type: "SetLayerOpacity", id: b, opacity: 0.5 });
  await expectMatchesCpu(page, "opacity");
  await run({ type: "AddMask", id: b, revealing: true });
  await run({ type: "InvertMask", id: b });
  await expectMatchesCpu(page, "hide-all mask");
  await run({ type: "DeleteMask", id: b });
  await run({ type: "GroupLayers", ids: [a, b] });
  const folder = await page.evaluate(() => { const api = (window as any).__compositor; const s = api.store.getState(); return s.documents[s.activeId].layers.find((l: any) => l.isGroup).id; });
  await run({ type: "AddMask", id: folder, revealing: false });
  await expectMatchesCpu(page, "folder hide-all");
  await run({ type: "DeleteMask", id: folder });
  await run({ type: "ToggleClipping", id: b });
  await expectMatchesCpu(page, "clipping stack");
  await run({ type: "SetLayerVisible", id: a, visible: false });
  await expectMatchesCpu(page, "clipped to a hidden base");
});

/**
 * A partially transparent mask, placed so it covers only part of its layer. Every mask
 * comparison until now used a uniform 1x1 mask, where the expected image is "the layer
 * vanished" - a wrong mask transform, a wrong `background`, or an all-zero coverage texture all
 * pass that. A gray ramp under a placement smaller than the layer makes all three visible: the
 * interior varies, and the area outside the placement falls back to the mask's background.
 *
 * The mask arrives through a package, because no Phase 2 command produces a non-uniform mask;
 * they come from a `.comp` file, which is also how one would arrive from macOS.
 */
for (const sampling of ["Nearest", "High quality"] as const) {
  test(`a placed gray-ramp mask matches the CPU compositor (${sampling} mask sampling)`, async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/");
    await expect(page.getByTestId("engine-ready")).toBeVisible();
    const red = await page.evaluate(redSquarePngBase64);
    const mask = await page.evaluate(grayRampMaskPngBase64);
    const state = await page.evaluate(async ({ red, mask, sampling }) => {
      const api = (window as unknown as { __compositor: any }).__compositor;
      const decode = (b: string) => Uint8Array.from(atob(b), (c: string) => c.charCodeAt(0));
      const id = "B1000000-0000-4000-8000-0000000000A1";
      const manifest = {
        format: "com.compositor.project", version: 7, colorSpace: "sRGB", resolution: 72,
        documentID: "B1000000-0000-4000-8000-0000000000D1", width: 32, height: 32,
        // Selected on open, so the Transform inspector occupies the tool-options row. That is
        // the chrome height the pinned viewport above is tuned for; without it the CanvasView
        // is one CSS px taller and centres the document on a half device pixel, which makes
        // every document row straddle two device rows (see `setup`).
        activeLayerID: id,
        layers: [{
          id, name: "masked", isVisible: true,
          imageFile: `${id}.png`, maskFile: `${id}.mask.png`, maskEnabled: true, maskLinked: false,
          // Smaller than the layer and rotated, so the placement matrix matters and part of the
          // layer falls outside the mask onto its background.
          maskPlacement: { origin: [6, 4], size: [18, 22], rotation: 12, flipX: false, flipY: false, sampling: "High quality" },
          transform: { origin: [0, 0], size: [32, 32], rotation: 0, flipX: false, flipY: false, sampling },
        }],
      };
      const doc = api.engine.openPackage({
        manifest: JSON.stringify(manifest),
        images: [{ name: `${id}.png`, bytes: decode(red) }, { name: `${id}.mask.png`, bytes: decode(mask) }],
      }, null);
      api.store.getState().openDocument(doc);
      await api.setZoom(1);
      api.setCheckerboard(false);
      return api.engine.state(doc).layers[0];
    }, { red, mask, sampling });
    // The package really carried a 32x32 mask, not a uniform one silently substituted.
    expect(state.hasMask).toBe(true);
    expect([state.maskWidth, state.maskHeight]).toEqual([32, 32]);
    expect(state.maskLinked).toBe(false);
    await expectMatchesCpu(page, `gray ramp mask (${sampling})`);
  });
}

test("distortion preview and group transform preview match the CPU compositor", async ({ page }) => {
  const { a, b } = await setup(page);
  const edit = { kind: "layer", id: a, draft: { origin: [2, 2], size: [2, 2], rotation: 0, flipX: false, flipY: false, sampling: "Nearest" }, corners: [[1, 1], [7, 1], [6, 5], [1, 5]] };
  await page.evaluate((edit) => { const api = (window as any).__compositor; api.store.getState().selectLayers([edit.id], edit.id); api.store.setState({ transformEdit: { kind: "layer", id: edit.id, ids: [edit.id], box: edit.draft, original: edit.draft, draft: edit.draft, corners: edit.corners, persistent: true, duplicated: false } }); api.store.getState().invalidate(); }, edit);
  await expectMatchesCpu(page, "distort preview", edit);
  const group = { kind: "group", ids: [a, b], box: { origin: [2, 2], size: [3, 3], rotation: 0, flipX: false, flipY: false, sampling: "High quality" }, draft: { origin: [1, 1], size: [6, 6], rotation: 0, flipX: false, flipY: false, sampling: "High quality" }, corners: null };
  await page.evaluate((g) => { const api = (window as any).__compositor; api.store.setState({ transformEdit: { kind: "group", id: g.ids[0], ids: g.ids, box: g.box, original: g.box, draft: g.draft, corners: null, persistent: false, duplicated: false } }); api.store.getState().invalidate(); }, group);
  await expectMatchesCpu(page, "group preview", group);
});
