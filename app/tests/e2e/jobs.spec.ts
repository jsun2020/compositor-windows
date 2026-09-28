import { test, expect, type Page } from "@playwright/test";
import { noisePngBase64 } from "./helpers";

// Phase 4b-1: the job worker, a second engine in a Web Worker (engine/tests/jobs.rs pins the engine
// side). Every layer here counts as large (`jobPixels` 0), so the panels' commits and the Levels
// histogram go through the worker, and each result is compared with the same edit made in place.

/** Two documents holding the same noise layer, the first open; every layer counts as large. Also
 * wraps `JobClient.run` to record each job's kind, in order, so a test can prove its edit or its
 * histogram actually crossed into the worker (ruling I5) rather than the store silently having
 * fallen back to the UI thread. Read back with `jobKinds`. */
async function setup(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(noisePngBase64);
  return page.evaluate(async (data) => {
    const api = (window as any).__compositor;
    const png = Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const make = () => { const doc = api.engine.newDocument(80, 70, false); api.engine.importImage(doc, png, "noise", { x: 41, y: 33 }); return doc; };
    const [here, there] = [make(), make()];
    api.store.setState({ jobPixels: 0 });
    api.store.getState().openDocument(there);
    const jobs = api.store.getState().jobs;
    const kinds: string[] = [];
    const original = jobs.run.bind(jobs);
    jobs.run = (channel: string, request: { kind: string }) => { kinds.push(request.kind); return original(channel, request); };
    (window as any).__jobKinds = kinds;
    return { there, here, layer: api.engine.state(there).layers[0].id, other: api.engine.state(here).layers[0].id };
  }, b64);
}
const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
const idle = (page: Page) => page.waitForFunction(() => !(window as any).__compositor.store.getState().working);
const jobKinds = (page: Page) => page.evaluate(() => (window as any).__jobKinds as string[]);
/** The largest byte difference between the two documents' composites. */
const worst = (page: Page, a: string, b: string) => page.evaluate(([a, b]) => {
  const api = (window as any).__compositor;
  const region = { x: 0, y: 0, width: 80, height: 70 };
  const x = api.engine.composite(a, region, 80, 70) as Uint8Array, y = api.engine.composite(b, region, 80, 70) as Uint8Array;
  let d = 0; for (let i = 0; i < x.length; i++) d = Math.max(d, Math.abs(x[i] - y[i]));
  return d;
}, [a, b]);

test("Levels on a large layer: the panel opens at once, its histogram comes from the worker, and OK applies through it as one step", async ({ page }) => {
  const ids = await setup(page);
  const depth = (await state(page)).undoDepth;
  await page.evaluate(() => (window as any).__compositor.store.getState().beginAdjust({ kind: "Levels" }));
  // The histogram the worker read is the one the engine reads in place.
  await page.waitForFunction(() => (window as any).__compositor.store.getState().adjustEdit?.histogram !== null);
  await expect(page.getByTestId("histogram-pending")).toHaveCount(0);
  const [fromWorker, inPlace] = await page.evaluate((ids) => {
    const api = (window as any).__compositor;
    return [api.store.getState().adjustEdit.histogram, api.engine.histogram(ids.there, ids.layer)];
  }, ids);
  expect(fromWorker).toEqual(inPlace);
  await page.getByLabel("Output white").fill("190");
  await page.getByLabel("Gamma").fill("1.3");
  const adjustment = await page.evaluate(() => (window as any).__compositor.store.getState().adjustEdit.adjustment);
  await page.getByRole("button", { name: "OK" }).click();
  await idle(page);
  expect((await state(page)).undoDepth).toBe(depth + 1);
  await page.evaluate(([ids, adjustment]) => (window as any).__compositor.engine.execute(ids.here, { type: "ApplyAdjustment", id: ids.other, adjustment }), [ids, adjustment] as const);
  expect(await worst(page, ids.there, ids.here)).toBe(0);
  // Ruling I5: prove the work actually crossed into the worker -- the histogram, then the commit --
  // rather than the store having silently fallen back to running it on the UI thread.
  expect(await jobKinds(page), "the histogram and the commit both ran in the job worker").toEqual(["histogram", "edit"]);
});

test("Levels under a selection on a large layer: the worker's histogram and commit clip to it, exactly as in place", async ({ page }) => {
  // Fix round 1, issue 3: exercises the points path end to end -- a job's selection travels as a
  // separate byte buffer (engine jobs.rs's JobSelection), never JSON, and only a real selection
  // makes it non-null (EngineClient.jobInput). Both prior jobs.spec.ts tests have no selection at
  // all, so a swapped mask/points argument order in job-worker.ts (jobs.ts and job-worker.ts pass
  // pixels, mask, points, in that order to run_edit_job/run_histogram_job) would pass every other
  // test here undetected.
  const ids = await setup(page);
  const box = { type: "SelectShape", kind: "Rectangle", points: [[10, 10], [60, 10], [60, 50], [10, 50]], mode: "Replace", antialiased: false };
  await page.evaluate(([ids, box]) => {
    const api = (window as any).__compositor;
    api.engine.execute(ids.there, box);
    api.store.getState().refresh(ids.there);
    api.engine.execute(ids.here, box);
  }, [ids, box] as const);
  // Taken after the selection: the SelectShape command above records its own undo entry.
  const depth = (await state(page)).undoDepth;
  await page.evaluate(() => (window as any).__compositor.store.getState().beginAdjust({ kind: "Levels" }));
  await page.waitForFunction(() => (window as any).__compositor.store.getState().adjustEdit?.histogram !== null);
  const [fromWorker, inPlace] = await page.evaluate((ids) => {
    const api = (window as any).__compositor;
    return [api.store.getState().adjustEdit.histogram, api.engine.histogram(ids.there, ids.layer)];
  }, ids);
  // The worker's histogram, read inside the selection, matches the engine's own -- proves the
  // selection's points actually reached the worker and were applied, not silently dropped or
  // swapped with something else.
  expect(fromWorker).toEqual(inPlace);
  await page.getByLabel("Output white").fill("190");
  await page.getByLabel("Gamma").fill("1.3");
  const adjustment = await page.evaluate(() => (window as any).__compositor.store.getState().adjustEdit.adjustment);
  await page.getByRole("button", { name: "OK" }).click();
  await idle(page);
  expect((await state(page)).undoDepth).toBe(depth + 1);
  await page.evaluate(([ids, adjustment]) => (window as any).__compositor.engine.execute(ids.here, { type: "ApplyAdjustment", id: ids.other, adjustment }), [ids, adjustment] as const);
  expect(await worst(page, ids.there, ids.here)).toBe(0);
  expect(await jobKinds(page), "the histogram and the commit both ran in the job worker, under the selection").toEqual(["histogram", "edit"]);
});

test("a blur on a large layer grows it through the worker exactly as it does in place", async ({ page }) => {
  const ids = await setup(page);
  await page.evaluate(() => (window as any).__compositor.store.getState().beginAdjust({ kind: "GaussianBlur" }));
  const params = { filter: "GaussianBlur", radius: 7 };
  await page.evaluate((params) => (window as any).__compositor.store.getState().updateAdjust({ params }), params);
  await page.getByRole("button", { name: "OK" }).click();
  await idle(page);
  await page.evaluate(([ids, params]) => (window as any).__compositor.engine.execute(ids.here, { type: "ApplyFilter", id: ids.other, params }), [ids, params] as const);
  const [a, b] = await page.evaluate((ids) => {
    const api = (window as any).__compositor;
    return [api.engine.state(ids.there).layers[0], api.engine.state(ids.here).layers[0]];
  }, ids);
  expect([a.pixelsWidth, a.pixelsHeight, a.transform]).toEqual([b.pixelsWidth, b.pixelsHeight, b.transform]);
  expect(a.pixelsWidth, "the blur grew the layer").toBeGreaterThan(64);
  expect(await worst(page, ids.there, ids.here)).toBe(0);
  // Ruling I5: the blur's commit ran in the worker, not silently on the UI thread.
  expect(await jobKinds(page), "the commit ran in the job worker").toEqual(["edit"]);
});
