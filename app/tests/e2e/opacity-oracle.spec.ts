import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";

interface Row { backdrop: number[]; source: number[]; opacityByte: number; mode: string; result: number[] }
const rows: Row[] = JSON.parse(readFileSync(new URL("../../../engine/tests/fixtures/core-graphics-opacity.json", import.meta.url), "utf8"));

for (const mode of ["Normal", "Multiply", "Screen"]) {
  test(`GPU covered source matches independent Mac ${mode} bytes`, async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/");
    await expect(page.getByTestId("engine-ready")).toBeVisible();
    const actual = await page.evaluate(async (cases) => {
      const api = (window as any).__compositor;
      function png(pm: number[]) {
        const c = document.createElement("canvas"); c.width = c.height = 8;
        const ctx = c.getContext("2d")!;
        const straight = pm.map((v, i) => i === 3 ? v : pm[3] ? Math.min(255, Math.round(v * 255 / pm[3])) : 0);
        const bytes = new Uint8ClampedArray(8 * 8 * 4);
        for (let i = 0; i < bytes.length; i += 4) bytes.set(straight, i);
        ctx.putImageData(new ImageData(bytes, 8, 8), 0, 0);
        return Uint8Array.from(atob(c.toDataURL("image/png").split(",")[1]), (v) => v.charCodeAt(0));
      }
      const results = [];
      for (const row of cases) {
        const doc = api.engine.newDocument(8, 8, false);
        // importImage takes the image's centre, unlike Layer::with_pixels.
        api.engine.importImage(doc, png(row.backdrop), "backdrop", { x: 4, y: 4 });
        api.engine.importImage(doc, png(row.source), "source", { x: 4, y: 4 });
        const id = api.engine.state(doc).activeLayerId;
        for (const layer of api.engine.state(doc).layers.filter((l: any) => l.name === "source" || l.name === "backdrop")) {
          if (layer.transform.origin[0] !== 0 || layer.transform.origin[1] !== 0) throw new Error("Oracle images must cover the document from its origin.");
        }
        api.engine.execute(doc, { type: "SetLayerOpacity", id, opacity: row.opacityByte / 255 });
        api.engine.execute(doc, { type: "SetLayerBlendMode", id, mode: row.mode });
        api.store.getState().openDocument(doc);
        await api.setZoom(1); api.setCheckerboard(false);
        await new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
        const gpu = api.readDocumentPixels();
        const cpu = api.engine.compositeEdit(doc, null, { x: 0, y: 0, width: 8, height: 8 }, 8, 8);
        const i = (4 * 8 + 4) * 4;
        results.push({ gpu: Array.from(gpu.slice(i, i + 4)), cpu: Array.from(cpu.slice(i, i + 4)), kind: api.store.getState().rendererKind });
      }
      return results;
    }, rows.filter((row) => row.mode === mode));
    const cases = rows.filter((row) => row.mode === mode);
    for (let i = 0; i < cases.length; i++) {
      expect(actual[i].kind).toBe("gl");
      expect(actual[i].cpu, JSON.stringify(cases[i])).toEqual(cases[i].result);
      expect(actual[i].gpu, JSON.stringify(cases[i])).toEqual(cases[i].result);
    }
  });
}
