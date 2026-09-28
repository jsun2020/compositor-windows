import { test, expect, type Page } from "@playwright/test";
import { softBlobPngBase64 } from "./helpers";

// Phase 4b-1: a styled layer too large to make its effects image on the UI thread is drawn plainly
// until the job worker's reduced image lands, then from it, then from the full-size image, which is
// the CPU compositor's own (engine/tests/jobs.rs pins the images). The sizes that choose the path
// are lowered here (`effectsLimits`) so a small layer takes it.

const DOC = "0B6C6B1E-4F1B-4B4E-9E0A-EFEFEFEFEF10";
const STYLED = "0B6C6B1E-4F1B-4B4E-9E0A-EFEFEFEFEF12";
const EFFECTS = {
  stroke: { blue: 0.2, green: 0.8, inside: false, opacity: 0.9, red: 0.1, size: 3 },
  shadow: { angle: 120, blue: 0.3, blur: 6, distance: 7, green: 0.2, opacity: 0.75, red: 0.2 },
  outerGlow: { blue: 0.2, green: 0.9, opacity: 0.6, red: 1, size: 5 },
};

/** The 40 x 24 soft blob at (28, 24) on a 96 x 73 canvas (odd, so 1:1 lands on whole device pixels in
 * this viewport), styled, alone. */
async function open(page: Page): Promise<string> {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const blob = await page.evaluate(softBlobPngBase64);
  return page.evaluate(async ({ blob, doc, styled, effects }) => {
    const api = (window as any).__compositor;
    // Everything counts as large; the reduced image is at most 32 px (the blob halved once).
    api.effectsLimits.sync = 0; api.effectsLimits.reduced = 32;
    const manifest = { format: "com.compositor.project", version: 9, colorSpace: "sRGB", documentID: doc, width: 96, height: 73,
      layers: [{ id: styled, name: "Styled", isVisible: true, imageFile: `${styled}.png`, effects,
        transform: { origin: [28, 24], size: [40, 24], rotation: 0, flipX: false, flipY: false, sampling: "High quality" } }] };
    const images = [{ name: `${styled}.png`, bytes: Uint8Array.from(atob(blob), (c: string) => c.charCodeAt(0)) }];
    const id = api.engine.openPackage({ manifest: JSON.stringify(manifest), images }, null);
    api.setCheckerboard(false);
    return id;
  }, { blob, doc: DOC, styled: STYLED, effects: EFFECTS });
}
/** One frame drawn now: how many of the GPU's pixels outside the blob's box show anything, and whether
 * the engine holds the full-size image. The picture is kept as `__pictures[name]`. (The CPU compositor
 * is not asked here: making its own image would put the full-size one in the engine's cache.) */
const look = (page: Page, name: string) => page.evaluate((name) => {
  const api = (window as any).__compositor;
  const s = api.store.getState();
  api.renderer.render(api.engine, s.documents[s.activeId], s.viewports[s.activeId], window.devicePixelRatio || 1, { checkerboard: false }, null);
  const gpu = api.readDocumentPixels() as Uint8Array;
  const pictures = ((window as any).__pictures ??= {});
  pictures[name] = gpu;
  let outside = 0;
  for (let y = 0; y < 73; y++) for (let x = 0; x < 96; x++) if ((x < 28 || x >= 68 || y < 24 || y >= 48) && gpu[(y * 96 + x) * 4 + 3] > 0) outside++;
  return { outside, full: api.engine.hasEffectsImage(s.activeId, s.documents[s.activeId].layers[0].id, null) as boolean };
}, name);
/** The largest channel difference between two kept pictures, or the kept `a` and the CPU's composite. */
const worst = (page: Page, a: string, b: string | "cpu") => page.evaluate(([a, b]) => {
  const api = (window as any).__compositor; const s = api.store.getState();
  const x = (window as any).__pictures[a] as Uint8Array;
  const y = b === "cpu" ? api.engine.composite(s.activeId, { x: 0, y: 0, width: 96, height: 73 }, 96, 73) as Uint8Array : (window as any).__pictures[b] as Uint8Array;
  let d = 0; for (let i = 0; i < x.length; i++) d = Math.max(d, Math.abs(x[i] - y[i]));
  return d;
}, [a, b]);

test("a styled layer too large for the UI thread is drawn plainly, then from the worker's reduced image, then at full size as the CPU draws it", async ({ page }) => {
  const doc = await open(page);
  // A worker whose every job comes to nothing: nothing but the layer itself can be drawn.
  await page.evaluate(async (doc) => {
    const api = (window as any).__compositor;
    (window as any).__realJobs = api.store.getState().jobs;
    api.store.setState({ jobs: { run: () => Promise.resolve(null), cancel: () => {} } });
    api.effectsLimits.full = 0;
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
  }, doc);
  // Compared pixel for pixel, so the document must sit on whole device pixels (LL-065(6)): checked.
  const whole = await page.evaluate(() => {
    const s = (window as any).__compositor.store.getState(); const d = s.documents[s.activeId]; const dpr = window.devicePixelRatio || 1;
    const r = s.viewports[s.activeId].documentRect({ width: d.width, height: d.height });
    return Number.isInteger(r.x * dpr) && Number.isInteger(r.y * dpr);
  });
  expect(whole, "the document sits on whole device pixels").toBe(true);
  expect((await look(page, "plain")).outside, "no effects: the layer alone").toBe(0);
  // The real worker, reduced images only: the effects appear.
  await page.evaluate(() => { const api = (window as any).__compositor; api.store.setState({ jobs: (window as any).__realJobs }); api.store.getState().invalidate(); });
  await expect.poll(async () => (await look(page, "reduced")).outside, { timeout: 15_000 }).toBeGreaterThan(50);
  expect((await look(page, "reduced")).full).toBe(false);
  // Full size allowed again: the worker's image goes into the engine's cache and the GPU draws it.
  await page.evaluate(() => { const api = (window as any).__compositor; api.effectsLimits.full = 24_000_000; api.store.getState().invalidate(); });
  await expect.poll(async () => (await look(page, "full")).full, { timeout: 15_000 }).toBe(true);
  expect((await look(page, "full")).outside).toBeGreaterThan(50);
  expect(await worst(page, "full", "cpu"), "the full-size image is the CPU's").toBeLessThanOrEqual(2);
  expect(await worst(page, "reduced", "full"), "the reduced image was a reduced one").toBeGreaterThan(2);
});
