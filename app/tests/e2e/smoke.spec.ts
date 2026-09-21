import { test, expect } from "@playwright/test";

test("engine loads in the browser", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toContainText("Compositor engine 0.1.0");
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
    return { hasMask: after.layers[0].hasMask, maskWidth: after.layers[0].maskWidth, nodes: plan.nodes.length, maskBytes: pixels ? pixels.length : -1, merge: api.engine.mergeAction(doc, [id]) };
  });
  expect(result.hasMask).toBe(true);
  expect(result.maskWidth).toBe(1);
  expect(result.nodes).toBe(1); // a blank layer still gets a plan node; renderers skip it for lack of pixels
  expect(result.maskBytes).toBe(1);
  expect(result.merge).toBeNull();
});
