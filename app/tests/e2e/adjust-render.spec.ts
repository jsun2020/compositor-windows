import { test, expect, type Page } from "@playwright/test";
import { noisePngBase64 } from "./helpers";

// The canvas and the CPU compositor must agree on every adjustment. The fixture is per-pixel
// noise: flat colour would hide a wrong LUT index, and a smooth ramp would hide a wrong hue.
test.use({ viewport: { width: 900, height: 720 } });

async function setup(page: Page): Promise<string> {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(noisePngBase64);
  return page.evaluate(async (data) => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const png = Uint8Array.from(atob(data), (c) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 64, false);
    api.engine.importImage(doc, png, "noise", { x: 32, y: 32 });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
    return doc as string;
  }, b64);
}

async function expectMatchesCpu(page: Page, label: string) {
  const r = await page.evaluate(async () => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const s = api.store.getState(); const d = s.documents[s.activeId];
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const gl = Array.from(api.readDocumentPixels()) as number[];
    const cpu = Array.from(api.engine.compositeEdit(d.id, s.previewEdit(), { x: 0, y: 0, width: d.width, height: d.height }, d.width, d.height)) as number[];
    return { gl, cpu, kind: s.rendererKind };
  });
  expect(r.gl.length).toBe(r.cpu.length);
  const worst = r.gl.reduce((m, v, i) => Math.max(m, Math.abs(v - r.cpu[i])), 0);
  // Grain hashes in f32 on both sides but rounds at different points; 3 covers that one level.
  expect(worst, `${label} (${r.kind}) max byte diff`).toBeLessThanOrEqual(label === "Grain" ? 3 : 2);
}

test("every adjustment layer renders the same on the GPU and the CPU", async ({ page }) => {
  await setup(page);
  const add = (kind: string, settings: unknown) => page.evaluate(({ kind, settings }) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    api.engine.execute(s.activeId, { type: "AddAdjustmentLayer", kind, seed: 7, shadows: [0.9, 0.1, 0.2], highlights: [0.1, 0.4, 1] });
    const state = api.engine.state(s.activeId);
    const layer = state.layers.filter((l: any) => l.adjustment).at(-1);
    if (settings) api.engine.execute(s.activeId, { type: "SetAdjustment", id: layer.id, adjustment: { ...layer.adjustment, ...(settings as object) } });
    s.refresh(); s.invalidate();
    return layer.id as string;
  }, { kind, settings });
  const remove = (id: string) => page.evaluate((id) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    api.engine.execute(s.activeId, { type: "DeleteLayers", ids: [id], bake: false }); s.refresh(); s.invalidate();
  }, id);

  const cases: [string, unknown][] = [
    ["Levels", { levels: { channel: "RGB", ranges: [{ black: 30, gamma: 1.6, white: 220, outputBlack: 12, outputWhite: 243 }, { black: 0, gamma: 1.2, white: 255, outputBlack: 0, outputWhite: 255 }, { black: 10, gamma: 1, white: 250, outputBlack: 0, outputWhite: 255 }, { black: 0, gamma: 1, white: 255, outputBlack: 0, outputWhite: 255 }] } }],
    ["Curves", { curves: { channel: "RGB", channels: [[{ x: 0, y: 15 }, { x: 96, y: 150 }, { x: 255, y: 240 }], [{ x: 0, y: 0 }, { x: 255, y: 255 }], [{ x: 0, y: 0 }, { x: 128, y: 100 }, { x: 255, y: 255 }], [{ x: 0, y: 0 }, { x: 255, y: 255 }]] } }],
    ["Exposure", { exposureSettings: { exposure: 0.7, offset: 0.06, gamma: 1.4 } }],
    ["Gradient Map", { gradientMapSettings: { shadows: { red: 0.9, green: 0.1, blue: 0.2 }, highlights: { red: 0.1, green: 0.4, blue: 1 }, reversed: false } }],
    ["Hue/Saturation", null],
    ["Grain", { grainSettings: { amount: 65, size: 2.5, roughness: 35, seed: 7 } }],
  ];
  for (const [kind, settings] of cases) {
    const id = await add(kind, settings);
    await expectMatchesCpu(page, kind);
    await remove(id);
  }
});

test("hue/saturation ranges, opacity, masks, blend modes and clipping match the CPU", async ({ page }) => {
  await setup(page);
  const id = await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState();
    api.engine.execute(s.activeId, { type: "AddAdjustmentLayer", kind: "Hue/Saturation", seed: 0, shadows: null, highlights: null });
    const layer = api.engine.state(s.activeId).layers.filter((l: any) => l.adjustment).at(-1);
    const hsv = { range: "Reds", colorize: false, invertRange: false,
      adjustments: { Master: { hue: 25, saturation: 15, lightness: -5 }, Blues: { hue: -40, saturation: 60, lightness: 10 } },
      bands: layer.adjustment.hsvSettings?.bands ?? {} };
    api.engine.execute(s.activeId, { type: "SetAdjustment", id: layer.id, adjustment: { ...layer.adjustment, hsvSettings: hsv } });
    s.refresh(); s.invalidate();
    return layer.id as string;
  });
  await expectMatchesCpu(page, "hue ranges");
  // Ruling (pre-flight, Task 13): colorize with a NON-MASTER range is the one combination where a
  // shader that reads adjustments.Master diverges from hsv.rs (which reads adjustment(range)) while
  // every Rust test stays green, because the divergence lives entirely in GLSL. The finished panel
  // cannot produce it - the Colorize toggle resets the settings object to Master - but a .comp file
  // or a SetAdjustment command can, and the parity constraint covers every adjustment layer, not
  // only UI-reachable ones. Reds carries different amounts from Master here, so the two disagree.
  const setColorize = (on: boolean, range: string, hue: number) => page.evaluate(({ id, on, range, hue }) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    const layer = api.engine.state(s.activeId).layers.find((l: any) => l.id === id);
    const hsv = { ...layer.adjustment.hsvSettings, colorize: on, range,
      adjustments: { ...layer.adjustment.hsvSettings.adjustments, Reds: { hue, saturation: 70, lightness: 0 } } };
    api.engine.execute(s.activeId, { type: "SetAdjustment", id, adjustment: { ...layer.adjustment, hsvSettings: hsv } });
    s.refresh(); s.invalidate();
  }, { id, on, range, hue });
  await setColorize(true, "Reds", 200);
  await expectMatchesCpu(page, "colorize with a non-Master range");
  // A NEGATIVE colorize hue is the second divergence the shader has to survive. Task 4 dropped the
  // negative wrap to match the Mac's truncatingRemainder, so hue stays negative into hslToRgb.
  // Rust's `%` truncates and its sector dispatch is `match sector as i64` with a catch-all, so a
  // hue of -300 lands on the catch-all arm; a GLSL mod() plus a `sector < 1.0` comparison chain
  // would send it to arm 0 instead and paint a different colour. -300 exercises both.
  await setColorize(true, "Reds", -300);
  await expectMatchesCpu(page, "colorize with a negative hue");
  await setColorize(false, "Reds", 200);
  const run = (cmd: unknown) => page.evaluate((cmd) => { const api = (window as any).__compositor; const s = api.store.getState(); api.engine.execute(s.activeId, cmd); s.refresh(); s.invalidate(); }, cmd);
  await run({ type: "SetLayerOpacity", id, opacity: 0.45 });
  await expectMatchesCpu(page, "opacity");
  await run({ type: "SetLayerBlendMode", id, mode: "Multiply" });
  await expectMatchesCpu(page, "blend mode");
  await run({ type: "SetLayerBlendMode", id, mode: "Normal" });
  await run({ type: "AddMask", id, revealing: true });
  await run({ type: "BlurMask", id, radius: 4 });
  await expectMatchesCpu(page, "soft mask");
  await run({ type: "ToggleClipping", id });
  await expectMatchesCpu(page, "clipped to the layer below");
});

test("a live panel preview draws the same as the CPU", async ({ page }) => {
  await setup(page);
  await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState();
    s.beginAdjust({ kind: "Curves" });
    // beginAdjust mutates the store; re-fetch rather than read adjustEdit off the pre-call
    // snapshot `s`, which zustand leaves frozen at its pre-mutation value.
    const next = JSON.parse(JSON.stringify(api.store.getState().adjustEdit.adjustment));
    next.curves.channels[0] = [{ x: 0, y: 40 }, { x: 128, y: 200 }, { x: 255, y: 250 }];
    api.store.getState().updateAdjust({ adjustment: next });
  });
  await expectMatchesCpu(page, "destructive preview");
  await page.evaluate(() => (window as any).__compositor.store.getState().cancelAdjust());
  await expectMatchesCpu(page, "preview cancelled");
});
