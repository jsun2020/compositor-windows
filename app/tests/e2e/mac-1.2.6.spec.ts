import { test, expect, type Page } from "@playwright/test";
import { noisePngBase64, redSquarePngBase64 } from "./helpers";

// Copied verbatim from adjust-layers.spec.ts, where they are file-local.
async function setup(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(noisePngBase64);
  await page.evaluate(async (data) => {
    const api = (window as any).__compositor;
    const png = Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 64, false);
    api.engine.importImage(doc, png, "noise", { x: 32, y: 32 });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
  }, b64);
}
const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });

test("a layer using a blend mode this build does not draw yet shows it, disabled", async ({ page }) => {
  await setup(page);
  await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState();
    const id = api.engine.state(s.activeId).layers[0].id;
    api.engine.execute(s.activeId, { type: "SetLayerBlendMode", id, mode: "Soft Light" });
    s.refresh(); s.invalidate();
  });
  expect((await state(page)).layers[0].blendMode).toBe("Soft Light");
  const select = page.getByRole("combobox", { name: "Blend mode", exact: true });
  await expect(select).toHaveValue("Soft Light");
  // Not toBeDisabled(): that assertion's "follow-label" retargeting walks from the <option>,
  // which is not itself a labelable element, up through the enclosing <label>Blend <select>...
  // to the label's control (the <select>), and reports the SELECT's disabled state instead of
  // the option's. Checking the attribute directly targets the option itself.
  await expect(select.locator("option[value='Soft Light']")).toHaveAttribute("disabled", "");
});

/** A v9 manifest holding one adjustment layer of `kind`, with every field a LayerAdjustment always
 * writes. Built in Node and passed to the page, because page.evaluate serializes only its callback. */
function adjustmentManifest(kind: string, id: string) {
  const range = { black: 0, gamma: 1, white: 255, outputBlack: 0, outputWhite: 255 };
  const line = [{ x: 0, y: 0 }, { x: 255, y: 255 }];
  return {
    format: "com.compositor.project", version: 9, colorSpace: "sRGB",
    documentID: "0B6C6B1E-4F1B-4B4E-9E0A-111111111111", width: 64, height: 48, activeLayerID: id,
    layers: [{ id, name: kind, isVisible: true,
      transform: { origin: [0, 0], size: [64, 48], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
      adjustment: { kind, hue: 0, saturation: 0, lightness: 0, colorize: false,
        levels: { channel: "RGB", ranges: [range, range, range, range] },
        curves: { channel: "RGB", channels: [line, line, line, line] } } }],
  };
}

test("an adjustment layer of a kind with no editor yet cannot be opened for editing", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.evaluate(async (manifest) => {
    const api = (window as any).__compositor;
    const doc = api.engine.openPackage({ manifest: JSON.stringify(manifest), images: [] }, null);
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
  }, adjustmentManifest("Invert", "0B6C6B1E-4F1B-4B4E-9E0A-555555555555"));
  await page.getByTestId("layer-row").nth(0).dblclick();
  await expect(page.getByTestId("adjust-panel")).toHaveCount(0);
  await page.getByRole("button", { name: "Layer", exact: true }).click();
  await expect(page.getByTestId("menu-layer-edit-adjustment")).toBeDisabled();
});

// Copied verbatim from blend.spec.ts, where it is file-local, and renamed: this file already has
// an adjust-layers-style `setup` above.
async function setupPinned(page: Page): Promise<{ doc: string; a: string; b: string }> {
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

test("a 40% folder dims each child, on the GPU and the CPU alike", async ({ page }) => {
  const { a, b } = await setupPinned(page);
  const run = (cmd: unknown) => page.evaluate(({ cmd }) => { const api = (window as any).__compositor; api.engine.execute(api.store.getState().activeId, cmd); api.store.getState().refresh(); }, { cmd });
  await run({ type: "GroupLayers", ids: [a, b] });
  const folder = await page.evaluate(() => { const api = (window as any).__compositor; const s = api.store.getState(); return s.documents[s.activeId].layers.find((l: any) => l.isGroup).id; });
  await run({ type: "SetLayerOpacity", id: folder, opacity: 0.4 });
  await expectMatchesCpu(page, "40% folder");
  // Each opaque red square alone: 0.4 x 255 = 102. Where they overlap: 0.4 + 0.4 x 0.6 = 0.64, so 163.
  // Ignoring the folder gives 255 for both; Photoshop's group fade gives 102 at the overlap too.
  const alphas = await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState(); const d = s.documents[s.activeId];
    const px = Array.from(api.engine.compositeEdit(d.id, null, { x: 0, y: 0, width: d.width, height: d.height }, d.width, d.height)) as number[];
    return px.filter((v, i) => i % 4 === 3 && v > 0);
  });
  expect(Math.abs(Math.min(...alphas) - 102)).toBeLessThanOrEqual(1);
  expect(Math.abs(Math.max(...alphas) - 163)).toBeLessThanOrEqual(1);
});

test("selecting a folder enables Opacity but leaves Blend mode disabled", async ({ page }) => {
  const { a, b } = await setupPinned(page);
  const run = (cmd: unknown) => page.evaluate(({ cmd }) => { const api = (window as any).__compositor; api.engine.execute(api.store.getState().activeId, cmd); api.store.getState().refresh(); }, { cmd });
  await run({ type: "GroupLayers", ids: [a, b] });
  const folder = await page.evaluate(() => { const api = (window as any).__compositor; const s = api.store.getState(); return s.documents[s.activeId].layers.find((l: any) => l.isGroup).id; });
  await page.evaluate((id) => { (window as any).__compositor.store.getState().selectLayers([id], id); }, folder);
  await expect(page.getByRole("spinbutton", { name: "Opacity", exact: true })).toBeEnabled();
  await expect(page.getByRole("combobox", { name: "Blend mode", exact: true })).toBeDisabled();
});
