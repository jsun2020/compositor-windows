import { test, expect } from "@playwright/test";

test("engine loads in the browser", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toContainText("Compositor engine 0.3.7");
  const ids = await page.evaluate(() => {
    const api = (window as unknown as { __compositor: { engine: { newDocument(w: number, h: number, e: boolean): string; documentIds(): string[] } } }).__compositor;
    api.engine.newDocument(10, 10, true);
    return api.engine.documentIds();
  });
  expect(ids).toHaveLength(1);
});

test("phase 2 client calls reach the engine", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result = await page.evaluate(() => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const doc = api.engine.newDocument(10, 10, true);
    const state = api.engine.state(doc);
    const id = state.layers[0].id;
    api.engine.execute(doc, { type: "AddMask", id, revealing: true });
    const after = api.engine.state(doc);
    const plan = api.engine.renderPlan(doc, null);
    const pixels = api.engine.maskPixels(doc, id);
    api.engine.execute(doc, { type: "AddAdjustmentLayer", kind: "Levels", seed: 0, shadows: null, highlights: null });
    const withAdjustment = api.engine.state(doc);
    const adjustmentLayer = withAdjustment.layers.find((l: any) => l.adjustment);
    const bins = api.engine.histogram(doc, adjustmentLayer.id);
    const auto = api.engine.autoLevels(bins, "Contrast");
    const planWithAdjustment = api.engine.renderPlan(doc, null);
    const adjustmentDraw = planWithAdjustment.nodes.map((n: any) => n.draw).find((d: any) => d && d.adjustment);
    return {
      hasMask: after.layers[0].hasMask, maskWidth: after.layers[0].maskWidth, nodes: plan.nodes.length, maskBytes: pixels ? pixels.length : -1, merge: api.engine.mergeAction(doc, [id]),
      adjustmentKind: adjustmentLayer.adjustment.kind, bins: bins.length, autoIsIdentity: auto.ranges[0].black === 0 && auto.ranges[0].white === 255, drawHasAdjustment: !!adjustmentDraw,
    };
  });
  expect(result.hasMask).toBe(true);
  expect(result.maskWidth).toBe(1);
  expect(result.nodes).toBe(1); // a blank layer still gets a plan node; renderers skip it for lack of pixels
  expect(result.maskBytes).toBe(1);
  expect(result.merge).toBeNull();
  expect(result.adjustmentKind).toBe("Levels");
  expect(result.bins).toBe(4);
  expect(result.autoIsIdentity).toBe(true);   // a blank document has nothing to stretch
  expect(result.drawHasAdjustment).toBe(true);
});
