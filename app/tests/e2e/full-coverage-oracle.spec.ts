import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";

interface Row { backdrop: number[]; source: number[]; opacityByte: number; mode: string; result: number[] }
const rows: Row[] = JSON.parse(readFileSync(new URL("../../../engine/tests/fixtures/core-graphics-full-coverage.json", import.meta.url), "utf8"));

for (const mode of ["Normal", "Multiply", "Screen"]) for (const opacityByte of [254, 255]) {
  test(`CPU and GPU match all independent Mac ${mode} pairs at opacity ${opacityByte}/255`, async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/");
    await expect(page.getByTestId("engine-ready")).toBeVisible();
    const cases = rows.filter((r) => r.mode === mode && r.opacityByte === opacityByte);
    expect(cases).toHaveLength(1296);
    const actual = await page.evaluate(async (matrix) => {
      const api = (window as any).__compositor;
      const side = 288, cell = 8, columns = 36;
      function png(key: "backdrop" | "source") {
        const canvas = document.createElement("canvas"); canvas.width = canvas.height = side;
        const ctx = canvas.getContext("2d")!;
        const pixels = new Uint8ClampedArray(side * side * 4);
        for (let n = 0; n < matrix.length; n++) {
          const pm = matrix[n][key];
          const straight = pm.map((v, i) => i === 3 ? v : pm[3] ? Math.min(255, Math.round(v * 255 / pm[3])) : 0);
          for (let y = 0; y < cell; y++) for (let x = 0; x < cell; x++) {
            const at = ((Math.floor(n / columns) * cell + y) * side + n % columns * cell + x) * 4;
            pixels.set(straight, at);
          }
        }
        ctx.putImageData(new ImageData(pixels, side, side), 0, 0);
        return Uint8Array.from(atob(canvas.toDataURL("image/png").split(",")[1]), (c) => c.charCodeAt(0));
      }
      const doc = api.engine.newDocument(side, side, false);
      api.engine.importImage(doc, png("backdrop"), "backdrop", { x: side / 2, y: side / 2 });
      api.engine.importImage(doc, png("source"), "source", { x: side / 2, y: side / 2 });
      const id = api.engine.state(doc).activeLayerId;
      const imported = api.engine.state(doc).layers.filter((l: any) => l.name === "source" || l.name === "backdrop");
      for (const layer of imported) {
        if (layer.transform.origin[0] !== 0 || layer.transform.origin[1] !== 0) throw new Error("Oracle input must cover the document from its origin.");
        const stored = new Uint8Array(api.engine.jobInput(doc, layer.id).pixels);
        for (let n = 0; n < matrix.length; n++) {
          const at = ((Math.floor(n / columns) * cell + 4) * side + n % columns * cell + 4) * 4;
          if (!matrix[n][layer.name as "backdrop" | "source"].every((v, i) => stored[at + i] === v)) throw new Error("PNG import changed an independent premultiplied input.");
        }
      }
      api.engine.execute(doc, { type: "SetLayerOpacity", id, opacity: matrix[0].opacityByte / 255 });
      api.engine.execute(doc, { type: "SetLayerBlendMode", id, mode: matrix[0].mode });
      api.store.getState().openDocument(doc);
      await api.setZoom(1); api.setCheckerboard(false);
      await new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
      const gpu = api.readDocumentPixels();
      const cpu = api.engine.compositeEdit(doc, null, { x: 0, y: 0, width: side, height: side }, side, side);
      return { kind: api.store.getState().rendererKind, pixels: matrix.map((_, n) => {
        const at = ((Math.floor(n / columns) * cell + 4) * side + n % columns * cell + 4) * 4;
        return { cpu: Array.from(cpu.slice(at, at + 4)), gpu: Array.from(gpu.slice(at, at + 4)) };
      }) };
    }, cases);
    expect(actual.kind).toBe("gl");
    for (let n = 0; n < cases.length; n++) {
      expect(actual.pixels[n].cpu, JSON.stringify(cases[n])).toEqual(cases[n].result);
      expect(actual.pixels[n].gpu, JSON.stringify(cases[n])).toEqual(cases[n].result);
    }
  });
}
