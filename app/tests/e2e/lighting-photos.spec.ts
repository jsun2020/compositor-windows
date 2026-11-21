import { expect, test } from "@playwright/test";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { clickMenu } from "./helpers";

interface Metrics {
  mae: number;
  p95: number;
  detailMae: number;
}

interface Photo {
  width: number;
  height: number;
  detailRect: [number, number, number, number];
  sha256: Record<string, string>;
  rim: Metrics;
  volume: Metrics;
}

const fixtureDir = resolve("app/tests/fixtures/mac1615-lighting");
const photos: Record<string, Photo> = JSON.parse(
  readFileSync(resolve(fixtureDir, "references.json"), "utf8"),
);

function fixture(name: string, photo: Photo) {
  const bytes = readFileSync(resolve(fixtureDir, name));
  expect(createHash("sha256").update(bytes).digest("hex")).toBe(photo.sha256[name]);
  return bytes.toString("base64");
}

for (const [name, photo] of Object.entries(photos)) {
  for (const [kind, action, title] of [
    ["rim", "rim-light", "Rim Light"],
    ["volume", "volumetric-light", "Volumetric Light"],
  ] as const) {
    test(`${title} on ${name} against native Mac 1.6.15`, async ({ page }, info) => {
      const input = fixture(name + ".png", photo);
      const reference = fixture(`mac-${name}-${kind}.png`, photo);
      await page.goto("/");
      await expect(page.getByTestId("engine-ready")).toBeVisible();
      await page.evaluate(({ input, width, height }) => {
        const { store, engine } = (window as any).__compositor;
        const id = engine.newDocument(width, height, false);
        engine.importImage(id, Uint8Array.from(atob(input), c => c.charCodeAt(0)), "Subject",
          { x: width / 2, y: height / 2 });
        store.getState().openDocument(id);
      }, { input, width: photo.width, height: photo.height });
      await clickMenu(page, "Filter", action);
      const dialog = page.getByRole("dialog", { name: title });
      const apply = dialog.getByRole("button", { name: "Apply", exact: true });
      await expect(apply).toBeEnabled({ timeout: 20_000 });
      if (kind === "volume") {
        await dialog.getByLabel("Lighting preview").evaluate(canvas => {
          const rect = canvas.getBoundingClientRect();
          canvas.dispatchEvent(new MouseEvent("click", { bubbles: true,
            clientX: rect.left + 0.25 * rect.width, clientY: rect.top + 0.3 * rect.height }));
        });
        await expect(apply).toBeEnabled();
      }
      await apply.click();
      await expect(dialog).toHaveCount(0);

      const result = await page.evaluate(async ({ reference, input, width, height, detailRect }) => {
        const { engine, store } = (window as any).__compositor;
        async function pixels(bytes: Uint8Array<ArrayBuffer>) {
          const bitmap = await createImageBitmap(new Blob([bytes], { type: "image/png" }));
          if (bitmap.width !== width || bitmap.height !== height) throw Error("Photo dimensions changed");
          const canvas = new OffscreenCanvas(width, height);
          const ctx = canvas.getContext("2d")!;
          ctx.drawImage(bitmap, 0, 0);
          bitmap.close();
          return ctx.getImageData(0, 0, width, height).data;
        }
        const decode = (s: string) => Uint8Array.from(atob(s), c => c.charCodeAt(0));
        const [actual, mac, original] = await Promise.all([
          pixels(engine.exportPng(store.getState().activeId)), pixels(decode(reference)), pixels(decode(input)),
        ]);
        const [left, top, w, h] = detailRect;
        const counts = new Uint32Array(256);
        let sum = 0, detail = 0, effect = 0, macEffect = 0;
        for (let y = 0; y < height; y++) {
          for (let x = 0; x < width; x++) {
            const i = 4 * (y * width + x);
            if (actual[i + 3] !== 255 || mac[i + 3] !== 255) throw Error("Lost photo opacity");
            const inDetail = x >= left && x < left + w && y >= top && y < top + h;
            for (let c = 0; c < 3; c++) {
              const delta = Math.abs(actual[i + c] - mac[i + c]);
              counts[delta]++;
              sum += delta;
              if (inDetail) detail += delta;
              effect += Math.abs(actual[i + c] - original[i + c]);
              macEffect += Math.abs(mac[i + c] - original[i + c]);
            }
          }
        }
        const samples = width * height * 3;
        let cumulative = 0;
        const p95 = counts.findIndex(count => (cumulative += count) >= 0.95 * samples);
        return { mae: sum / samples, p95, detailMae: detail / (w * h * 3),
          change: effect / samples, macChange: macEffect / samples };
      }, { reference, input, width: photo.width, height: photo.height, detailRect: photo.detailRect });
      await info.attach("photo-comparison", { body: JSON.stringify(result), contentType: "application/json" });
      expect(result.macChange).toBeGreaterThan(0);
      expect(result.change).toBeGreaterThan(result.macChange / 4);
      // Compare the full photo and its selected edge to the reviewed port on these same Mac references.
      // Fractions below one 8-bit channel level allow for cross-host raster rounding.
      expect(result.mae).toBeLessThanOrEqual(photo[kind].mae + 0.15);
      expect(result.p95).toBeLessThanOrEqual(photo[kind].p95 + 1);
      expect(result.detailMae).toBeLessThanOrEqual(photo[kind].detailMae + 0.35);
    });
  }
}
