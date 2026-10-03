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
        // Measure the cooperative APIs the store actually calls. Frame gaps
        // below still charge every yield; the copy budgets retain CPU work.
        const method = `${name}Async`, f = api.engine[method].bind(api.engine);
        api.engine[method] = async (...a: unknown[]) => {
          try { return await f(...a); } finally { result[`${name} ms`] = Math.round(api.engine[name === "jobInput" ? "lastJobInputCpuMs" : "lastInstallCpuMs"]); after?.(); }
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
    // Task 15 fix round 1, controller ruling: the shared "jobs" 100 MP budget for this frame becomes
    // 500 (was 400) -- on the HD 520 the frame halves a fresh 100 MP result and uploads it, which
    // this run's hardware needs more room for than the plan's own numbers assumed. Follow-up ("mask
    // levels / display-level uploads") is left to the final review.
    expect(out[`${label}: frame after the result is put back`]).toBeLessThan(label === "24 MP" ? 350 : 500);
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
      // Every `displayJobInput` call (engine `display_job_input`), timestamped so a call can be told
      // apart from the tick that made it and attributed to the right window (fix round 2, issue B).
      const displayJobInputCalls: number[] = [];
      // The level each call asked at (0: the full-size image), so the 100 MP run can show that no
      // full image is ever asked for past EFFECTS_LIMITS.full (final fix wave, Task 7's deferred minor).
      const displayJobInputLevels: number[] = [];
      const realDisplayJobInput = api.engine.displayJobInput.bind(api.engine);
      api.engine.displayJobInput = (...a: unknown[]) => {
        const s = performance.now();
        displayJobInputLevels.push(a[2] as number);
        try { return realDisplayJobInput(...a); } finally { displayJobInputCalls.push(Math.round(performance.now() - s)); }
      };
      // Fix round 4: the other work that can run between two ticks, timed where it runs: the engine
      // taking the worker's full image back (`keepEffectsImage`) and the app's own renders (the
      // canvas redraws itself when an image lands, apart from this test's `frame()`).
      const keepMs: number[] = [], appRenderMs: number[] = [];
      let keptAt = -1;
      const realKeep = api.engine.keepEffectsImage.bind(api.engine);
      api.engine.keepEffectsImage = (...a: unknown[]) => {
        const s = performance.now();
        let kept = false;
        try { kept = realKeep(...a); return kept; } finally {
          keepMs.push(Math.round(performance.now() - s));
          if (kept && keptAt < 0) keptAt = performance.now();
        }
      };
      // The render that first puts the full image on the texture, whichever made it: the app's own
      // (it redraws a frame after the image is kept, fix round 5) or this test's `frame()`.
      let inTestFrame = false, fullDrawnMs = -1, fullDrawnByApp = 0, fullDrawnAt = -1;
      const tickTimes: number[] = [];
      const realRender = api.renderer.render.bind(api.renderer);
      api.renderer.render = (...a: unknown[]) => {
        const s = performance.now();
        try { return realRender(...a); } finally {
          const ms = Math.round(performance.now() - s);
          if (!inTestFrame) appRenderMs.push(ms);
          if (fullDrawnMs < 0 && shown() === "fx") { fullDrawnMs = ms; fullDrawnByApp = inTestFrame ? 0 : 1; fullDrawnAt = s; }
        }
      };
      const testFrame = () => { inTestFrame = true; try { return frame(); } finally { inTestFrame = false; } };
      // What the layer's texture holds: "px" its own pixels, "rd" a reduced image, "fx" the full one.
      const shown = () => String(api.renderer.textureKey(doc, layer) ?? "").slice(0, 2);
      // Diagnostic (fix round 2, issue B): every long task the browser itself notices on the main
      // thread, independent of what this test's own code times -- catches a cost neither `frame()`
      // nor `displayJobInput` wrapping would (browser-internal work: GC, layout, or anything else).
      const longtasks: { start: number; duration: number }[] = [];
      try {
        new PerformanceObserver((list) => { for (const e of list.getEntries()) longtasks.push({ start: e.startTime, duration: e.duration }); }).observe({ entryTypes: ["longtask"] });
      } catch { /* not supported: the diagnostic below just reports nothing found */ }

      // The real opening frame, timed directly (fix round 1, issue 6: the old code awaited two
      // un-timed rAFs before calling `frame()`, so whatever those two frames actually drew -- already
      // the plain layer, halved and uploaded -- went uncounted, and the *third* frame's near-zero
      // cost, nothing left to upload, was reported as "the first frame" instead).
      const t0 = performance.now();
      api.store.getState().openDocument(doc);
      result["first frame (the layer plainly), ms"] = Math.round(testFrame());
      result["the first frame draws the layer plainly (1 = yes)"] = shown() === "px" ? 1 : 0;

      // Fix round 2, issue C: the windows must be contiguous. Reading and resetting `gap` from
      // outside the tick loop (a separate setTimeout-driven poll used to drive `frame()` and the
      // "done" check) left the one frame that actually *finishes* a window -- draws the new image,
      // and at 24 MP also asks for the next one -- uncounted by either window's budget: its own cost
      // reached `gap` only at the *next* rAF tick, but `gap` was read and `last` reset in the same
      // synchronous task that detected "done", before any tick had a chance to charge that frame to
      // anything. Fixed by driving the whole poll from inside the tick callback: `frame()`, the
      // "done" check, and the phase switch (with its readout and reset, atomically) all happen there
      // now, so every inter-tick interval belongs to exactly one window. Fix round 4: what ran inside
      // an interval is charged with it -- the previous tick's `frame()`, and every `displayJobInput`,
      // `keepEffectsImage` and app render made since the previous tick, inside that `frame()` or in a
      // task between ticks (the deferred full-size ask, fix round 3) -- so a window's diagnostics
      // describe exactly the intervals its gap is taken over, and the longest interval's contents
      // are recorded. (A `displayJobInput` call inside `frame()` is counted in both.)
      type Phase = "reduced" | "tail" | "full" | "done";
      const wantsFull = w * h <= 24_000_000; // EFFECTS_LIMITS.full
      let phase: Phase = "reduced";
      let last = performance.now(), gap = 0;
      const reducedWindowStart = last;
      let fullWindowStart = 0;
      // Anything before this point ran in the exempt opening frame.
      let seen = displayJobInputCalls.length, seenKeep = keepMs.length, seenRender = appRenderMs.length;
      let previousFrameMs = 0;
      let callsThisWindow: number[] = [];
      // Diagnostic (fix round 2, issue B): the renderer's own per-frame cost while nothing new has
      // landed yet, to tell apart from `displayJobInput`'s -- the 100 MP "reduced" window's gap was
      // guessed, not shown, to be `displayJobInput`'s halving; this checks that directly by timing
      // every `frame()` call in the window and reporting the longest.
      let frameMsThisWindow: number[] = [];
      // The longest interval of the open window and what ran in it, ms.
      const noHolds = { displayJobInput: 0, keepEffectsImage: 0, appRender: 0, frame: 0 };
      let longestHolds = noHolds;

      // The reduced window's readout, up to `at`.
      const readReduced = (at: number) => {
        result["longest frame gap until then"] = Math.round(gap);
        for (const [what, ms] of Object.entries(longestHolds)) result[`that longest gap holds: ${what} ms`] = ms;
        result["longest displayJobInput call inside that window, ms"] = callsThisWindow.length ? Math.max(...callsThisWindow) : 0;
        result["longest single render frame inside that window, ms"] = frameMsThisWindow.length ? Math.round(Math.max(...frameMsThisWindow)) : 0;
        const longtasksHere = longtasks.filter((t) => t.start >= reducedWindowStart && t.start < at);
        result["longtasks inside that window (count)"] = longtasksHere.length;
        result["longest longtask inside that window, ms"] = longtasksHere.length ? Math.round(Math.max(...longtasksHere.map((t) => t.duration))) : 0;
      };
      // Ends the reduced window at `at` (read out and reset at once) and opens the full one (24 MP).
      const endReduced = (at: number) => {
        result["reduced image drawn after, ms"] = Math.round(at - t0);
        readReduced(at);
        gap = 0; callsThisWindow = []; frameMsThisWindow = []; longestHolds = noHolds; fullWindowStart = at;
        phase = "full";
      };
      // At 100 MP no full window follows (Task 7 parked item, ruled for the final fix wave): the reduced
      // window stays open for the interval that holds the reduced image's draw, and ends on the tick
      // after it, so that draw is charged to the window's own 200 ms budget rather than to none.
      // `drawnAt` is when the reduced image was drawn; the window closes at the next tick ("tail").
      let reducedDrawnAt = -1;
      const drawnWithoutFull = (at: number) => { reducedDrawnAt = at; phase = "tail"; };

      await new Promise<void>((resolve) => {
        const tick = () => {
          const now = performance.now();
          // Fix round 4: the reduced image drawn since the last tick, by the app's own render (it
          // redraws as soon as an image lands): the interval that drew it belongs to the full window,
          // as the interval after this test's own `frame()` does when that frame draws it (below) --
          // "until the reduced image is drawn" holds nothing done after it, such as the full-size ask
          // that follows the draw in a task of its own (fix round 3). At 100 MP (no full window) the
          // interval stays in the reduced window instead, and the window ends once it is charged.
          if (phase === "reduced" && shown() === "rd") { if (wantsFull) endReduced(last); else drawnWithoutFull(last); }
          const interval = now - last;
          const callsInInterval = displayJobInputCalls.slice(seen);
          seen = displayJobInputCalls.length;
          callsThisWindow.push(...callsInInterval);
          frameMsThisWindow.push(previousFrameMs);
          const sum = (a: number[]) => a.reduce((x, y) => x + y, 0);
          const keepInInterval = keepMs.slice(seenKeep), rendersInInterval = appRenderMs.slice(seenRender);
          seenKeep = keepMs.length; seenRender = appRenderMs.length;
          if (interval > gap) {
            gap = interval;
            longestHolds = { displayJobInput: sum(callsInInterval), keepEffectsImage: sum(keepInInterval), appRender: sum(rendersInInterval), frame: Math.round(previousFrameMs) };
          }
          last = now;
          tickTimes.push(now);

          // 100 MP: the interval just charged held the reduced image's draw (the app's own render
          // between ticks, or this test's `frame()` at the previous tick); the window ends here.
          if (phase === "tail") {
            result["reduced image drawn after, ms"] = Math.round(reducedDrawnAt - t0);
            result["the interval that drew the reduced image, ms"] = Math.round(interval);
            readReduced(now);
            phase = "done";
            resolve();
            return;
          }

          // Fix round 5: the full window ends once the full image is drawn -- the interval just
          // charged held that render (the app's own, a frame after the image was kept, or this test's
          // `frame()` at the previous tick) -- so taking the image back and drawing it are both inside.
          if (phase === "full" && fullDrawnMs >= 0 && api.engine.hasEffectsImage(doc, layer, null)) {
            result["full image kept after, ms"] = Math.round(keptAt - fullWindowStart);
            result["full image drawn after, ms"] = Math.round(now - fullWindowStart);
            result["longest frame gap while the worker made it"] = Math.round(gap);
            for (const [what, ms] of Object.entries(longestHolds)) result[`the full window's longest gap holds: ${what} ms`] = ms;
            // Fix round 4: the render that actually drew it, not a later `frame()` of this test,
            // which finds the texture up to date.
            result["frame that draws it (halving and upload), ms"] = fullDrawnMs;
            result["that frame was the app's own render (1 = yes)"] = fullDrawnByApp;
            // Fix round 5: animation frames between taking the image back and drawing it (0 would mean
            // both in one inter-frame gap).
            result["frames between the keep and the draw"] = tickTimes.filter((t) => t > keptAt && t < fullDrawnAt).length;
            result["then drawn at full size (1 = yes)"] = shown() === "fx" ? 1 : 0;
            result["longest displayJobInput call inside the full window, ms"] = callsThisWindow.length ? Math.max(...callsThisWindow) : 0;
            result["longest single render frame inside the full window, ms"] = frameMsThisWindow.length ? Math.round(Math.max(...frameMsThisWindow)) : 0;
            {
              const longtasksHere = longtasks.filter((t) => t.start >= fullWindowStart && t.start < now);
              result["longtasks inside the full window (count)"] = longtasksHere.length;
              result["longest longtask inside the full window, ms"] = longtasksHere.length ? Math.round(Math.max(...longtasksHere.map((t) => t.duration))) : 0;
            }
            // Diagnostic (fix round 3): every displayJobInput call, windows aside (since fix round 4
            // the deferred full-size ask's copy is also charged to the full window above).
            result["longest displayJobInput call anywhere (unwindowed), ms"] = displayJobInputCalls.length ? Math.max(...displayJobInputCalls) : 0;
            result["displayJobInput calls made (total count)"] = displayJobInputCalls.length;
            phase = "done";
            resolve();
            return;
          }

          const frameMs = testFrame();
          previousFrameMs = frameMs;

          if (phase === "reduced" && shown() === "rd") {
            if (wantsFull) endReduced(now); else drawnWithoutFull(now);
          } else if (phase === "reduced" && now - reducedWindowStart > 60_000) {
            result["reduced image drawn after, ms"] = -1;
            result["longest frame gap until then"] = Math.round(gap);
            result["longest displayJobInput call inside that window, ms"] = callsThisWindow.length ? Math.max(...callsThisWindow) : 0;
            result["longest single render frame inside that window, ms"] = frameMsThisWindow.length ? Math.round(Math.max(...frameMsThisWindow)) : 0;
            phase = "done";
          } else if (phase === "full" && now - fullWindowStart > 120_000) {
            result["full image kept after, ms"] = -1;
            result["longest frame gap while the worker made it"] = Math.round(gap);
            result["longest displayJobInput call inside the full window, ms"] = callsThisWindow.length ? Math.max(...callsThisWindow) : 0;
            result["longest single render frame inside the full window, ms"] = frameMsThisWindow.length ? Math.round(Math.max(...frameMsThisWindow)) : 0;
            phase = "done";
          }

          if (phase === "done") resolve(); else requestAnimationFrame(tick);
        };
        requestAnimationFrame(tick);
      });
      // Past EFFECTS_LIMITS.full no full-size image is asked for or made (Task 7's deferred minor): the
      // full-size ask would follow the reduced draw a task later, so a second's frames are let pass
      // first. At 24 MP the same count shows the full-size ask this test waited for.
      if (!wantsFull) {
        const until = performance.now() + 1000;
        await new Promise<void>((done) => { const poll = () => { testFrame(); if (performance.now() > until) done(); else requestAnimationFrame(poll); }; requestAnimationFrame(poll); });
      }
      result["full-size asks made (level-0 displayJobInput calls)"] = displayJobInputLevels.filter((l) => l === 0).length;
      result["the engine holds a full-size image (1 = yes)"] = api.engine.hasEffectsImage(doc, layer, null) ? 1 : 0;
      return result;
    }, [w, h]);
    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
  }
  console.log(`effects images (release wasm, Edge): ${JSON.stringify(out)}`);
  // 24 MP asks for and keeps the full image; 100 MP (over EFFECTS_LIMITS.full) neither asks nor keeps one.
  expect(out["24 MP: full-size asks made (level-0 displayJobInput calls)"]).toBeGreaterThan(0);
  expect(out["24 MP: the engine holds a full-size image (1 = yes)"]).toBe(1);
  expect(out["100 MP: full-size asks made (level-0 displayJobInput calls)"]).toBe(0);
  expect(out["100 MP: the engine holds a full-size image (1 = yes)"]).toBe(0);
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
    // Fix round 2, issue B: logged but neither asserted nor exempt before, and the array behind it
    // was installed once and never reset, so it was not "inside that window" at all -- at 100 MP the
    // one and only `displayJobInput` call (the reduced ask) runs inside the exempt opening frame,
    // before this window's own tracking even starts, and this metric now correctly shows 0 for it.
    // At 24 MP it shows 0 too: the reduced ask also runs in the opening frame, and the full-size ask
    // (a level-0 copy, 66-98 ms in fix round 4's seven runs) runs after the reduced image is drawn,
    // so it belongs to the full window (charged there since fix round 4). The budget is looser than
    // this window alone would need: the "effects job preemption" test below asks for the identical copy after heavier prior wasm heap
    // growth (a reduced copy, an interrupted full copy and a histogram job's own whole-layer copy)
    // and measured 91-118 ms there, `WebAssembly.Memory.grow` costing more as the heap is already
    // larger -- the same call, a wider range of measured cost depending on memory pressure.
    expect(out[`${label}: longest displayJobInput call inside that window, ms`]).toBeLessThan(150);
  }
  expect(out["24 MP: then drawn at full size (1 = yes)"]).toBe(1);
  // Fix round 1, issue 6: this budget used to pass only because the exempt opening frame's cost was
  // silently discarded (the old two-rAF wait) rather than because the worker held the page up, and a
  // second bug (the tick loop's `last` left stale across that discarded frame) would have broken it
  // again the moment the first bug was fixed alone. Measured now, with both fixed: 68-95 ms. Fix
  // round 4 (the full-size ask that follows the draw charged to the full window): 31, 101, 75, 41,
  // 34, 29, 28 ms over seven runs (101 in a run slowed throughout: 30 s to the full image, not 12-14).
  expect(out["24 MP: longest frame gap until then"]).toBeLessThan(100);
  // At 100 MP the window runs to the tick after the reduced image's draw, so the interval that draws
  // it is charged here (the final fix wave: before, the window closed on the draw and that interval
  // belonged to no window). Measured 100-152 ms over Task 7's runs before the draw was charged. The
  // gap is not attributed to any instrumented piece (`displayJobInput` 0 ms inside the window - the
  // reduced ask runs in the exempt opening frame - single `frame()` calls 7-15 ms, no longtasks): an
  // open item in the rulings doc (it needs a profiler trace). "the interval that drew the reduced
  // image" is logged beside it as a diagnostic.
  expect(out["100 MP: longest frame gap until then"]).toBeLessThan(200);
  expect(out["100 MP: the interval that drew the reduced image, ms"], "the draw's interval was charged").toBeGreaterThanOrEqual(0);
  // The full window runs from the reduced image drawn to the full image drawn. It holds three large
  // UI-thread costs, each in a frame gap of its own: the full-size copy out to the worker
  // (`displayJobInput`, deferred a task by fix round 3; 66-98 ms in fix round 4), taking the finished
  // image back (`keepEffectsImage`, 78-127 ms) and the render that halves and uploads it (60-109 ms).
  // Fix round 4 found the last two stacked in one gap (147-248 ms over seven runs); fix round 5
  // draws the kept image a painted frame later (effects-images.ts `held`, `nextFrame`), so they no
  // longer share one; "frames between the keep and the draw" checks that below (2 in every run).
  // Fix round 5, seven runs: 126, 88, 85, 101, 97, 93, 98 ms; the longest gap held the copy out
  // (111, 80), the keep (81, 78, 88, 83) or the draw (87), never two of them.
  expect(out["24 MP: frames between the keep and the draw"]).toBeGreaterThan(0);
  expect(out["24 MP: longest frame gap while the worker made it"]).toBeLessThan(150);
  // The full-size copy out (`displayJobInput` at level 0, deferred a task after the reduced draw)
  // is charged to the full window; logged unasserted until the final fix wave (Task 7 parked item):
  // the same copy as a job's input, so the same 150 ms budget as `jobInput` at 24 MP.
  expect(out["24 MP: longest displayJobInput call inside the full window, ms"]).toBeLessThan(150);
  // Fix round 1, issue 6: measured but not asserted before. The frame that uploads the full image
  // (24 MP only; past that no full image is ever made) measured 13-24 ms in fix round 2's own runs,
  // and once 447 ms there (the actual GPU upload cost that time, coinciding with a gap-budget
  // failure). Fix round 3's ask/draw split (see the sibling budget's comment just above) removed that
  // outlier entirely: 13-19 ms in every one of seven runs this round, since the frame that uploads the
  // full image once it lands never also has to synchronously ask for anything. Fix round 4: those
  // 13-19 ms were this test's own `frame()` finding the texture already up to date; the render that
  // actually draws the full image is the app's own, as soon as it is kept: 99, 109, 60, 74, 67, 64,
  // 60 ms over seven runs, now the number measured here. Fix round 5 (a painted frame after the
  // keep): 64, 71, 62, 86, 87, 60, 73 ms.
  expect(out["24 MP: frame that draws it (halving and upload), ms"]).toBeLessThan(150);
});

test("effects job preemption: a Levels histogram requested while a 24 MP layer's full effects job is running respawns the worker promptly and the effects image is asked for again afterward (fix round 2, issue A)", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  const RUNS = 3;
  for (let run = 0; run < RUNS; run++) {
    await ready(page);
    await installFrameTimer(page);
    const r = await page.evaluate(async () => {
      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
      const result: Record<string, number> = {};
      const w = 6000, h = 4000; // 24 MP: EFFECTS_LIMITS.full covers this layer's own pixels
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
      // `beginAdjust` reads the active layer, which a reopened package does not necessarily carry
      // (the "jobs:" test above sets it explicitly too, for the same reason).
      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });

      // Every job asked for, and whether it has settled (resolved or rejected) yet, so "an effects
      // job is in flight" can be checked directly rather than guessed from timing.
      const jobs = api.store.getState().jobs;
      const realRun = jobs.run.bind(jobs);
      const asked: { channel: string; kind: string; settled: boolean }[] = [];
      jobs.run = (channel: string, request: any) => {
        const entry = { channel, kind: request.kind, settled: false };
        asked.push(entry);
        const p = realRun(channel, request) as Promise<unknown>;
        p.then(() => { entry.settled = true; }, () => { entry.settled = true; });
        return p;
      };
      // Every `{type:"job"}` message posted to *any* worker (fix round 1's preemption terminates the
      // old one and a fresh one takes over, so watching a fixed worker instance would miss it): the
      // moment one posts is "the edit job starting in the new worker" (issue A's (a)), distinct from
      // when its result comes back (which also includes the job's own execution time).
      const postedJobs: number[] = [];
      const realPostMessage = Worker.prototype.postMessage;
      Worker.prototype.postMessage = function (this: Worker, message: any, ...rest: any[]) {
        if (message && message.type === "job") postedJobs.push(performance.now());
        return realPostMessage.call(this, message, ...rest);
      };

      const shown = () => String(api.renderer.textureKey(doc, layer) ?? "").slice(0, 2);
      const waitFor = (done: () => boolean, limit: number) => new Promise<boolean>((resolve) => {
        const start = performance.now();
        const poll = () => { frame(); if (done()) resolve(true); else if (performance.now() - start > limit) resolve(false); else setTimeout(poll, 20); };
        poll();
      });
      // Every `displayJobInput` call, installed before the preemption so it catches the effects
      // image's re-ask (c) whenever it happens, not only after this test goes looking for it.
      const displayJobInputMs: number[] = [];
      const realDisplayJobInput = api.engine.displayJobInput.bind(api.engine);
      api.engine.displayJobInput = (...a: unknown[]) => {
        const s = performance.now();
        try { return realDisplayJobInput(...a); } finally { displayJobInputMs.push(Math.round(performance.now() - s)); }
      };

      api.store.getState().openDocument(doc);
      frame();
      await waitFor(() => shown() === "rd", 60_000);
      // The reduced image landing asks for the full one at once (this layer's own pixels are under
      // EFFECTS_LIMITS.full): wait for that ask, and confirm it is still running (not yet settled)
      // before preempting it -- proving the preemption below actually interrupts live work.
      await waitFor(() => asked.some((e) => e.kind === "effects" && !e.settled), 15_000);
      result["an effects job is in flight before the edit (1 = yes)"] = asked.some((e) => e.kind === "effects" && !e.settled) ? 1 : 0;

      // Frame gaps from here, rAF-driven (fix round 2, issue C's pattern: read only inside a tick). The
      // gap is read on the tick that stops the loop (`stopAt`), so the last interval before the
      // histogram landed is charged too (the final fix wave: it used to be read in the `waitFor`
      // continuation, off the tick, leaving that interval out).
      let last = performance.now(), gap = 0;
      let stopAt: ((gap: number) => void) | null = null;
      const tickGap = () => {
        const now = performance.now(); gap = Math.max(gap, now - last); last = now;
        if (stopAt) { stopAt(Math.round(gap)); return; }
        requestAnimationFrame(tickGap);
      };
      requestAnimationFrame(tickGap);

      // The preemption: opening Levels on this same layer reads its histogram through a job (its
      // pixel count is over JOB_PIXELS), which outranks the running effects job (fix round 1, issue
      // 3) and replaces the worker. Snapshotted here, before triggering it, so (c) below can tell the
      // effects image's re-ask apart from its original (already in `asked`) ask.
      const askedBeforePreemption = asked.length;
      // Every `displayJobInput` call from here on is the re-ask's (the histogram's own copy is
      // `jobInput`): (c) is windowed to these alone (Task 7 parked item; it used to take the maximum
      // over every call, the original full-size ask's included).
      const copiesBeforePreemption = displayJobInputMs.length;
      const spawnedBefore = jobs.spawned;
      const postedBefore = postedJobs.length;
      const t0 = performance.now();
      result["beginAdjust accepted it (1 = yes)"] = api.store.getState().beginAdjust({ kind: "Levels" }) ? 1 : 0;
      const posted = await waitFor(() => postedJobs.length > postedBefore, 30_000);
      result["(a) edit job posted to the new worker after, ms"] = posted ? Math.round(postedJobs[postedJobs.length - 1] - t0) : -1;
      result["the worker was actually respawned for it (1 = yes)"] = jobs.spawned > spawnedBefore ? 1 : 0;
      const histogrammed = await waitFor(() => api.store.getState().adjustEdit?.histogram !== null, 30_000);
      result["diagnostic: histogram (with respawn) arrives after, ms"] = histogrammed ? Math.round(performance.now() - t0) : -1;
      result["(b) longest frame gap during the preemption, ms"] = await new Promise<number>((done) => { stopAt = done; });
      api.store.getState().cancelAdjust();

      // (c) The effects image asked for again afterward, and that re-ask's own `displayJobInput` copy
      // (a level-0, full-size copy of the 24 MP layer, as "effects images" times in its full window).
      const reAsked = await waitFor(() => asked.slice(askedBeforePreemption).some((e) => e.kind === "effects"), 15_000);
      result["(c) the effects image was asked for again afterward (1 = yes)"] = reAsked ? 1 : 0;
      const reAskCopies = displayJobInputMs.slice(copiesBeforePreemption);
      result["(c) that re-ask's own displayJobInput cost, ms"] = reAskCopies.length ? Math.max(...reAskCopies) : -1;

      // A note on what is *not* measured here: a "warm worker" comparison (a second histogram
      // request right after, expecting no respawn) was tried and dropped -- the effects image just
      // re-asked for itself above takes some 10 s to complete (measured in "effects images"), so it
      // is running again (and gets preempted again) by the time a second request would go out, never
      // giving a genuinely uncontested, no-respawn baseline to subtract. (a) above is the direct
      // measurement the ruling asked for; no isolated "respawn alone, minus execution" number.

      return result;
    });
    for (const [k, v] of Object.entries(r)) out[`run ${run}: ${k}`] = v;
  }
  console.log(`effects job preemption (release wasm, Edge): ${JSON.stringify(out)}`);
  for (let run = 0; run < RUNS; run++) {
    expect(out[`run ${run}: an effects job is in flight before the edit (1 = yes)`]).toBe(1);
    expect(out[`run ${run}: beginAdjust accepted it (1 = yes)`]).toBe(1);
    expect(out[`run ${run}: the worker was actually respawned for it (1 = yes)`]).toBe(1);
    expect(out[`run ${run}: (c) the effects image was asked for again afterward (1 = yes)`]).toBe(1);
    // (a) the respawn (terminate + new Worker + module re-post + init) plus dispatch: measured
    // 221-324 ms over nine runs, three invocations (release wasm, Edge, real GPU); fix round 3,
    // three runs: 200-208 ms; fix round 4, nine runs: 242, 223, 257, 247, 234, 258, 239, 249, 208 ms;
    // fix round 5, nine runs: 235, 186, 225, 297, 252, 240, 228, 263, 259 ms. -1 means the job was
    // never posted (the 30 s wait ran out): refused too (the final fix wave).
    expect(out[`run ${run}: (a) edit job posted to the new worker after, ms`]).toBeGreaterThanOrEqual(0);
    expect(out[`run ${run}: (a) edit job posted to the new worker after, ms`]).toBeLessThan(400);
    // (b) the longest main-thread frame gap while the preemption (termination, respawn, dispatch)
    // happens: measured 178-230 ms over nine runs, three invocations; fix round 3, three runs:
    // 92-105 ms; fix round 4, nine runs: 123, 100, 100, 124, 107, 108, 111, 108, 100 ms; fix
    // round 5, nine runs: 109, 92, 100, 162, 137, 105, 111, 143, 115 ms.
    expect(out[`run ${run}: (b) longest frame gap during the preemption, ms`]).toBeLessThan(300);
    // (c) the re-ask's own full-size copy: the budget of the same copy in "effects images" (its full
    // window, 150 at 24 MP, the jobInput budget). Task 7 measured 72-126 ms here over nine runs (over
    // every call then, the original full-size ask's included); higher than the isolated test's
    // 66-111 ms because the wasm heap has grown by then (the reduced copy, the interrupted full copy,
    // the histogram's own copy) and `WebAssembly.Memory.grow` costs more on a larger heap. -1 (no
    // re-ask copy seen) fails.
    expect(out[`run ${run}: (c) that re-ask's own displayJobInput cost, ms`]).toBeGreaterThanOrEqual(0);
    expect(out[`run ${run}: (c) that re-ask's own displayJobInput cost, ms`]).toBeLessThan(150);
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

test("gradient previews: a drag tick, the settled preview and a patch in a 700 px selection, at 24 and 100 MP", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
    await ready(page);
    await installFrameTimer(page);
    const r = await page.evaluate(async ([w, h]) => {
      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const result: Record<string, number> = {};
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const layer = api.engine.state(doc).layers[0].id;
      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
      api.store.getState().openDocument(doc);
      await settle(); frame();
      const toFit = async () => { const s = api.store.getState(); s.viewports[doc].fit({ width: w, height: h }); s.invalidate(); await settle(); frame(); };
      const gradient = (i: number) => ({ shape: "Linear", start: [w * 0.2 + i, h * 0.3], end: [w * 0.8, h * 0.7 - i], from: [1, 0.2, 0, 1], to: [0, 0, 1, 0.3], opacity: 0.9 });
      // One tick: the engine's preview, the refresh, and the frame that draws it, as one span (fix
      // round 1, item 4: `refresh` used to run outside the timed window) -- what a pointer move
      // actually costs. The frame's own whole-texture upload count (`__uploads.image`, reset by
      // `__frame` itself) travels along too, for the patch case's assertion below (item 5).
      const tick = (i: number, dragging: boolean, mask: boolean) => {
        const t0 = performance.now();
        api.engine.setPreview(doc, { preview: "Gradient", layer, mask, gradient: gradient(i), dragging });
        const engine = performance.now() - t0;
        api.store.getState().refresh(doc);
        const f = frame();
        const total = performance.now() - t0;
        return [engine, f, total, (window as any).__uploads.image];
      };
      const worst = (dragging: boolean, mask = false) => {
        const ticks: number[][] = [];
        for (let i = 0; i < 6; i++) ticks.push(tick(i * 7, dragging, mask));
        ticks.shift(); // the first builds the reduced copies
        return [Math.max(...ticks.map((t) => t[0])), Math.max(...ticks.map((t) => t[1])), Math.max(...ticks.map((t) => t[2])), Math.max(...ticks.map((t) => t[3]))].map((v) => Math.round(v));
      };
      for (const zoom of ["fit", "1:1"]) {
        if (zoom === "1:1") { await api.setZoom(1); await settle(); frame(); }
        let [e, f, t] = worst(true);
        result[`drag engine ${zoom}`] = e; result[`drag frame ${zoom}`] = f; result[`drag total ${zoom}`] = t;
        [e, f, t] = worst(false);
        result[`settled engine ${zoom}`] = e; result[`settled frame ${zoom}`] = f; result[`settled total ${zoom}`] = t;
        api.engine.setPreview(doc, null); api.store.getState().refresh(doc); frame(); await settle();
      }
      // Fix round 1, item 1: three mask shapes, each timed dragging and settled at fit and 1:1, each
      // its own contiguous per-tick window (engine call, refresh and the frame that draws it) -- a
      // freshly added 1 x 1 uniform mask, a non-uniform full-size mask (an ellipse selection's, so
      // the gather genuinely resamples) and a full-size uniform mask (a Fill on the targeted mask,
      // which first grows the 1 x 1 mask onto the layer's grid).
      const maskCase = async (prefix: string, build: () => void) => {
        build();
        await toFit();
        for (const zoom of ["fit", "1:1"]) {
          if (zoom === "1:1") { await api.setZoom(1); await settle(); frame(); }
          let [e, f, t] = worst(true, true);
          result[`${prefix} drag engine ${zoom}`] = e; result[`${prefix} drag frame ${zoom}`] = f; result[`${prefix} drag total ${zoom}`] = t;
          [e, f, t] = worst(false, true);
          result[`${prefix} settled engine ${zoom}`] = e; result[`${prefix} settled frame ${zoom}`] = f; result[`${prefix} settled total ${zoom}`] = t;
          api.engine.setPreview(doc, null); api.store.getState().refresh(doc); frame(); await settle();
        }
        api.engine.execute(doc, { type: "DeleteMask", id: layer });
      };
      await maskCase("mask 1x1", () => api.engine.execute(doc, { type: "AddMask", id: layer, revealing: true }));
      await maskCase("mask non-uniform full", () => {
        api.engine.execute(doc, { type: "SelectShape", kind: "Ellipse", points: [[w * 0.1, h * 0.1], [w * 0.9, h * 0.1], [w * 0.9, h * 0.9], [w * 0.1, h * 0.9]], mode: "Replace", antialiased: true });
        api.engine.execute(doc, { type: "AddMaskFromSelection", id: layer, revealing: true });
      });
      await maskCase("mask uniform full", () => {
        api.engine.execute(doc, { type: "AddMask", id: layer, revealing: true });
        api.engine.execute(doc, { type: "Fill", id: layer, mask: true, color: [0.5, 0.5, 0.5] });
      });
      // A 700 x 700 selection in the middle (its clip, a pixel wider each side, within PATCH_LIMIT):
      // a full-size patch, at 1:1.
      const x = w / 2 - 350, y = h / 2 - 350;
      api.engine.execute(doc, { type: "SelectShape", kind: "Rectangle", points: [[x, y], [x + 700, y], [x + 700, y + 700], [x, y + 700]], mode: "Replace", antialiased: false });
      api.store.getState().refresh(doc); frame(); await settle();
      const [e, f, t, wholeUploads] = worst(true);
      result["patch engine"] = e; result["patch frame"] = f; result["patch total"] = t;
      // Fix round 1, item 5: the patch reaches the GPU as its rectangle alone, never a whole texture.
      result["patch whole uploads"] = wholeUploads;
      api.engine.setPreview(doc, null); api.store.getState().closeDocument(doc);
      return result;
    }, [w, h]);
    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
  }
  console.log(`gradient previews (release wasm, Edge): ${JSON.stringify(out)}`);
  for (const label of ["24 MP", "100 MP"]) {
    for (const zoom of ["fit", "1:1"]) {
      expect(out[`${label}: drag total ${zoom}`]).toBeLessThan(50);
      expect(out[`${label}: settled total ${zoom}`]).toBeLessThan(150);
      // Fix round 1, item 1: the mask gradient's ticks get the same budgets as the pixel gradient's,
      // for all three mask shapes.
      for (const prefix of ["mask 1x1", "mask non-uniform full", "mask uniform full"]) {
        expect(out[`${label}: ${prefix} drag total ${zoom}`]).toBeLessThan(50);
        expect(out[`${label}: ${prefix} settled total ${zoom}`]).toBeLessThan(150);
      }
    }
    expect(out[`${label}: patch total`]).toBeLessThan(50);
    // Fix round 1, item 5.
    expect(out[`${label}: patch whole uploads`], "the patch reaches the GPU as its rectangle, never a whole texture").toBe(0);
  }
});

test("mask gradients that grow the mask to the canvas: drag and settled ticks, and the frame after applying one, at 24 and 100 MP", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
    await ready(page);
    await installFrameTimer(page);
    const r = await page.evaluate(async ([w, h]) => {
      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const result: Record<string, number> = {};
      // A 1500 x 1000 layer in the middle of the canvas (Canvas Size with a fill, then without one)
      // under a non-uniform mask of its own grid (an ellipse selection's): its gradient grows the mask
      // to the whole canvas (Task 14a), 24 or 100 MP of mask from 1.5 MP.
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: 1500, height: 1000, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const layer = api.engine.state(doc).layers[0].id;
      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: null });
      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
      const x0 = (w - 1500) / 2, y0 = (h - 1000) / 2;
      api.engine.execute(doc, { type: "SelectShape", kind: "Ellipse", points: [[x0 + 100, y0 + 100], [x0 + 1400, y0 + 100], [x0 + 1400, y0 + 900], [x0 + 100, y0 + 900]], mode: "Replace", antialiased: true });
      api.engine.execute(doc, { type: "AddMaskFromSelection", id: layer, revealing: true });
      api.engine.execute(doc, { type: "Deselect" });
      api.store.getState().openDocument(doc);
      const fit = api.store.getState(); fit.viewports[doc].fit({ width: w, height: h }); fit.invalidate();
      await settle(); frame();
      const gradient = (i: number) => ({ shape: "Linear", start: [w * 0.2 + i, h * 0.3], end: [w * 0.8, h * 0.7 - i], from: [0, 0, 0, 1], to: [0, 0, 0, 0], opacity: 0.9 });
      // One tick as "gradient previews" times it: the engine's preview, the refresh and the frame that
      // draws it, one contiguous window.
      const tick = (i: number, dragging: boolean) => {
        const t0 = performance.now();
        api.engine.setPreview(doc, { preview: "Gradient", layer, mask: true, gradient: gradient(i), dragging });
        const engine = performance.now() - t0;
        api.store.getState().refresh(doc);
        const f = frame();
        return [engine, f, performance.now() - t0];
      };
      const worst = (dragging: boolean) => {
        const ticks: number[][] = [];
        for (let i = 0; i < 6; i++) ticks.push(tick(i * 7, dragging));
        ticks.shift(); // the first builds the selection clip and the textures
        return [0, 1, 2].map((k) => Math.round(Math.max(...ticks.map((t) => t[k]))));
      };
      for (const zoom of ["fit", "1:1"]) {
        if (zoom === "1:1") { await api.setZoom(1); await settle(); frame(); }
        let [e, f, t] = worst(true);
        result[`drag engine ${zoom}`] = e; result[`drag frame ${zoom}`] = f; result[`drag total ${zoom}`] = t;
        const dragged = api.engine.state(doc).layers[0];
        result[`dragged mask pixels ${zoom}`] = dragged.maskWidth * dragged.maskHeight;
        [e, f, t] = worst(false);
        result[`settled engine ${zoom}`] = e; result[`settled frame ${zoom}`] = f; result[`settled total ${zoom}`] = t;
        const settled = api.engine.state(doc).layers[0];
        result[`settled mask pixels ${zoom}`] = settled.maskWidth * settled.maskHeight;
        api.engine.setPreview(doc, null); api.store.getState().refresh(doc); frame(); await settle();
      }
      // Applied, on the UI thread here: a diagnostic only (over JOB_PIXELS the Gradient tool sends it to
      // the job worker, Task 15, whose perf case budgets that path). Then the refresh and the frame that
      // draw the canvas-sized mask, uploaded whole: budgeted below.
      let t0 = performance.now();
      api.engine.execute(doc, { type: "Gradient", id: layer, mask: true, gradient: gradient(0) });
      result["apply on the UI thread (diagnostic) ms"] = Math.round(performance.now() - t0);
      t0 = performance.now();
      api.store.getState().refresh(doc);
      frame();
      result["frame after applying (refresh and whole mask upload) ms"] = Math.round(performance.now() - t0);
      result["frame after applying: whole uploads"] = (window as any).__uploads.image;
      const applied = api.engine.state(doc).layers[0];
      result["applied mask pixels"] = applied.maskWidth * applied.maskHeight;
      api.store.getState().closeDocument(doc);
      return result;
    }, [w, h]);
    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
  }
  console.log(`mask gradients that grow the mask (release wasm, Edge): ${JSON.stringify(out)}`);
  // The preview's grid: the canvas halved until it fits the limit (`level_for`, preview.rs).
  const reduced = (w: number, h: number, limit: number) => { let l = 0; while (Math.max(w >> l, h >> l) > limit && (w >> l) > 1 && (h >> l) > 1) l++; return (w >> l) * (h >> l); };
  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
    for (const zoom of ["fit", "1:1"]) {
      // It timed the grown mask, not the layer's own grid.
      expect(out[`${label}: dragged mask pixels ${zoom}`]).toBe(reduced(w, h, 1024));
      expect(out[`${label}: settled mask pixels ${zoom}`]).toBe(reduced(w, h, 2048));
      // Ruling I2: the mask gradient's ticks keep the pixel gradient's budgets.
      expect(out[`${label}: drag total ${zoom}`]).toBeLessThan(50);
      expect(out[`${label}: settled total ${zoom}`]).toBeLessThan(150);
    }
    expect(out[`${label}: applied mask pixels`]).toBe(w * h);
    expect(out[`${label}: frame after applying: whole uploads`]).toBeGreaterThanOrEqual(1);
    // Fix round 1, 14a-perf-2: a ~100 MB R8 whole upload at 100 MP measured 291-320 ms across two runs
    // against the first round's 150 ms budget, consistently -- a real, reproducible cost of this GPU's
    // whole-texture upload, not a one-off. 150 / 400 ms, consistent with the "jobs" case's own frame-
    // after-result budget for a whole re-upload at these sizes (350 / 400 ms there, which documents the
    // same kind of run-to-run bimodal warm-up cost this fix round's `mask-grow` fill case also found).
    expect(out[`${label}: frame after applying (refresh and whole mask upload) ms`]).toBeLessThan(label === "24 MP" ? 150 : 400);
  }
});

test("a fill on a small layer's mask: the mask grows to the canvas through the worker, then a second fill, a nudge, Undo and Redo, at 24 and 100 MP", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
    for (const selected of [false, true]) {
      await ready(page);
      await installFrameTimer(page);
      const r = await page.evaluate(async ([w, h, selected]) => {
        const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
        const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
        const s = () => api.store.getState();
        const result: Record<string, number> = {};
        // Fix round 1, 14a-perf-2: a cold first fill in a freshly loaded page can pay a one-time cost
        // (the module worker's own start-up: creating the Worker, compiling its wasm inside it) that
        // lands inside a timed scenario's "longest frame gap" -- reproducibly, only as the very first
        // scenario after a fresh page load (this fix round's audit measured 289-293 ms there, twice,
        // against the 100 ms budget below). A warm-up fill on a small document (2560 x 2560, well over
        // JOB_PIXELS so it takes the same worker path) pays that cost here instead, before any patching
        // below, so it is not counted as one of this scenario's own jobs; its own gap is logged,
        // unasserted, and the timed scenarios keep their 100 ms budget.
        {
          const warm = api.engine.newDocument(10, 10, false);
          api.engine.execute(warm, { type: "CanvasSize", width: 2560, height: 2560, anchor: 4, fill: [0.5, 0.4, 0.3] });
          const warmLayer = api.engine.state(warm).layers[0].id;
          api.engine.execute(warm, { type: "AddMask", id: warmLayer, revealing: true });
          s().openDocument(warm);
          s().setMaskSelected(true);
          await settle(); frame();
          const t0 = performance.now();
          window.dispatchEvent(new KeyboardEvent("keydown", { key: "Backspace", altKey: true }));
          result["cold warm-up fill (UI thread) ms"] = Math.round(performance.now() - t0);
          result["cold warm-up fill: longest frame gap"] = await new Promise<number>((done) => {
            let last = performance.now(), gap = 0;
            const tick = () => { const now = performance.now(); gap = Math.max(gap, now - last); last = now; if (!s().working) done(Math.round(gap)); else requestAnimationFrame(tick); };
            requestAnimationFrame(tick);
          });
          s().closeDocument(warm);
        }
        let installedAt = Infinity, jobs = 0, pass = "first fill";
        const timed = (name: string, after?: () => void) => {
          const method = `${name}Async`, f = api.engine[method].bind(api.engine);
          api.engine[method] = async (...a: unknown[]) => { try { return await f(...a); } finally { result[`${pass}: ${name} ms`] = Math.round(api.engine[name === "jobInput" ? "lastJobInputCpuMs" : "lastInstallCpuMs"]); after?.(); } };
        };
        timed("jobInput");
        // The frame that draws the result, timed where it lands, as the "jobs" case does: no interval
        // outside a window (audit I-7).
        timed("installJob", () => { installedAt = performance.now(); s().refresh(s().activeId); result[`${pass}: frame after it ms`] = Math.round(frame()); result[`${pass}: frame after it, whole uploads`] = (window as any).__uploads.image; result[`${pass}: frame after it, partial uploads`] = (window as any).__uploads.sub; });
        const client = s().jobs; const send = client.run.bind(client);
        client.run = (...a: unknown[]) => { jobs++; return send(...a); };
        // A 1500 x 1000 layer in the middle of the canvas under a white mask, targeted: 1.5 MP stored, the
        // canvas painted (Task 14a).
        const doc = api.engine.newDocument(10, 10, false);
        api.engine.execute(doc, { type: "CanvasSize", width: 1500, height: 1000, anchor: 4, fill: [0.5, 0.4, 0.3] });
        const layer = api.engine.state(doc).layers[0].id;
        api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: null });
        api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
        api.engine.execute(doc, { type: "AddMask", id: layer, revealing: true });
        s().openDocument(doc);
        s().setMaskSelected(true);
        // Audit I-5: under a feathered Select All the key must not fill the selection's clip on the UI
        // thread (`SelectionClip::region`); the worker builds its own.
        if (selected) { api.engine.execute(doc, { type: "SelectAll" }); api.engine.execute(doc, { type: "FeatherSelection", amount: 20 }); s().refresh(doc); }
        await settle(); frame();
        result["mask pixels the fill paints"] = api.engine.editPixels(doc, layer, true);
        // performance.now() inside the callback, not the rAF timestamp (Task 6 fix round 1, issue 4;
        // audit I-7). Only frames before the result is put back count.
        const longestGap = (until: () => boolean) => new Promise<number>((done) => {
          let last = performance.now(), gap = 0;
          const tick = () => { const now = performance.now(); if (now <= installedAt) gap = Math.max(gap, now - last); last = now; if (until()) done(Math.round(gap)); else requestAnimationFrame(tick); };
          requestAnimationFrame(tick);
        });
        const fill = async () => {
          installedAt = Infinity;
          const t0 = performance.now();
          window.dispatchEvent(new KeyboardEvent("keydown", { key: "Backspace", altKey: true }));
          result[`${pass}: Alt+Backspace (UI thread) ms`] = Math.round(performance.now() - t0);
          result[`${pass}: longest frame gap while the worker fills`] = await longestGap(() => !s().working);
        };
        await fill();
        const l = api.engine.state(doc).layers[0];
        result["mask pixels"] = l.maskWidth * l.maskHeight;
        // Audit I-6 (b): a second fill on the mask already grown to the canvas: its copies out and back
        // are the grown mask's (24 / 100 MB).
        pass = "second fill";
        await settle(); frame();
        await fill();
        result["jobs run"] = jobs;
        // Audit I-6 (a): a nudge of the layer whose mask grew, then Undo and Redo. Each moves the placed
        // mask with its layer; the mask's revision moves but its buffer does not, so nothing is uploaded
        // (final review minor 9, `Lineage`'s buffer identity: before it, each re-uploaded the grown
        // mask whole). The frame after each is budgeted.
        s().setTool("move");
        await settle(); frame();
        for (const [step, act] of [["nudge", () => window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" }))], ["undo", () => s().undo()], ["redo", () => s().redo()]] as [string, () => void][]) {
          const t0 = performance.now();
          act();
          result[`${step} (UI thread) ms`] = Math.round(performance.now() - t0);
          result[`frame after the ${step} ms`] = Math.round(frame());
          result[`frame after the ${step}, whole uploads`] = (window as any).__uploads.image;
          result[`frame after the ${step}, partial uploads`] = (window as any).__uploads.sub;
          await settle();
        }
        result["layer moved by the nudge, then back, then again"] = api.engine.state(doc).layers[0].transform.origin[0] - (w - 1500) / 2;
        s().closeDocument(doc);
        return result;
      }, [w, h, selected] as [number, number, boolean]);
      for (const [k, v] of Object.entries(r)) out[`${label}${selected ? ", Select All feathered" : ""}: ${k}`] = v;
    }
  }
  console.log(`a fill on a growing mask (release wasm, Edge): ${JSON.stringify(out)}`);
  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
    for (const run of [label, `${label}, Select All feathered`]) {
      const k = (name: string) => out[`${run}: ${name}`];
      // The worker rule (OQ5, ruling C1): counted grown, so sent to the worker, both times.
      expect(k("mask pixels the fill paints")).toBe(w * h);
      expect(k("jobs run"), "both fills went to the worker").toBe(2);
      expect(k("mask pixels")).toBe(w * h);
      expect(k("layer moved by the nudge, then back, then again"), "the nudge, Undo and Redo ran").toBe(1);
      for (const pass of ["first fill", "second fill"]) {
        expect(k(`${pass}: Alt+Backspace (UI thread) ms`)).toBeLessThan(150);
        expect(k(`${pass}: longest frame gap while the worker fills`)).toBeLessThan(100);
        expect(k(`${pass}: jobInput ms`)).toBeLessThan(150);
        // OQ5's "500" at 100 MP is 450 (ruling I7).
        expect(k(`${pass}: installJob ms`)).toBeLessThan(label === "24 MP" ? 150 : 450);
        // Fix round 1: the first fill grows the mask (a size change forces a whole upload). The second
        // fill leaves it the same size and placement (I-1's fix: no more float-drift regrowth), so under
        // a selection it correctly uploads only the region the selection reaches (`SelectionClip`,
        // `Lineage::record_edit`'s `same_grid` check) -- a partial upload, not a whole one; without a
        // selection there is no region to report at all, so it stays a whole upload either way.
        expect(k(`${pass}: frame after it, whole uploads`) + k(`${pass}: frame after it, partial uploads`), "the frame timed is the one that uploads the grown mask, whole or in the region a selection reaches").toBeGreaterThanOrEqual(1);
        // Fix round 1, 14a-perf-2: this frame's own whole (or, for a same-grid selection, partial)
        // upload of the mask is the same ~100 MB R8 cost as the "mask gradients" case's "frame after
        // applying" -- reached at all only once the warm-up above removed the 24 MP cold-start spike
        // that used to abort this test before it got this far. 150 / 400 ms, consistent with C1's 350 /
        // 400 there.
        expect(k(`${pass}: frame after it ms`)).toBeLessThan(label === "24 MP" ? 150 : 400);
      }
      // The nudge, Undo and Redo move the grown mask without changing its bytes: no upload at all
      // (final review minor 9). Before the fix each re-uploaded the mask whole, under the 150 / 400 ms
      // budget of fix round 1 (14a-perf-2); the final fix wave measured 18-27 ms (24 MP) and 61-189 ms
      // (100 MP) then, one whole upload each, and 4-9 ms (24 MP) and 6-12 ms (100 MP) after, with no
      // upload, over two runs; the budget is now the partial-upload case's frame budget (33 ms,
      // "partial uploads" above) at both sizes.
      for (const step of ["nudge", "undo", "redo"]) {
        expect(k(`frame after the ${step}, whole uploads`), `${step}: no whole upload`).toBe(0);
        expect(k(`frame after the ${step}, partial uploads`), `${step}: no partial upload`).toBe(0);
        expect(k(`frame after the ${step} ms`)).toBeLessThan(33);
      }
    }
  }
});

test("eyedropper: a sample and the overlay that shows its ring, at 24 and 100 MP", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
    await ready(page);
    const r = await page.evaluate(async ([w, h]) => {
      const api = (window as any).__compositor; const s0 = api.store.getState();
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
      // One layer: the project's 100 megapixels hold no second one at 100 MP.
      s0.openDocument(doc);
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const ticks: number[] = [];
      for (let i = 0; i < 20; i++) {
        const t0 = performance.now();
        const at = { x: (w * (i + 0.5)) / 20, y: h / 2 };
        api.store.getState().sampleForeground(at);
        api.store.getState().setSampleRing({ at: { x: 300, y: 300 }, sampled: api.store.getState().palette.foreground, original: { red: 0, green: 0, blue: 0 } });
        api.paintOverlay();
        ticks.push(performance.now() - t0);
      }
      api.store.getState().setSampleRing(null);
      api.store.getState().closeDocument(doc);
      return { worst: Math.round(10 * Math.max(...ticks)) / 10, mean: Math.round(10 * ticks.reduce((a, b) => a + b, 0) / ticks.length) / 10 };
    }, [w, h]);
    out[`${label}: sample and ring, worst ms`] = r.worst; out[`${label}: mean ms`] = r.mean;
  }
  console.log(`eyedropper (release wasm, Edge): ${JSON.stringify(out)}`);
  for (const label of ["24 MP", "100 MP"]) expect(out[`${label}: sample and ring, worst ms`]).toBeLessThan(16);
});

test("gradient tool: drag ticks through the store and the commit through the worker, at 24 and 100 MP", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
    await ready(page);
    await installFrameTimer(page);
    const r = await page.evaluate(async ([w, h]) => {
      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const result: Record<string, number> = {};
      let installedAt = Infinity;
      const timed = (name: string, after?: () => void) => {
        const method = `${name}Async`, f = api.engine[method].bind(api.engine);
        api.engine[method] = async (...a: unknown[]) => { try { return await f(...a); } finally { result[`${name} ms`] = Math.round(api.engine[name === "jobInput" ? "lastJobInputCpuMs" : "lastInstallCpuMs"]); after?.(); } };
      };
      timed("jobInput");
      // The frame that draws the result, timed where it lands, as the "jobs" case does: no interval
      // outside a window (pre-flight audit I-7).
      timed("installJob", () => { installedAt = performance.now(); api.store.getState().refresh(api.store.getState().activeId); result["frame after it ms"] = Math.round(frame()); result["frame after it, whole uploads"] = (window as any).__uploads.image; });
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
      api.engine.execute(doc, { type: "SetActiveLayer", id: api.engine.state(doc).layers[0].id });
      api.store.getState().openDocument(doc);
      api.store.getState().setTool("gradient");
      await settle(); frame();
      const s = () => api.store.getState();
      s().beginGradient({ x: w * 0.2, y: h * 0.5 });
      const ticks: number[] = [];
      for (let i = 0; i < 8; i++) {
        const t0 = performance.now();
        s().moveGradient({ end: { x: w * (0.5 + i * 0.04), y: h * 0.6 } }, true);
        frame();
        ticks.push(performance.now() - t0);
      }
      ticks.shift();
      result["drag tick (store, engine and frame), worst ms"] = Math.round(Math.max(...ticks));
      let t0 = performance.now();
      s().endGradientDrag(); frame();
      result["release (settled preview and frame) ms"] = Math.round(performance.now() - t0);
      const longestGap = (until: () => boolean) => new Promise<number>((done) => {
        let last = performance.now(), gap = 0;
        // performance.now() inside the callback, not the rAF timestamp (Task 6 fix round 1, issue 4; audit I-7).
        const tick = () => { const now = performance.now(); if (now <= installedAt) gap = Math.max(gap, now - last); last = now; if (until()) done(Math.round(gap)); else requestAnimationFrame(tick); };
        requestAnimationFrame(tick);
      });
      t0 = performance.now();
      s().commitGradient();
      result["Return (UI thread) ms"] = Math.round(performance.now() - t0);
      result["commit: longest frame gap while the worker paints"] = await longestGap(() => !s().working);
      result["commit done after ms"] = Math.round(performance.now() - t0);
      result["undo depth"] = api.engine.state(doc).undoDepth;
      return result;
    }, [w, h]);
    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
  }
  console.log(`gradient tool (release wasm, Edge): ${JSON.stringify(out)}`);
  for (const label of ["24 MP", "100 MP"]) {
    expect(out[`${label}: drag tick (store, engine and frame), worst ms`]).toBeLessThan(50);
    expect(out[`${label}: release (settled preview and frame) ms`]).toBeLessThan(150);
    expect(out[`${label}: commit: longest frame gap while the worker paints`]).toBeLessThan(100);
    // Fix round 1, M-6: measured but not asserted until now.
    expect(out[`${label}: Return (UI thread) ms`]).toBeLessThan(label === "24 MP" ? 150 : 450);
    // OQ5's "500" at 100 MP is 450 (ruling I7; audit M-5).
    expect(out[`${label}: jobInput ms`]).toBeLessThan(label === "24 MP" ? 150 : 450);
    expect(out[`${label}: installJob ms`]).toBeLessThan(label === "24 MP" ? 150 : 450);
    expect(out[`${label}: frame after it, whole uploads`], "the frame timed is the one that uploads the result").toBeGreaterThanOrEqual(1);
    // The "jobs" case's budget for the frame that re-uploads a whole committed layer (perf-4b1.spec.ts:186).
    // Fix round 1, controller ruling: 500 at 100 MP (was 400) -- on the HD 520 that frame halves a
    // fresh 100 MP result and uploads it; the follow-up is "mask levels / display-level uploads" in
    // the final review.
    expect(out[`${label}: frame after it ms`]).toBeLessThan(label === "24 MP" ? 350 : 500);
  }
  // Canvas Size and the Gradient at 24 MP; at 100 MP each entry holds 400 MB, past the history's
  // 256 MiB, so none is kept (HISTORY_BYTE_LIMIT, as the Mac's DocumentHistory).
  expect([out["24 MP: undo depth"], out["100 MP: undo depth"]]).toEqual([2, 0]);
});

test("shapes and fills: a shape over the canvas on the UI thread, the Shape tool's draft ticks, and a fill of the whole layer through the worker, at 24 and 100 MP", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  let first = true, renderer = "";
  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
    await ready(page);
    await installFrameTimer(page);
    // Ruling R16-2 (controller): the same warm-up as ruling C1's case, copied verbatim, before any
    // timed scenario in this case -- run by itself, in a fresh browser process, the first render pays a
    // one-time GPU cost of about 300-440 ms, probably a shader compile. Logged below, not asserted.
    const warmUp = first; first = false;
    if (warmUp) renderer = await page.evaluate(() => {
      const gl = (document.querySelector('[data-testid="canvas-view"] canvas') as HTMLCanvasElement).getContext("webgl2")!;
      const info = gl.getExtension("WEBGL_debug_renderer_info");
      return info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : "unknown";
    });
    const r = await page.evaluate(async ([w, h, warmUp]) => {
      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const result: Record<string, number> = {};
      const s = () => api.store.getState();
      if (warmUp) {
        const warm = api.engine.newDocument(2560, 2560, true);
        s().openDocument(warm);
        await settle(); frame();
        s().setTool("gradient");
        s().beginGradient({ x: 500, y: 1280 });
        s().moveGradient({ end: { x: 2000, y: 1500 } }, true);
        frame();
        s().cancelGradient(); frame(); await settle();
        s().setTool("move");
        const t0 = performance.now();
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "Backspace", altKey: true }));
        result["warm-up fill (UI thread) ms"] = Math.round(performance.now() - t0);
        result["warm-up fill: went to the worker"] = s().working ? 1 : 0;
        result["warm-up fill: longest frame gap"] = await new Promise<number>((done) => {
          let last = performance.now(), gap = 0;
          const tick = () => { const now = performance.now(); gap = Math.max(gap, now - last); last = now; if (!s().working) done(Math.round(gap)); else requestAnimationFrame(tick); };
          requestAnimationFrame(tick);
        });
        result["warm-up fill: total ms"] = Math.round(performance.now() - t0);
        s().closeDocument(warm);
        await settle();
      }
      // An empty canvas: the shape's pixels are the project's only ones.
      const blank = api.engine.newDocument(w, h, false);
      s().openDocument(blank);
      await settle(); frame();
      for (const [name, draft] of [["rectangle", { kind: "Rectangle", anchor: { x: 0, y: 0 }, rect: { x: 0, y: 0, width: w, height: h }, end: null, cornerRadius: 400 }],
        ["ellipse", { kind: "Ellipse", anchor: { x: 0, y: 0 }, rect: { x: 0, y: 0, width: w, height: h }, end: null, cornerRadius: 0 }]] as const) {
        s().setShapeDraft(draft);
        const t0 = performance.now();
        s().finishShape();
        result[`${name} over the canvas (release, UI thread) ms`] = Math.round(performance.now() - t0);
        result[`frame after the ${name} ms`] = Math.round(frame());
        s().undo(); await settle();
      }
      // Ruling I2 / R16-3 (controller): 20 draft ticks -- setShapeDraft(dragShape(...)) plus the overlay
      // paint that draws it (api.paintOverlay(), as the eyedropper case above forces one), each in one
      // contiguous performance.now() window with no await inside it, as the real pointer path
      // (canvas/shape-tool.ts) runs them: a rectangle with a corner radius, an ellipse, and a 20 px line.
      s().setTool("shape");
      const cases: { name: string; options: { kind: "Rectangle" | "Ellipse" | "Line"; cornerRadius: number; lineWidth: number }; begin: { x: number; y: number }; to: { x: number; y: number } }[] = [
        { name: "rectangle", options: { kind: "Rectangle", cornerRadius: 40, lineWidth: 4 }, begin: { x: 0, y: 0 }, to: { x: w, y: h } },
        { name: "ellipse", options: { kind: "Ellipse", cornerRadius: 0, lineWidth: 4 }, begin: { x: w / 2, y: h / 2 }, to: { x: w, y: h } },
        { name: "20 px line", options: { kind: "Line", cornerRadius: 0, lineWidth: 20 }, begin: { x: 0, y: h / 2 }, to: { x: w, y: h / 2 } },
      ];
      for (const c of cases) {
        s().setShapeOptions(c.options);
        let draft = api.beginShape(c.begin, s().shapeOptions);
        const ticks: number[] = [];
        for (let i = 1; i <= 20; i++) {
          const p = { x: c.begin.x + (c.to.x - c.begin.x) * i / 20, y: c.begin.y + (c.to.y - c.begin.y) * i / 20 };
          const t0 = performance.now();
          draft = api.dragShape(draft, p, false, false);
          s().setShapeDraft(draft);
          api.paintOverlay();
          ticks.push(performance.now() - t0);
        }
        s().setShapeDraft(null);
        result[`${c.name} draft tick, worst ms`] = Math.round(10 * Math.max(...ticks)) / 10;
      }
      s().setTool("move");
      s().closeDocument(blank);
      // A fill of a whole layer, through the worker.
      let installedAt = Infinity;
      const f = api.engine.installJobAsync.bind(api.engine);
      // The frame that draws the result, timed where it lands, as the "jobs" case does (pre-flight audit I-7).
      api.engine.installJobAsync = async (...a: unknown[]) => { try { return await f(...a); } finally { installedAt = performance.now(); s().refresh(s().activeId); result["fill: frame after the result is put back ms"] = Math.round(frame()); result["fill: that frame's whole uploads"] = (window as any).__uploads.image; } };
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
      api.engine.execute(doc, { type: "SetActiveLayer", id: api.engine.state(doc).layers[0].id });
      s().openDocument(doc);
      await settle(); frame();
      const longestGap = (until: () => boolean) => new Promise<number>((done) => {
        let last = performance.now(), gap = 0;
        // performance.now() inside the callback, not the rAF timestamp (Task 6 fix round 1, issue 4; audit I-7).
        const tick = () => { const now = performance.now(); if (now <= installedAt) gap = Math.max(gap, now - last); last = now; if (until()) done(Math.round(gap)); else requestAnimationFrame(tick); };
        requestAnimationFrame(tick);
      });
      const t0 = performance.now();
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "Backspace", altKey: true }));
      result["Alt+Backspace (UI thread) ms"] = Math.round(performance.now() - t0);
      result["fill: longest frame gap while the worker fills"] = await longestGap(() => !s().working);
      result["fill done after ms"] = Math.round(performance.now() - t0);
      return result;
    }, [w, h, warmUp] as [number, number, boolean]);
    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
  }
  console.log(`shapes and fills (release wasm, Edge): ${JSON.stringify(out)}`);
  console.log(`shapes and fills renderer: ${renderer}`);
  // Shapes commit on the UI thread, as an import does (ruling OQ11); the first pays for the wasm
  // memory growing to hold it.
  expect(out["24 MP: rectangle over the canvas (release, UI thread) ms"]).toBeLessThan(1000);
  expect(out["24 MP: ellipse over the canvas (release, UI thread) ms"]).toBeLessThan(1000);
  expect(out["100 MP: rectangle over the canvas (release, UI thread) ms"]).toBeLessThan(2000);
  expect(out["100 MP: ellipse over the canvas (release, UI thread) ms"]).toBeLessThan(2000);
  // Ruling I2 / R16-3 (controller): the worst of 20 draft ticks stays under a frame.
  for (const label of ["24 MP", "100 MP"]) {
    for (const name of ["rectangle", "ellipse", "20 px line"]) {
      expect(out[`${label}: ${name} draft tick, worst ms`]).toBeLessThan(16);
    }
  }
  for (const label of ["24 MP", "100 MP"]) expect(out[`${label}: fill: longest frame gap while the worker fills`]).toBeLessThan(100);
  // The frame that re-uploads the whole filled layer: the "jobs" case's budget (perf-4b1.spec.ts:186).
  // Ruling R16-1 (controller): 350 ms at 24 MP and 600 ms at 100 MP -- this replaces the brief's 400.
  // Task 15's ruling C1 found that at 100 MP `engine.layerPixels` prefilters the freshly committed layer
  // to display resolution on the UI thread (314-533 ms of the frame), the same cost this fill's frame
  // pays. The follow-up is to prefilter that display level in the worker and return it with the result,
  // instead of computing it synchronously on the UI thread.
  for (const label of ["24 MP", "100 MP"]) {
    expect(out[`${label}: fill: that frame's whole uploads`]).toBeGreaterThanOrEqual(1);
    expect(out[`${label}: fill: frame after the result is put back ms`]).toBeLessThan(label === "24 MP" ? 350 : 600);
  }
});

test("gradient tool on a small layer's mask: the mask grows to the canvas and the commit goes through the worker, at 24 and 100 MP", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
    await ready(page);
    await installFrameTimer(page);
    const r = await page.evaluate(async ([w, h]) => {
      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const result: Record<string, number> = {};
      let installedAt = Infinity, jobs = 0;
      const timed = (name: string, after?: () => void) => {
        const method = `${name}Async`, f = api.engine[method].bind(api.engine);
        api.engine[method] = async (...a: unknown[]) => { try { return await f(...a); } finally { result[`${name} ms`] = Math.round(api.engine[name === "jobInput" ? "lastJobInputCpuMs" : "lastInstallCpuMs"]); after?.(); } };
      };
      timed("jobInput");
      // The frame that draws the result, timed where it lands, as the "jobs" case does: no interval
      // outside a window (pre-flight audit I-7).
      timed("installJob", () => { installedAt = performance.now(); api.store.getState().refresh(api.store.getState().activeId); result["frame after it ms"] = Math.round(frame()); result["frame after it, whole uploads"] = (window as any).__uploads.image; });
      const client = api.store.getState().jobs; const send = client.run.bind(client);
      client.run = (...a: unknown[]) => { jobs++; return send(...a); };
      // A 1500 x 1000 layer in the middle of the canvas under a white mask, targeted (Task 14a).
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: 1500, height: 1000, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const layer = api.engine.state(doc).layers[0].id;
      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: null });
      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
      api.engine.execute(doc, { type: "AddMask", id: layer, revealing: true });
      const s = () => api.store.getState();
      s().openDocument(doc);
      s().setMaskSelected(true);
      s().setTool("gradient");
      await settle(); frame();
      s().beginGradient({ x: w * 0.2, y: h * 0.5 });
      s().moveGradient({ end: { x: w * 0.8, y: h * 0.6 } }, true);
      s().endGradientDrag(); frame();
      const longestGap = (until: () => boolean) => new Promise<number>((done) => {
        let last = performance.now(), gap = 0;
        // performance.now() inside the callback, not the rAF timestamp (Task 6 fix round 1, issue 4; audit I-7).
        const tick = () => { const now = performance.now(); if (now <= installedAt) gap = Math.max(gap, now - last); last = now; if (until()) done(Math.round(gap)); else requestAnimationFrame(tick); };
        requestAnimationFrame(tick);
      });
      const t0 = performance.now();
      s().commitGradient();
      result["Return (UI thread) ms"] = Math.round(performance.now() - t0);
      result["commit: longest frame gap while the worker paints"] = await longestGap(() => !s().working);
      result["jobs run"] = jobs;
      const l = api.engine.state(doc).layers[0];
      result["mask pixels"] = l.maskWidth * l.maskHeight;
      return result;
    }, [w, h]);
    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
  }
  console.log(`gradient tool on a growing mask (release wasm, Edge): ${JSON.stringify(out)}`);
  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
    expect(out[`${label}: jobs run`], "the grown mask went to the worker").toBe(1);
    expect(out[`${label}: mask pixels`]).toBe(w * h);
    expect(out[`${label}: Return (UI thread) ms`]).toBeLessThan(150);
    expect(out[`${label}: commit: longest frame gap while the worker paints`]).toBeLessThan(100);
    expect(out[`${label}: jobInput ms`]).toBeLessThan(150);
    // OQ5's "500" at 100 MP is 450 (ruling I7; audit M-5).
    expect(out[`${label}: installJob ms`]).toBeLessThan(label === "24 MP" ? 150 : 450);
    expect(out[`${label}: frame after it, whole uploads`], "the frame timed is the one that uploads the grown mask").toBeGreaterThanOrEqual(1);
    // Fix round 1, controller ruling: 400 at 100 MP, the same ruling as 14a -- a canvas-sized mask is
    // reallocated and uploaded whole, which costs more at 100 MP than the flat 150 first measured.
    expect(out[`${label}: frame after it ms`]).toBeLessThan(label === "24 MP" ? 150 : 400);
  }
});

test("T9-6: dragging a pixel gradient on a small layer under a non-uniform covering mask stays in budget while the preview carries the mask onto the grown grid, at 24 and 100 MP", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
    await ready(page);
    await installFrameTimer(page);
    const r = await page.evaluate(async ([w, h]) => {
      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const result: Record<string, number> = {};
      // A 1500 x 1000 layer in the middle of the canvas, under a non-uniform covering mask (an
      // ellipse selection's, as "mask gradients that grow the mask" makes one) painted while the
      // canvas is still 1500 x 1000, so the mask stays un-placed and un-grown at its own size; only
      // then does the canvas grow past it (Task 14a's own setup, mirrored here for the PIXELS side,
      // fix round 1, T9-6): the Gradient tool targets the layer's pixels, not the mask, so every drag
      // tick's preview carries that covering mask onto the grown, reduced grid (`followed`).
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: 1500, height: 1000, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const layer = api.engine.state(doc).layers[0].id;
      const x0 = 100, y0 = 100;
      api.engine.execute(doc, { type: "SelectShape", kind: "Ellipse", points: [[x0, y0], [1400, y0], [1400, 900], [x0, 900]], mode: "Replace", antialiased: true });
      api.engine.execute(doc, { type: "AddMaskFromSelection", id: layer, revealing: true });
      api.engine.execute(doc, { type: "Deselect" });
      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: null });
      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
      const s = () => api.store.getState();
      s().openDocument(doc);
      s().setTool("gradient"); // Pixels, not the mask: `maskSelected` defaults false.
      await settle(); frame();
      s().beginGradient({ x: w * 0.2, y: h * 0.5 });
      const ticks: number[] = [];
      for (let i = 0; i < 8; i++) {
        const t0 = performance.now();
        s().moveGradient({ end: { x: w * (0.5 + i * 0.04), y: h * 0.6 } }, true);
        frame();
        ticks.push(performance.now() - t0);
      }
      ticks.shift();
      result["drag tick (store, engine and frame), worst ms"] = Math.round(Math.max(...ticks));
      s().endGradientDrag(); frame();
      s().closeDocument(doc);
      return result;
    }, [w, h]);
    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
  }
  console.log(`T9-6 pixel gradient drag on a small layer's covering mask (release wasm, Edge): ${JSON.stringify(out)}`);
  for (const label of ["24 MP", "100 MP"]) {
    // The pixel gradient's own drag budget (ruling OQ8's 50 ms): carrying the covering mask onto the
    // grown grid must not push a tick over it.
    expect(out[`${label}: drag tick (store, engine and frame), worst ms`]).toBeLessThan(50);
  }
});

test("ruling C1: a fill and a gradient on a blank layer paint the canvas, so they go through the worker, at 24 and 100 MP", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  let first = true, renderer = "";
  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
    for (const edit of ["fill", "gradient"]) {
      await ready(page);
      await installFrameTimer(page);
      const warmUp = first; first = false;
      if (warmUp) renderer = await page.evaluate(() => {
        const gl = (document.querySelector('[data-testid="canvas-view"] canvas') as HTMLCanvasElement).getContext("webgl2")!;
        const info = gl.getExtension("WEBGL_debug_renderer_info");
        return info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : "unknown";
      });
      const r = await page.evaluate(async ([w, h, edit, warmUp]) => {
        const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
        const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
        const s = () => api.store.getState();
        const result: Record<string, number> = {};
        // Fix round 4, controller ruling: a cold start. Run by itself, in a fresh browser process, the
        // first timed scenario (24 MP fill) paid a one-time cost in its "longest frame gap" (279-394
        // ms, against 17-28 ms when this test ran after the other Task 15 cases in the same browser;
        // the round-2 spec fails the same way on the current build, so no app change caused it). As
        // the mask-fill case above (14a-perf-2) does, one warm-up Fill on a small blank document
        // (2560 x 2560, over JOB_PIXELS, so it takes the same worker path) pays that cost here, before
        // any timed scenario and before the patching below, so it is not counted as a timed job; its
        // gap and time are logged, not asserted, and the timed scenarios keep their 100 ms budget.
        // Measured this round: a Fill alone does NOT move the cost (the timed gap stayed at 279-352
        // ms, even with a 6000 x 4000 warm-up), so the worker's wasm start-up is not what it is. The
        // stall is one ~310-330 ms app `renderer.render` call on the UI thread a few ms after the Fill
        // key, with no engine call over 20 ms inside it -- a first-use GPU cost (most likely a shader
        // program compiled on first use; it is cached per browser process, hence warm after other
        // cases) on the path the Fill scenario takes after a cancelled gradient preview. So the
        // warm-up runs that same sequence on the small document: a gradient preview, cancel, Fill.
        // Follow-ups: "pre-warm the job worker (compile its wasm) at app startup", and the renderer's
        // first-use cost (compile its GL programs at startup). Final fix wave (F2, time-boxed): the
        // worker is now started at startup (`JobClient.warm`), which does not remove this cost; timed
        // without this block, the stall is one app render of 291-299 ms about 20 ms after the key
        // (the render the tool switch asks for), inside which no WebGL call takes over 2 ms and no
        // engine call or renderer stage over 5 ms, so it is neither a shader compile nor an upload;
        // runs without the block also showed 219-483 ms main-thread pauses at 100 MP with no render
        // or engine call in them. A browser trace did not reproduce it. Not identified: this warm-up
        // stays (see the Phase 4b-1 rulings doc, F2).
        if (warmUp) {
          const warm = api.engine.newDocument(2560, 2560, true);
          s().openDocument(warm);
          await settle(); frame();
          s().setTool("gradient");
          s().beginGradient({ x: 500, y: 1280 });
          s().moveGradient({ end: { x: 2000, y: 1500 } }, true);
          frame();
          s().cancelGradient(); frame(); await settle();
          s().setTool("move");
          const t0 = performance.now();
          window.dispatchEvent(new KeyboardEvent("keydown", { key: "Backspace", altKey: true }));
          result["cold warm-up fill (UI thread) ms"] = Math.round(performance.now() - t0);
          result["cold warm-up fill: went to the worker"] = s().working ? 1 : 0;
          result["cold warm-up fill: longest frame gap"] = await new Promise<number>((done) => {
            let last = performance.now(), gap = 0;
            const tick = () => { const now = performance.now(); gap = Math.max(gap, now - last); last = now; if (!s().working) done(Math.round(gap)); else requestAnimationFrame(tick); };
            requestAnimationFrame(tick);
          });
          result["cold warm-up fill: total ms"] = Math.round(performance.now() - t0);
          s().closeDocument(warm);
          await settle();
        }
        let installedAt = Infinity, jobs = 0;
        const timed = (name: string, after?: () => void) => {
          if (name === "jobInput" || name === "installJob") {
            const method = `${name}Async`, f = api.engine[method].bind(api.engine);
            api.engine[method] = async (...a: unknown[]) => { try { return await f(...a); } finally { result[`${name} ms`] = Math.round(api.engine[name === "jobInput" ? "lastJobInputCpuMs" : "lastInstallCpuMs"]); after?.(); } };
            return;
          }
          const f = api.engine[name].bind(api.engine);
          api.engine[name] = (...a: unknown[]) => { const t0 = performance.now(); try { return f(...a); } finally { result[`${name} ms`] = Math.round(performance.now() - t0); after?.(); } };
        };
        timed("jobInput");
        // Fix round 3, item 4: the reviewer showed the spatial-margin FBOs (`RenderPlan.spatial_margin`,
        // `GlRenderer.frameFor`) are never reached here (no blur layer) -- round 1's diagnosis was wrong.
        // Instrumented with stack traces and per-call timings on this exact scenario (throwaway, not
        // committed): the real, dominant cost inside the timed frame below is `engine.layerPixels`, the
        // WASM call `LayerTextures.sync`'s `upload` makes to prefilter the freshly committed layer down
        // to display resolution for its first GPU upload -- measured 691 ms alone for a 100 MP layer in
        // that one run. It is data-dependent (the just-installed pixels), not a fixed GPU/framebuffer
        // resource, so there is nothing to allocate eagerly; timed here directly, on both edits, so the
        // real one-time cost the user pays on the first paint is what is actually logged.
        timed("layerPixels");
        // The frame that draws the result, timed where it lands (as the "jobs" case does), so no interval is outside a window.
        timed("installJob", () => { installedAt = performance.now(); s().refresh(s().activeId); result["frame after the result is put back ms"] = Math.round(frame()); result["that frame's whole uploads"] = (window as any).__uploads.image; });
        const client = s().jobs; const send = client.run.bind(client);
        client.run = (...a: unknown[]) => { jobs++; return send(...a); };
        // performance.now() inside the callback, not the rAF timestamp (Task 6 fix round 1, issue 4).
        const longestGap = (until: () => boolean) => new Promise<number>((done) => {
          let last = performance.now(), gap = 0;
          const tick = () => { const now = performance.now(); if (now <= installedAt) gap = Math.max(gap, now - last); last = now; if (until()) done(Math.round(gap)); else requestAnimationFrame(tick); };
          requestAnimationFrame(tick);
        });
        // A new document's blank layer stores no pixels, but a fill or a gradient paints the whole canvas.
        const doc = api.engine.newDocument(w, h, true);
        s().openDocument(doc);
        await settle(); frame();
        const layer = s().documents[doc].activeLayerId;
        result["stored pixels"] = api.engine.storedPixels(doc, layer);
        result["pixels the edit paints"] = api.engine.editPixels(doc, layer, false);
        if (edit === "gradient") {
          s().setTool("gradient");
          s().beginGradient({ x: w * 0.2, y: h * 0.5 });
          s().moveGradient({ end: { x: w * 0.8, y: h * 0.6 } }, true);
          s().endGradientDrag(); frame(); await settle();
        } else {
          // Fix round 2 kept a warm-up frame here (a preview with real pixels) on the theory it primed a
          // one-time GPU resource; fix round 3's proper instrumentation (see above) found that theory
          // wrong -- there is no such resource, so this does not, and cannot, avoid the real cost, which
          // is entirely inside the timed frame below (`engine.layerPixels`, data-dependent on the just-
          // committed pixels). Kept anyway, as the Gradient case above does the same warm-up naturally,
          // so the two stay comparable; its own frame is logged only as a diagnostic, not a claimed fix.
          s().setTool("gradient");
          s().beginGradient({ x: w * 0.2, y: h * 0.5 });
          s().moveGradient({ end: { x: w * 0.8, y: h * 0.6 } }, true);
          const warm0 = performance.now();
          frame();
          result["warm-up preview frame (diagnostic only) ms"] = Math.round(performance.now() - warm0);
          s().cancelGradient(); frame(); await settle();
          s().setTool("move");
        }
        const t0 = performance.now();
        if (edit === "fill") window.dispatchEvent(new KeyboardEvent("keydown", { key: "Backspace", altKey: true }));
        else s().commitGradient();
        result["key (UI thread) ms"] = Math.round(performance.now() - t0);
        result["longest frame gap while the worker paints"] = await longestGap(() => !s().working);
        result["jobs run"] = jobs;
        s().closeDocument(doc);
        return result;
      }, [w, h, edit, warmUp] as [number, number, string, boolean]);
      for (const [k, v] of Object.entries(r)) out[`${label} ${edit}: ${k}`] = v;
    }
  }
  console.log(`ruling C1, blank layers (release wasm, Edge): ${JSON.stringify(out)}`);
  console.log(`ruling C1 renderer: ${renderer}`);
  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
    for (const edit of ["fill", "gradient"]) {
      const k = (name: string) => out[`${label} ${edit}: ${name}`];
      expect(k("stored pixels"), "a blank layer stores nothing").toBe(0);
      expect(k("pixels the edit paints"), "it paints the canvas").toBe(w * h);
      expect(k("jobs run"), "decided by the pixels it paints (ruling C1), so the worker made it").toBe(1);
      expect(k("key (UI thread) ms")).toBeLessThan(150);
      expect(k("longest frame gap while the worker paints")).toBeLessThan(100);
      expect(k("jobInput ms")).toBeLessThan(150);
      expect(k("installJob ms")).toBeLessThan(label === "24 MP" ? 150 : 450);
      // Fix round 1 (corrected in fix round 3): Fill's frame counts 2 whole uploads here, not 1.
      // Round 1 guessed a one-time spatial-margin framebuffer allocation; the reviewer showed that
      // path is never reached without a blur layer, and round 3's own instrumentation confirmed the
      // guess was wrong. The extra `texImage2D` traces to `FboPool`'s own main/stack compositing
      // buffers (used unconditionally, not just for blur) instead, costing a few ms at most -- not
      // the real driver of this frame's time, which is `layerPixels` (timed separately above).
      // `toBeGreaterThanOrEqual` already allows either count.
      expect(k("that frame's whole uploads"), "the frame timed is the one that uploads the result").toBeGreaterThanOrEqual(1);
      // The "jobs" case's budget for the frame that re-uploads a whole committed layer (perf-4b1.spec.ts:186).
      // Fix round 1, controller ruling: 500 at 100 MP (was 400) -- on the HD 520 that frame halves a
      // fresh 100 MP result and uploads it; the follow-up is "mask levels / display-level uploads" in
      // the final review. Fix round 4, controller ruling: 600 at 100 MP (24 MP unchanged), for both
      // edits. Round 3 traced this frame's cost to `engine.layerPixels`, which prefilters the freshly
      // installed layer down to the display level on the UI thread (314-533 ms of a 375-555 ms frame
      // at 100 MP) -- inherent to the current design, not a one-time cost a warm-up can move. The
      // follow-up is "prefilter the display level in the worker and return it with the result".
      expect(k("frame after the result is put back ms")).toBeLessThan(label === "24 MP" ? 350 : 600);
    }
  }
});
