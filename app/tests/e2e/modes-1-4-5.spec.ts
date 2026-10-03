import { test, expect, type Page } from "@playwright/test";
import { hueSafeNoisePngBase64, noisePngBase64 } from "./helpers";

// Compositor 1.4.5's blend results (Phase 4.5): an adjustment layer and a clipping stack's base blend in
// their real modes (LiveMaskRenderer.swift:52-57, :126-134 at v1.4.5), where 1.2.10 drew the eight
// modes only Core Image computes as Normal. The engine pins the formulas (engine/tests/blend_modes_v9.rs);
// here the GPU is held to the CPU in every such mode.
const CORE_IMAGE_ONLY = ["Linear Burn", "Linear Dodge (Add)", "Vivid Light", "Linear Light", "Pin Light", "Hard Mix", "Subtract", "Divide"];

/** A 64 x 64 noise layer at zoom 1 on a pinned viewport (adjust-render.spec.ts's setup), with a second,
 * different noise above it and a third clipped to that one; returns the ids of the second and third. */
async function setup(page: Page): Promise<{ doc: string; base: string; child: string }> {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const [under, over] = [await page.evaluate(hueSafeNoisePngBase64), await page.evaluate(noisePngBase64)];
  return page.evaluate(async ([under, over]) => {
    const api = (window as any).__compositor;
    const png = (b64: string) => Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 64, false);
    api.engine.importImage(doc, png(under), "under", { x: 32, y: 32 });
    api.engine.importImage(doc, png(over), "base", { x: 32, y: 32 });
    const base = api.engine.state(doc).activeLayerId;
    api.engine.importImage(doc, png(under), "child", { x: 32, y: 32 });
    const child = api.engine.state(doc).activeLayerId;
    api.engine.execute(doc, { type: "SetLayerOpacity", id: child, opacity: 0.5 });
    api.engine.execute(doc, { type: "ToggleClipping", id: child });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
    return { doc, base, child };
  }, [under, over]);
}

const run = (page: Page, cmd: unknown) => page.evaluate((cmd) => {
  const api = (window as any).__compositor; const s = api.store.getState();
  api.engine.execute(s.activeId, cmd); s.refresh(); s.invalidate();
}, cmd);

/** The largest byte difference between what the GPU drew and the CPU compositor, over the document. */
async function worstAgainstCpu(page: Page): Promise<number> {
  return page.evaluate(async () => {
    const api = (window as any).__compositor;
    const s = api.store.getState(); const d = s.documents[s.activeId];
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const gl = Array.from(api.readDocumentPixels()) as number[];
    const cpu = Array.from(api.engine.composite(d.id, { x: 0, y: 0, width: d.width, height: d.height }, d.width, d.height)) as number[];
    return gl.length === cpu.length ? gl.reduce((m, v, i) => Math.max(m, Math.abs(v - cpu[i])), 0) : 256;
  });
}

test("a clipping stack based in each Core-Image-only mode draws on the GPU as on the CPU, and not as Normal", async ({ page }) => {
  const ids = await setup(page);
  const normal = await page.evaluate(() => { const api = (window as any).__compositor; const s = api.store.getState(); return Array.from(api.engine.composite(s.activeId, { x: 0, y: 0, width: 64, height: 64 }, 64, 64)) as number[]; });
  for (const mode of CORE_IMAGE_ONLY) {
    await run(page, { type: "SetLayerBlendMode", id: ids.base, mode });
    const plan = await page.evaluate((doc) => (window as any).__compositor.engine.renderPlan(doc, null), ids.doc);
    expect(plan.nodes.find((n: { kind: string }) => n.kind === "stack").base.blend, `${mode}: the stack's own mode`).toBe(mode);
    // Vivid Light divides by what is left of the group, which turns the one level the GPU's 8-bit
    // group surface can differ by into up to 3 (measured on p45-scratch, 2026-09-30).
    expect(await worstAgainstCpu(page), mode).toBeLessThanOrEqual(mode === "Vivid Light" ? 3 : 2);
    const cpu = await page.evaluate(() => { const api = (window as any).__compositor; const s = api.store.getState(); return Array.from(api.engine.composite(s.activeId, { x: 0, y: 0, width: 64, height: 64 }, 64, 64)) as number[]; });
    expect(cpu.some((v, i) => Math.abs(v - normal[i]) > 8), `${mode} is not drawn as Normal`).toBe(true);
  }
});

test("a Levels layer in each Core-Image-only mode draws on the GPU as on the CPU", async ({ page }) => {
  await setup(page);
  // Pinned at 721 while an adjustment layer is selected (the chrome grows by a row).
  await page.setViewportSize({ width: 1280, height: 721 });
  const levels = await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState();
    api.engine.execute(s.activeId, { type: "AddAdjustmentLayer", kind: "Levels", seed: 0, shadows: null, highlights: null });
    const layer = api.engine.state(s.activeId).layers.find((l: any) => l.adjustment);
    const adjustment = JSON.parse(JSON.stringify(layer.adjustment));
    adjustment.levels.ranges[0].outputBlack = 128; adjustment.levels.ranges[0].outputWhite = 128;
    api.engine.execute(s.activeId, { type: "SetAdjustment", id: layer.id, adjustment });
    s.refresh(); s.invalidate();
    return layer.id as string;
  });
  for (const mode of CORE_IMAGE_ONLY) {
    await run(page, { type: "SetLayerBlendMode", id: levels, mode });
    expect(await worstAgainstCpu(page), mode).toBeLessThanOrEqual(2);
  }
});
