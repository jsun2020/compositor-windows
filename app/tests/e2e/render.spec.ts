import { test, expect } from "@playwright/test";

/** A 4x4 document with a red 2x2 layer at (1,1) rendered at zoom 1 must match the engine's CPU composite. */
test("gl renderer matches the CPU compositor", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result = await page.evaluate(async () => {
    /**
     * Draws a 2x2 fully-opaque red square on a canvas and returns its PNG bytes, decoded
     * in-browser so the test never depends on a hand-verified base64 literal.
     */
    async function redSquarePng(): Promise<Uint8Array> {
      const canvas = document.createElement("canvas");
      canvas.width = 2;
      canvas.height = 2;
      const ctx = canvas.getContext("2d")!;
      ctx.fillStyle = "#ff0000";
      ctx.fillRect(0, 0, 2, 2);
      const blob = await new Promise<Blob>((resolve, reject) => {
        canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("toBlob failed"))), "image/png");
      });
      return new Uint8Array(await blob.arrayBuffer());
    }

    const api = (window as unknown as { __compositor: any }).__compositor;
    const doc = api.engine.newDocument(4, 4, false);
    const png = await redSquarePng();
    api.engine.importImage(doc, png, "red", { x: 2, y: 2 });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const gl = Array.from(api.readDocumentPixels()) as number[];
    const cpu = Array.from(api.engine.composite(doc, { x: 0, y: 0, width: 4, height: 4 }, 4, 4)) as number[];
    return { gl, cpu, kind: api.store.getState().rendererKind };
  });
  expect(result.gl.length).toBe(64);
  for (let i = 0; i < 64; i++) expect(Math.abs(result.gl[i] - result.cpu[i]), `byte ${i} (${result.kind})`).toBeLessThanOrEqual(2);
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
