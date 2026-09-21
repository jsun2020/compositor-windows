import { test, expect, type Page } from "@playwright/test";
import { redSquarePngBase64 } from "./helpers";

/** Reads the on-screen document pixels and compares them with the CPU compositor within 2/255 per byte. */
async function matchesCpu(page: Page, label: string): Promise<void> {
  const result = await page.evaluate(async () => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const s = api.store.getState(); const d = s.documents[s.activeId];
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const gl = Array.from(api.readDocumentPixels()) as number[];
    const cpu = Array.from(api.engine.composite(d.id, { x: 0, y: 0, width: d.width, height: d.height }, d.width, d.height)) as number[];
    return { gl, cpu, kind: s.rendererKind };
  });
  expect(result.gl.length).toBe(result.cpu.length);
  const worst = result.gl.reduce((m, v, i) => Math.max(m, Math.abs(v - result.cpu[i])), 0);
  expect(worst, `${label} (${result.kind}) max byte diff`).toBeLessThanOrEqual(2);
}

/** A 4x4 document with a red 2x2 layer at (1,1) rendered at zoom 1 must match the engine's CPU composite. */
test("gl renderer matches the CPU compositor", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(redSquarePngBase64);
  await page.evaluate(async (b64) => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const doc = api.engine.newDocument(4, 4, false);
    const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    api.engine.importImage(doc, bytes, "red", { x: 2, y: 2 });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
  }, b64);
  await matchesCpu(page, "red square");
});

test("cpu fallback renders when WebGL2 is unavailable", async ({ page }) => {
  await page.addInitScript(() => {
    const original = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (type: string, ...rest: unknown[]) {
      if (type === "webgl2") return null;
      return (original as any).call(this, type, ...rest);
    };
  });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const kind = await page.evaluate(async () => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const doc = api.engine.newDocument(4, 4, true);
    api.store.getState().openDocument(doc);
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    return api.store.getState().rendererKind;
  });
  expect(kind).toBe("cpu");
});
