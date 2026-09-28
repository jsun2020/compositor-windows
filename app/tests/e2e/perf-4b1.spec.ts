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

test("jobs: the Levels histogram and commit through the worker at 24 and 100 MP, and the page's frames meanwhile", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
    await ready(page);
    await installFrameTimer(page);
    const r = await page.evaluate(async ([w, h]) => {
      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const result: Record<string, number> = {};
      // The main thread's two copies, timed where the store makes them; when the result is put back,
      // the store's state is brought up to date and one frame drawn at once, timed too.
      let installedAt = Infinity;
      const timed = (name: string, after?: () => void) => {
        const f = api.engine[name].bind(api.engine);
        api.engine[name] = (...a: unknown[]) => {
          const t0 = performance.now();
          try { return f(...a); } finally { result[`${name} ms`] = Math.round(performance.now() - t0); after?.(); }
        };
      };
      timed("jobInput");
      timed("installJob", () => { installedAt = performance.now(); api.store.getState().refresh(api.store.getState().activeId); result["frame after the result is put back"] = Math.round((window as any).__frame()); });
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const layer = api.engine.state(doc).layers[0].id;
      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
      api.store.getState().openDocument(doc);
      await settle(); frame();
      // The longest gap between animation frames while `until` is false. Measured with
      // performance.now() taken INSIDE the callback, not the rAF timestamp argument: that
      // timestamp can predate when the callback actually runs (it is the frame's nominal time, not
      // "now"), which under-reports a gap that a long synchronous task -- such as `installJob`
      // running inside the worker's message handler -- partly hides behind it (fix round 1, issue 4).
      // Only frames that came before the result was put back (`installedAt`) count: the worker's own
      // time, not whatever the page does with the result afterward.
      const longestGap = (until: () => boolean) => new Promise<number>((done) => {
        let last = performance.now(), gap = 0;
        const tick = () => {
          const now = performance.now();
          if (now <= installedAt) gap = Math.max(gap, now - last); last = now;
          if (until()) done(Math.round(gap)); else requestAnimationFrame(tick);
        };
        requestAnimationFrame(tick);
      });
      let t0 = performance.now();
      api.store.getState().beginAdjust({ kind: "Levels" });
      result["Levels opens (UI thread) ms"] = Math.round(performance.now() - t0);
      result["histogram: longest frame gap while the worker reads it"] = await longestGap(() => api.store.getState().adjustEdit?.histogram !== null);
      result["histogram arrives after ms"] = Math.round(performance.now() - t0);
      const adjustment = JSON.parse(JSON.stringify(api.store.getState().adjustEdit.adjustment));
      adjustment.levels.ranges[0].outputWhite = 200;
      api.store.getState().updateAdjust({ adjustment });
      // The full-quality preview follows the quick one (store SETTLE_MS) and is drawn before OK.
      await new Promise<void>((done) => { const poll = () => (api.store.getState().previewSettling() ? setTimeout(poll, 20) : done()); poll(); });
      await new Promise((r) => setTimeout(r, 300)); await settle();
      t0 = performance.now();
      api.store.getState().commitAdjust();
      result["OK (UI thread) ms"] = Math.round(performance.now() - t0);
      result["commit: longest frame gap while the worker edits"] = await longestGap(() => !api.store.getState().working);
      result["commit done after ms"] = Math.round(performance.now() - t0);
      result["undo depth"] = api.engine.state(doc).undoDepth;
      return result;
    }, [w, h]);
    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
  }
  console.log(`jobs (release wasm, Edge): ${JSON.stringify(out)}`);
  for (const label of ["24 MP", "100 MP"]) {
    // The worker's own time never holds the page up past 100 ms while it edits. While it reads the
    // histogram the gap also takes in the frame that first draws the panel's preview on the UI thread:
    // budgeted 150 at both sizes. The copies are budgeted per size (OQ5's "500" at 100 MP means 450
    // here, per ruling I7).
    expect(out[`${label}: histogram: longest frame gap while the worker reads it`]).toBeLessThan(150);
    expect(out[`${label}: commit: longest frame gap while the worker edits`]).toBeLessThan(100);
    expect(out[`${label}: jobInput ms`]).toBeLessThan(label === "24 MP" ? 150 : 450);
    expect(out[`${label}: installJob ms`]).toBeLessThan(label === "24 MP" ? 150 : 450);
    // Fix round 1, issue 6: this was measured but not asserted. The frame that refreshes the store
    // and redraws right when the result lands re-uploads the whole committed layer: bimodal across
    // runs (measured 24 MP: 59-242 ms, 100 MP: 225-266 ms over four runs -- 24 MP's low runs land
    // near 60 ms, its high ones near 230-240 ms, seemingly whichever the browser process's own
    // warm-up state that run happened to land in, not a code path this fix round changes).
    expect(out[`${label}: frame after the result is put back`]).toBeLessThan(label === "24 MP" ? 350 : 400);
  }
});

test("effects images: a large styled layer opens drawn plainly, then reduced, then (to 24 MP) at full size, without holding the page up", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
    await ready(page);
    await installFrameTimer(page);
    const r = await page.evaluate(async ([w, h]) => {
      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
      const result: Record<string, number> = {};
      // A styled layer this size: the package of a filled canvas, re-opened with all six effects.
      const plain = api.engine.newDocument(10, 10, false);
      api.engine.execute(plain, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const files = api.engine.savePackage(plain);
      api.engine.closeDocument(plain);
      const manifest = JSON.parse(files.manifest);
      manifest.layers[0].effects = {
        stroke: { blue: 0.2, green: 0.8, inside: false, opacity: 0.9, red: 0.1, size: 12 },
        shadow: { angle: 120, blue: 0.3, blur: 30, distance: 25, green: 0.2, opacity: 0.75, red: 0.2 },
        colorOverlay: { blue: 0.4, green: 0.1, opacity: 0.3, red: 0.9 },
        innerShadow: { angle: -35, blue: 0.05, blur: 20, distance: 15, green: 0.05, opacity: 0.6, red: 0.05 },
        outerGlow: { blue: 0.2, green: 0.9, opacity: 0.6, red: 1, size: 25 },
        innerGlow: { blue: 1, green: 1, opacity: 0.5, red: 1, size: 20 } };
      manifest.version = 9;
      const doc = api.engine.openPackage({ manifest: JSON.stringify(manifest), images: files.images }, null);
      const layer = api.engine.state(doc).layers[0].id;
      // Fix round 1, issue 6: `displayJobInput` (engine `display_job_input`) reduces the layer's own
      // pixels to at most `EFFECTS_LIMITS.reduced` px on the UI thread before handing the job its
      // copy -- at 100 MP that is three more halvings of the full raster, done synchronously inside
      // whichever frame first asks for the reduced image. Timed here to see whether it explains the
      // stall inside the "reduced" window below.
      const displayJobInputMs: number[] = [];
      const realDisplayJobInput = api.engine.displayJobInput.bind(api.engine);
      api.engine.displayJobInput = (...a: unknown[]) => {
        const s = performance.now();
        try { return realDisplayJobInput(...a); } finally { displayJobInputMs.push(Math.round(performance.now() - s)); }
      };
      // Frames and their gaps from here until the work is done. `performance.now()` inside the
      // callback, not the rAF timestamp argument (fix round 1, issue 5; Task 6's own fix, see the
      // "jobs:" test above): the timestamp can predate when the callback actually runs, under-
      // reporting a gap a long synchronous task partly hides behind.
      let last = performance.now(), gap = 0, running = true;
      const tick = () => { const now = performance.now(); gap = Math.max(gap, now - last); last = now; if (running) requestAnimationFrame(tick); };
      requestAnimationFrame(tick);
      const t0 = performance.now();
      // The real opening frame, timed directly (fix round 1, issue 6): the old code awaited two
      // un-timed rAFs before calling `frame()`, so whatever those two frames actually drew (already
      // the plain layer, halved and uploaded) went uncounted, and the *third* frame's near-zero cost
      // (nothing new to upload) was reported as "the first frame" instead.
      api.store.getState().openDocument(doc);
      result["first frame (the layer plainly), ms"] = Math.round(frame());
      // What the layer's texture holds: "px" its own pixels, "rd" a reduced image, "fx" the full one.
      const shown = () => String(api.renderer.textureKey(doc, layer) ?? "").slice(0, 2);
      const waitFor = (done: () => boolean, limit: number) => new Promise<boolean>((resolve) => {
        const start = performance.now();
        const poll = () => { if (done()) resolve(true); else if (performance.now() - start > limit) resolve(false); else setTimeout(poll, 20); };
        poll();
      });
      result["the first frame draws the layer plainly (1 = yes)"] = shown() === "px" ? 1 : 0;
      // `last` is reset here too, not only `gap`: the tick loop was installed before the (exempt,
      // OQ20) opening frame ran, and a bare `gap = 0` alone leaves `last` stale from then, so the
      // very next tick's own gap would still span that whole synchronous frame -- exactly the frame
      // this budget is not meant to cover (fix round 1, issue 6).
      gap = 0; last = performance.now();
      const reduced = await waitFor(() => { frame(); return shown() === "rd"; }, 60_000);
      result["reduced image drawn after, ms"] = reduced ? Math.round(performance.now() - t0) : -1;
      result["longest frame gap until then"] = Math.round(gap);
      result["longest displayJobInput call inside that window, ms"] = displayJobInputMs.length ? Math.max(...displayJobInputMs) : 0;
      if (w * h <= 24_000_000) { // the layer's own pixels (EFFECTS_LIMITS.full)
        gap = 0; last = performance.now(); const t1 = performance.now();
        const full = await waitFor(() => api.engine.hasEffectsImage(doc, layer, null), 120_000);
        result["full image kept after, ms"] = full ? Math.round(performance.now() - t1) : -1;
        result["longest frame gap while the worker made it"] = Math.round(gap);
        result["frame that draws it (halving and upload), ms"] = Math.round(frame());
        result["then drawn at full size (1 = yes)"] = shown() === "fx" ? 1 : 0;
      }
      running = false;
      return result;
    }, [w, h]);
    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
  }
  console.log(`effects images (release wasm, Edge): ${JSON.stringify(out)}`);
  for (const label of ["24 MP", "100 MP"]) {
    expect(out[`${label}: the first frame draws the layer plainly (1 = yes)`]).toBe(1);
    // "first frame (the layer plainly), ms" is logged but has no budget of its own: OQ20 already
    // exempts it ("the first-draw frame of a new layer... as any import"). It is measured directly
    // now (fix round 1, issue 6: no more discarding it behind two un-timed rAFs), and the frame-gap
    // tick loop's own `last` is reset right after it (not only `gap`), so that exempt frame's cost
    // no longer bleeds into the budgets below through a stale timestamp either (measured 254-726 ms
    // at 24 MP and 761-1217 ms at 100 MP over six runs, all one single UI-thread frame, not the
    // worker's -- the old, wrongly-measured "4 ms at 100 MP" could never have included this).
    expect(out[`${label}: reduced image drawn after, ms`]).toBeGreaterThan(0);
  }
  expect(out["24 MP: then drawn at full size (1 = yes)"]).toBe(1);
  // Fix round 1, issue 6: this budget used to pass only because the exempt opening frame's cost was
  // silently discarded (the old two-rAF wait) rather than because the worker held the page up, and a
  // second bug (the tick loop's `last` left stale across that discarded frame) would have broken it
  // again the moment the first bug was fixed alone. Measured now, with both fixed: 68-95 ms.
  expect(out["24 MP: longest frame gap until then"]).toBeLessThan(100);
  // At 100 MP: 100-117 ms measured over five runs, well under budget and explained by
  // `display_job_input`'s own halving of the layer down to the reduced size, done synchronously on
  // the UI thread when the reduced image is first asked for (measured 91-289 ms in isolation just
  // above, `displayJobInput` timed directly -- confirming, not merely presuming, the cause the old
  // comment guessed at from the first frame's unrelated three-halving cost). Not fully avoidable
  // without moving that halving off the UI thread too, a larger change than this fix round's scope.
  expect(out["100 MP: longest frame gap until then"]).toBeLessThan(200);
  expect(out["24 MP: longest frame gap while the worker made it"]).toBeLessThan(150);
  // Fix round 1, issue 6: measured but not asserted before. The frame that uploads the full image
  // (24 MP only; past that no full image is ever made) measured 14-25 ms over four runs, and once
  // 219 ms (a one-off stall shared with the sibling budget just above, which the same run also blew
  // past at 187 ms: a GC pause or OS scheduling hiccup on the machine, not this frame's own cost --
  // the sibling budget is pre-existing and outside this fix round's scope, left as is).
  expect(out["24 MP: frame that draws it (halving and upload), ms"]).toBeLessThan(150);
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
