import { test, expect } from "@playwright/test";

test("development effect replay retains one fully linked canvas renderer", async ({ page }) => {
  await page.addInitScript(() => {
    const counts = { linked: 0, deleted: 0 };
    (window as any).__canvasProgramLifetime = counts;
    const gl = WebGL2RenderingContext.prototype, link = gl.linkProgram, remove = gl.deleteProgram;
    gl.linkProgram = function (program) { counts.linked++; return link.call(this, program); };
    gl.deleteProgram = function (program) { counts.deleted++; return remove.call(this, program); };
  });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result = await page.evaluate(async () => {
    const api = (window as any).__compositor;
    const programs = Object.values(api.renderer.programs).filter((p: any) => p?.program);
    const counts = (window as any).__canvasProgramLifetime;
    const initial = { ...counts }, doc = api.engine.newDocument(64, 48, true);
    api.store.getState().openDocument(doc);
    await api.setZoom(2.3);
    const state = api.store.getState();
    api.renderer.render(api.engine, state.documents[doc], state.viewports[doc], window.devicePixelRatio || 1, { checkerboard: true }, null);
    const pixels = api.renderer.readPixels();
    api.store.getState().closeDocument(doc);
    return { programs: programs.length, initial, after: { ...counts }, painted: pixels.some((v: number) => v !== 0) };
  });
  expect(result.programs).toBeGreaterThan(0);
  expect(result.initial).toEqual({ linked: result.programs, deleted: 0 });
  expect(result.after).toEqual(result.initial);
  expect(result.painted).toBe(true);
});
