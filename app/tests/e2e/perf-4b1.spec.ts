import { test, expect } from "@playwright/test";

// Phase 4b-1's timings (LL-073), in the release engine and on the real GPU: run `pnpm wasm`, then
// `$env:PERF = "1"; npx playwright test app/tests/e2e/perf-4b1.spec.ts`, and rebuild `pnpm wasm:dev`
// afterwards. The installed Edge runs WebGL on the GPU as WebView2 does (Playwright's own Chromium
// falls back to software GL). Each test prints what it measured and checks its budget; the plan's
// tasks record the numbers.
test.skip(!process.env.PERF, "set PERF=1 after pnpm wasm to measure");
test.use({ channel: "msedge", viewport: { width: 1440, height: 900 } });

/** Opens the page and waits for the engine; every texture upload is counted in `__uploads`. */
async function ready(page: import("@playwright/test").Page) {
  await page.addInitScript(() => {
    const log = { image: 0, sub: 0 };
    (window as any).__uploads = log;
    const proto = WebGL2RenderingContext.prototype as any;
    const image = proto.texImage2D, sub = proto.texSubImage2D;
    proto.texImage2D = function (...args: unknown[]) { log.image++; return image.apply(this, args); };
    proto.texSubImage2D = function (...args: unknown[]) { log.sub++; return sub.apply(this, args); };
  });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
}

/** Installs `__frame()` in the page: one render of the active document through the app's renderer,
 * ended by a 1 x 1 readPixels so the GPU work lands inside the timing; returns its milliseconds.
 * `__lastUploads` then says how many whole and partial texture uploads that frame made. */
async function installFrameTimer(page: import("@playwright/test").Page) {
  await page.evaluate(() => {
    const api = (window as any).__compositor;
    const gl = (document.querySelector('[data-testid="canvas-view"] canvas') as HTMLCanvasElement).getContext("webgl2")!;
    const px = new Uint8Array(4);
    (window as any).__frame = () => {
      const s = api.store.getState();
      const log = (window as any).__uploads; log.image = 0; log.sub = 0;
      const t0 = performance.now();
      api.renderer.render(api.engine, s.documents[s.activeId], s.viewports[s.activeId], window.devicePixelRatio || 1, { checkerboard: true }, s.previewEdit());
      gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px);
      const ms = performance.now() - t0;
      (window as any).__lastUploads = `${log.image} whole, ${log.sub} partial`;
      return ms;
    };
  });
}

test("partial uploads: the frame after an edit inside a selection, at fit and at 1:1, 24 and 100 MP", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
    // A fresh page per size: nothing of the other size's document is left in memory or on the GPU.
    await ready(page);
    await installFrameTimer(page);
    const r = await page.evaluate(async ([w, h]) => {
      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const layer = api.engine.state(doc).layers[0].id;
      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
      api.store.getState().openDocument(doc);
      await settle(); frame();
      const result: Record<string, number> = {};
      // A 1024 x 1024 selection in the middle, cleared: the engine's edit, then the next frame.
      const box = (x: number, y: number) => ({ type: "SelectShape", kind: "Rectangle", points: [[x, y], [x + 1024, y], [x + 1024, y + 1024], [x, y + 1024]], mode: "Replace", antialiased: false });
      for (const zoom of ["fit", "1:1"]) {
        if (zoom === "1:1") { await api.setZoom(1); await settle(); frame(); }
        const at = zoom === "fit" ? [w / 2 - 1500, h / 2 - 1500] : [w / 2 - 512, h / 2 - 512];
        api.engine.execute(doc, box(at[0], at[1]));
        api.store.getState().refresh(doc); frame(); await settle();
        let t0 = performance.now();
        api.engine.execute(doc, { type: "ClearSelectedPixels", id: layer, mask: false });
        result[`clear in 1024 px at ${zoom}, engine`] = Math.round(performance.now() - t0);
        api.store.getState().refresh(doc);
        result[`frame after it at ${zoom}`] = Math.round(10 * frame()) / 10;
        // The frame just rendered: how many whole (texImage2D) and partial (texSubImage2D) uploads
        // it made, so the test can assert this edit uploaded only a rectangle, never a whole texture.
        result[`partial edit whole uploads at ${zoom}`] = (window as any).__uploads.image;
        result[`partial edit sub uploads at ${zoom}`] = (window as any).__uploads.sub;
        await settle();
        // For comparison: a whole-layer edit, and the frame after it (halving and a whole upload).
        api.engine.execute(doc, { type: "Deselect" });
        t0 = performance.now();
        api.engine.execute(doc, { type: "InvertPixels", id: layer, mask: false });
        result[`invert (whole) at ${zoom}, engine`] = Math.round(performance.now() - t0);
        api.store.getState().refresh(doc);
        result[`frame after the whole edit at ${zoom}`] = Math.round(frame());
        result[`whole edit whole uploads at ${zoom}`] = (window as any).__uploads.image;
        await settle();
      }
      api.store.getState().closeDocument(doc);
      return result;
    }, [w, h]);
    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
  }
  console.log(`partial uploads (release wasm, Edge): ${JSON.stringify(out)}`);
  for (const label of ["24 MP", "100 MP"]) for (const zoom of ["fit", "1:1"]) {
    expect(out[`${label}: frame after it at ${zoom}`]).toBeLessThan(33);
    // Ruling M4: the partial edit's frame uploads no whole texture and at least one partial one.
    expect(out[`${label}: partial edit whole uploads at ${zoom}`], "no whole-layer upload for a partial edit").toBe(0);
    expect(out[`${label}: partial edit sub uploads at ${zoom}`], "the partial edit uploads via texSubImage2D").toBeGreaterThan(0);
    // For comparison: the whole-layer edit's frame does upload a fresh texture.
    expect(out[`${label}: whole edit whole uploads at ${zoom}`], "the whole-layer edit uploads a fresh texture").toBeGreaterThan(0);
  }
});

test("history: whole-layer edits at 24 and 100 MP stay within memory, and a push or undo at the cap is cheap", async ({ page }) => {
  test.setTimeout(900_000);
  await ready(page);
  const out = await page.evaluate(() => {
    const api = (window as any).__compositor;
    const result: Record<string, number | number[]> = {};
    for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const layer = api.engine.state(doc).layers[0].id;
      const times: number[] = [];
      for (let i = 0; i < 20; i++) {
        const t0 = performance.now();
        api.engine.execute(doc, { type: "InvertPixels", id: layer, mask: false });
        times.push(Math.round(performance.now() - t0));
      }
      const s = api.engine.state(doc);
      result[`${label}: invert ms (20 edits)`] = times;
      result[`${label}: undo depth after 20 edits`] = s.undoDepth;
      result[`${label}: wasm MB after 20 edits`] = Math.round(api.engine.wasmBytes() / 1048576);
      api.engine.closeDocument(doc);
    }
    // 1000 layers with pixels of their own and a full history: each push trims one entry.
    const doc = api.engine.newDocument(200, 200, false);
    const png = api.engine.exportPng(api.engine.newDocument(8, 8, true));
    for (let i = 0; i < 1000; i++) api.engine.importImage(doc, png, `L${i}`, { x: 100, y: 100 });
    const layer = api.engine.state(doc).layers[0].id;
    for (let i = 0; i < 100; i++) api.engine.execute(doc, { type: "RenameLayer", id: layer, name: `n${i}` });
    const pushes: number[] = [];
    for (let i = 0; i < 10; i++) {
      const t0 = performance.now();
      api.engine.execute(doc, { type: "RenameLayer", id: layer, name: `m${i}` });
      pushes.push(performance.now() - t0);
    }
    result["1000 layers, 100 entries: rename at the cap, ms (mean of 10)"] = Math.round(10 * pushes.reduce((a, b) => a + b, 0) / pushes.length) / 10;
    result["1000 layers: undo depth"] = api.engine.state(doc).undoDepth;
    // Ruling I2: undo at the cap gets the same budget as the push at the cap above. An undo at the
    // ENTRY cap keeps the count at 100 (moving an entry from undo to redo does not change the
    // total, so trim has nothing to drop): this times the bookkeeping (hold/release/id-tracking),
    // not a trim.
    const undos: number[] = [];
    for (let i = 0; i < 10; i++) {
      const t0 = performance.now();
      api.engine.undo(doc);
      undos.push(performance.now() - t0);
    }
    result["1000 layers, 100 entries: undo at the cap, ms (mean of 10)"] = Math.round(10 * undos.reduce((a, b) => a + b, 0) / undos.length) / 10;
    result["1000 layers: undo depth after 10 undos"] = api.engine.state(doc).undoDepth;
    return result;
  });
  console.log(`history (release wasm): ${JSON.stringify(out)}`);
  expect(out["100 MP: wasm MB after 20 edits"]).toBeLessThan(2560);
  expect(out["1000 layers: undo depth"]).toBe(100);
  expect(out["1000 layers, 100 entries: rename at the cap, ms (mean of 10)"]).toBeLessThan(5 + 20);
  expect(out["1000 layers: undo depth after 10 undos"]).toBe(90);
  // No looser than the push-at-cap budget just above (ruling I2).
  expect(out["1000 layers, 100 entries: undo at the cap, ms (mean of 10)"]).toBeLessThan(5 + 20);
});
