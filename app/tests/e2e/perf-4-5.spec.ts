import { test, expect } from "@playwright/test";

// Phase 4.5's timings (LL-073), in the release engine and on the real GPU: run `pnpm wasm`, then
// `$env:PERF = "1"; npx playwright test app/tests/e2e/perf-4-5.spec.ts`, and rebuild `pnpm wasm:dev`
// afterwards. The installed Edge runs WebGL on the GPU as WebView2 does; each test logs
// UNMASKED_RENDERER_WEBGL, and a software renderer (SwiftShader, Basic Render) invalidates a run. Each
// test prints what it measured and checks its budgets. Every test opens a fresh page per size and warms
// the path up before timing it (LL-074): the cold costs are logged, never asserted.
test.skip(!process.env.PERF, "set PERF=1 after pnpm wasm to measure");
test.use({ channel: "msedge", viewport: { width: 1440, height: 900 } });

const SIZES: [string, number, number][] = [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]];

/** Opens the page and waits for the engine; every texture upload is counted in `__uploads`. Returns the
 * GPU's name (UNMASKED_RENDERER_WEBGL). */
async function ready(page: import("@playwright/test").Page): Promise<string> {
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
  // `__frame()`: one render of the active document through the app's renderer, ended by a 1 x 1
  // readPixels so the GPU work lands inside the timing; its milliseconds. `__uploads` then says how many
  // whole and partial texture uploads that frame made.
  return page.evaluate(() => {
    const api = (window as any).__compositor;
    const gl = (document.querySelector('[data-testid="canvas-view"] canvas') as HTMLCanvasElement).getContext("webgl2")!;
    const px = new Uint8Array(4);
    (window as any).__frame = () => {
      const s = api.store.getState();
      const log = (window as any).__uploads; log.image = 0; log.sub = 0;
      const t0 = performance.now();
      api.renderer.render(api.engine, s.documents[s.activeId], s.viewports[s.activeId], window.devicePixelRatio || 1, { checkerboard: true }, s.previewEdit());
      gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px);
      return performance.now() - t0;
    };
    const info = gl.getExtension("WEBGL_debug_renderer_info");
    return info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : "unknown";
  });
}

// Fix round 1, I2: the pre-flight's 150 ms budget at 24 MP was never measured; ops/selection.rs's
// `clear_selected` (predating this task: a to_vec, a per-pixel pass and a memcmp over the whole layer,
// O(layer) not O(selection)) is the review's native-bench-confirmed cost, and `seed_adopted` adds only
// about 5-11 ms on top of it. Measured warmed (one full round run first, LL-074) both alone and in the
// whole file: see task-3-report.md, "Fix round 1" for the numbers and which of these applied.
const DELETE_BUDGET_24MP = 150;
const ROUNDS = 5;

test("F1: a Delete inside a selection after a job's result at 24 and 100 MP, at fit and at 40 %: the adopted halving seeded, not made again", async ({ page }) => {
  // `seed_halvings` -> `seed_adopted` (raster.rs): after a whole-layer result is adopted, an edit inside a selection on
  // the UI thread copies the adopted halving (1/4^level of the layer: 6.25 MB at level 3 of 100 MP, 100 MB at level 1)
  // and halves again only the selection's reach. 40 % gives level 1, the dearest copy.
  //
  // Fix round 1, I1: a Delete inside a selection is a PARTIAL edit. The renderer's partial-upload path
  // (gl-renderer.ts:150-153) calls `pixelsDelta` then `engine.layerRegion` (`Engine::layer_region`,
  // engine.rs:342's non-preview branch, which itself goes through `layer_raster` -> `Raster::reduced`),
  // never `engine.layerPixels` (the whole-upload path `layer_pixels_ptr` serves). A broken `seed_adopted`
  // cannot show up under `layerPixels` here -- it shows up under `layerRegion`, so that is what is timed
  // and asserted; `layerPixels` is still timed and logged (never 0 would be a surprise worth seeing) but
  // no longer asserted, since with `whole == 0` it can only ever restate that assertion.
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  let renderer = "";
  for (const [label, w, h] of SIZES) {
    renderer = await ready(page);
    const r = await page.evaluate(async ([w, h, ROUNDS]) => {
      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const s = () => api.store.getState();
      const idle = () => new Promise<void>((done) => { const poll = () => (s().working ? setTimeout(poll, 20) : done()); poll(); });
      const result: Record<string, number> = {};
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const layer = api.engine.state(doc).layers[0].id;
      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
      s().openDocument(doc);
      await settle(); frame();
      // Each install's display halving (the fill's result adopted); `layerPixels` (the whole-upload
      // halving, logged only, I1) and `layerRegion` (the partial-upload path this scenario actually
      // takes) timed inside the frame after the Delete.
      let inFrame = false, halving = 0, region = 0;
      const displays: number[] = [];
      const install = api.engine.installJobAsync.bind(api.engine);
      api.engine.installJobAsync = (...a: unknown[]) => { displays.push((a[6] as ArrayBuffer | null)?.byteLength ?? 0); return install(...a); };
      const pixels = api.engine.layerPixels.bind(api.engine);
      api.engine.layerPixels = (...a: unknown[]) => { const t = performance.now(); try { return pixels(...a); } finally { if (inFrame) halving += performance.now() - t; } };
      const layerRegion = api.engine.layerRegion.bind(api.engine);
      api.engine.layerRegion = (...a: unknown[]) => { const t = performance.now(); try { return layerRegion(...a); } finally { if (inFrame) region += performance.now() - t; } };
      // One round: the whole layer filled through the worker (its result comes back halved to the canvas's level and
      // is adopted), then a 1024 x 1024 marquee cleared on the UI thread, as the Delete key does (`deleteKeyPressed`).
      // `n` only offsets the marquee so each round clears a fresh, still-filled patch; `wasmBytes()` (I2) is logged
      // so heap growth shows up in the numbers instead of being guessed at.
      const round = async (n: number) => {
        if (api.engine.state(doc).selection) s().run({ type: "Deselect" });
        const before = displays.length;
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "Backspace", altKey: true }));
        await idle(); frame(); await settle();
        const adopted = displays.length > before && displays[displays.length - 1] > 0 ? 1 : 0;
        const x = Math.round(w / 2 - 512 + 64 * n), y = Math.round(h / 2 - 512);
        s().run({ type: "SelectShape", kind: "Rectangle", points: [[x, y], [x + 1024, y], [x + 1024, y + 1024], [x, y + 1024]], mode: "Replace", antialiased: false });
        frame(); await settle();
        const t0 = performance.now();
        s().run({ type: "ClearSelectedPixels", id: layer, mask: false });
        const step = performance.now() - t0;
        halving = 0; region = 0; inFrame = true;
        const after = frame();
        inFrame = false;
        const whole = (window as any).__uploads.image as number;
        const sub = (window as any).__uploads.sub as number;
        const cleared = api.engine.composite(doc, { x: x + 512, y: y + 512, width: 1, height: 1 }, 1, 1)[3] === 0 ? 1 : 0;
        const memory = api.engine.wasmBytes();
        await settle();
        return { step, after, halving, region, whole, sub, adopted, cleared, memory };
      };
      for (const zoom of ["fit", "40 %"]) {
        if (zoom === "40 %") { await api.setZoom(0.4); await settle(); frame(); }
        // A full warm round (I2, discarded entirely: `n` -1 clears its own patch, never read below) settles
        // the heap (allocator growth, GC) before anything is timed.
        await round(-1);
        // Cold (LL-074): the first TIMED round at this zoom, logged only.
        const cold = await round(0);
        result[`${zoom}: cold Delete ms`] = Math.round(cold.step);
        result[`${zoom}: cold frame after ms`] = Math.round(cold.after);
        result[`${zoom}: cold memory bytes`] = cold.memory;
        let step = 0, after = 0, halved = 0, region = 0, whole = 0, sub = 0, adopted = 0, cleared = 0, memory = 0;
        const afters: number[] = [];
        for (let n = 1; n <= ROUNDS; n++) {
          const t = await round(n);
          step = Math.max(step, t.step); after = Math.max(after, t.after); halved = Math.max(halved, t.halving); region = Math.max(region, t.region);
          whole += t.whole; sub += t.sub; adopted += t.adopted; cleared += t.cleared; memory = Math.max(memory, t.memory);
          afters.push(t.after);
        }
        // I3: the frame after only calls renderPlan, pixelsDelta and crops at most 1 MB out of the
        // adopted halving -- the worst-of-N spikes seen on this laptop are jitter (GC, the OS scheduler),
        // not this frame's own cost, so the budget is asserted on the median, with the worst logged.
        afters.sort((a, b) => a - b);
        const medianAfter = afters[Math.floor(afters.length / 2)];
        result[`${zoom}: Delete (UI thread), worst of ${ROUNDS} ms`] = Math.round(step);
        result[`${zoom}: frame after, median of ${ROUNDS} ms`] = Math.round(medianAfter);
        result[`${zoom}: frame after, worst of ${ROUNDS} ms`] = Math.round(after);
        result[`${zoom}: layerPixels in the frame after, worst ms`] = Math.round(halved);
        result[`${zoom}: layerRegion in the frame after, worst ms`] = Math.round(region);
        result[`${zoom}: whole uploads in the frames after`] = whole;
        result[`${zoom}: partial uploads in the frames after`] = sub;
        result[`${zoom}: rounds whose fill came back halved`] = adopted;
        result[`${zoom}: rounds whose Delete cleared the centre`] = cleared;
        result[`${zoom}: memory bytes, worst`] = memory;
        result[`${zoom}: canvas scale x 1000`] = Math.round(s().viewports[doc].pointsPerPixel * (window.devicePixelRatio || 1) * 1000);
      }
      s().closeDocument(doc);
      return result;
    }, [w, h, ROUNDS] as [number, number, number]);
    for (const [k, v] of Object.entries(r)) out[`${label} ${k}`] = v;
  }
  console.log(`F1 Delete after a job (release wasm, Edge): ${JSON.stringify(out)}`);
  console.log(`F1 Delete after a job renderer: ${renderer}`);
  // Budgets (fix round 1): 4b-1's for the frame after a partial edit, on its median (33 ms, no whole
  // upload: perf-4b1.spec.ts, ruling M4, I3); for the Delete key on the UI thread, `DELETE_BUDGET_24MP`
  // at 24 MP (see its own comment for which value and why) and 400 ms at 100 MP, where the seeding
  // copies up to 100 MB at level 1. `layerRegion`, not `layerPixels` (I1), is F1's own budget here: the
  // adopted halving was seeded, so the partial upload crops it and halves nothing.
  for (const [label] of SIZES) for (const zoom of ["fit", "40 %"]) {
    const k = (name: string) => out[`${label} ${zoom}: ${name}`];
    expect(k("rounds whose fill came back halved"), "every fill's result came back halved and was adopted").toBe(ROUNDS);
    expect(k("rounds whose Delete cleared the centre"), "every Delete ran").toBe(ROUNDS);
    expect(k("whole uploads in the frames after"), "a partial edit uploads no whole texture").toBe(0);
    expect(k("partial uploads in the frames after"), "each Delete's frame took the partial path").toBeGreaterThanOrEqual(ROUNDS);
    expect(k("layerRegion in the frame after, worst ms"), "the adopted halving was seeded: the partial upload crops it and halves nothing").toBeLessThan(20);
    expect(k(`frame after, median of ${ROUNDS} ms`)).toBeLessThan(33);
    expect(k(`Delete (UI thread), worst of ${ROUNDS} ms`)).toBeLessThan(label === "24 MP" ? DELETE_BUDGET_24MP : 400);
  }
});

test("F1: the frame after a job's result at 24 and 100 MP, Levels on a layer and a fill of a blank layer, with no halving on the UI thread", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  let renderer = "";
  for (const [label, w, h] of SIZES) {
    for (const edit of ["levels", "fill"]) {
      renderer = await ready(page);
      const r = await page.evaluate(async ([w, h, edit]) => {
        const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
        const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
        const s = () => api.store.getState();
        const result: Record<string, number> = {};
        const idle = () => new Promise<void>((done) => { const poll = () => (s().working ? setTimeout(poll, 20) : done()); poll(); });
        // Warm-up (LL-074): the timed path itself on a smaller document - Levels committed through the worker and
        // the frame that uploads its adopted halving - then a fill through the worker, so neither the worker's first
        // jobs nor the renderer's first draw of a job's result is timed; every cost is logged, none asserted.
        const warm = api.engine.newDocument(10, 10, false);
        api.engine.execute(warm, { type: "CanvasSize", width: 2560, height: 2560, anchor: 4, fill: [0.5, 0.4, 0.3] });
        api.engine.execute(warm, { type: "SetActiveLayer", id: api.engine.state(warm).layers[0].id });
        s().openDocument(warm);
        await settle(); frame();
        let t0 = performance.now();
        s().beginAdjust({ kind: "Levels" });
        await new Promise<void>((done) => { const poll = () => (s().adjustEdit?.histogram === null ? setTimeout(poll, 20) : done()); poll(); });
        const warmLevels = JSON.parse(JSON.stringify(s().adjustEdit.adjustment));
        warmLevels.levels.ranges[0].outputWhite = 200;
        s().updateAdjust({ adjustment: warmLevels });
        await new Promise<void>((done) => { const poll = () => (s().previewSettling() ? setTimeout(poll, 20) : done()); poll(); });
        s().commitAdjust();
        await idle();
        result["cold warm-up Levels through the worker ms"] = Math.round(performance.now() - t0);
        result["cold warm-up frame after the Levels ms"] = Math.round(frame());
        t0 = performance.now();
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "Backspace", altKey: true }));
        await idle();
        result["cold warm-up fill through the worker ms"] = Math.round(performance.now() - t0);
        result["cold warm-up frame after ms"] = Math.round(frame());
        s().closeDocument(warm);
        await settle();
        // The timed document, drawn once at fit before the edit.
        let doc: string;
        if (edit === "levels") {
          doc = api.engine.newDocument(10, 10, false);
          api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
          api.engine.execute(doc, { type: "SetActiveLayer", id: api.engine.state(doc).layers[0].id });
        } else doc = api.engine.newDocument(w, h, true);
        s().openDocument(doc);
        await settle(); frame();
        // The UI thread's copies, timed where the store makes them; the frame that draws the result is
        // timed inside the install, so the gap windows below are contiguous with it, and `layerPixels`
        // (the halving that used to run here) is timed inside that frame.
        let installedAt = Infinity, inFrame = false, halving = 0, displays: number[] = [];
        const timed = (name: string, before?: (a: unknown[]) => void, after?: () => void) => {
          const f = api.engine[name].bind(api.engine);
          api.engine[name] = (...a: unknown[]) => { before?.(a); const t = performance.now(); try { return f(...a); } finally { const ms = performance.now() - t; if (name === "layerPixels") { if (inFrame) halving += ms; } else result[`${name} ms`] = Math.round(ms); after?.(); } };
        };
        // Cooperatively copied buffers now cross through the async APIs. Keep
        // the original UI-work budgets: count the time spent copying/installing,
        // excluding frame yields, and still measure every frame gap below.
        const timedAsync=(name:string,key:string,cpu:string,before?:(a:unknown[])=>void,after?:()=>void)=>{
          const f=api.engine[name].bind(api.engine);api.engine[name]=async(...a:unknown[])=>{before?.(a);const result=await f(...a);reread();after?.();return result;};
          const reread=()=>{result[`${key} ms`]=Math.round(api.engine[cpu]);};
        };
        timedAsync("jobInputAsync","jobInput","lastJobInputCpuMs");
        timed("layerPixels");
        timedAsync("installJobAsync","installJob","lastInstallCpuMs", (a) => displays.push((a[6] as ArrayBuffer | null)?.byteLength ?? 0), () => {
          installedAt = performance.now();
          s().refresh(s().activeId);
          inFrame = true;
          result["frame after the result is put back ms"] = Math.round(frame());
          inFrame = false;
          result["that frame's whole uploads"] = (window as any).__uploads.image;
        });
        const longestGap = (until: () => boolean) => new Promise<number>((done) => {
          let last = performance.now(), gap = 0;
          const tick = () => { const now = performance.now(); if (now <= installedAt) gap = Math.max(gap, now - last); last = now; if (until()) done(Math.round(gap)); else requestAnimationFrame(tick); };
          requestAnimationFrame(tick);
        });
        if (edit === "levels") {
          s().beginAdjust({ kind: "Levels" });
          await new Promise<void>((done) => { const poll = () => (s().adjustEdit?.histogram === null ? setTimeout(poll, 20) : done()); poll(); });
          const adjustment = JSON.parse(JSON.stringify(s().adjustEdit.adjustment));
          adjustment.levels.ranges[0].outputWhite = 200;
          s().updateAdjust({ adjustment });
          await new Promise<void>((done) => { const poll = () => (s().previewSettling() ? setTimeout(poll, 20) : done()); poll(); });
          await new Promise((r) => setTimeout(r, 300)); await settle();
          t0 = performance.now();
          s().commitAdjust();
        } else {
          t0 = performance.now();
          window.dispatchEvent(new KeyboardEvent("keydown", { key: "Backspace", altKey: true }));
        }
        result["the key or OK (UI thread) ms"] = Math.round(performance.now() - t0);
        result["longest frame gap while the worker edits"] = await longestGap(() => !s().working);
        result["done after ms"] = Math.round(performance.now() - t0);
        result["layerPixels in the frame after ms"] = Math.round(halving);
        result["display buffer bytes"] = displays[0] ?? -1;
        const vp = s().viewports[doc];
        result["canvas scale x 1000"] = Math.round(vp.pointsPerPixel * (window.devicePixelRatio || 1) * 1000);
        s().closeDocument(doc);
        return result;
      }, [w, h, edit] as [number, number, string]);
      for (const [k, v] of Object.entries(r)) out[`${label} ${edit}: ${k}`] = v;
    }
  }
  console.log(`F1 (release wasm, Edge): ${JSON.stringify(out)}`);
  console.log(`F1 renderer: ${renderer}`);
  for (const [label] of SIZES) {
    for (const edit of ["levels", "fill"]) {
      const k = (name: string) => out[`${label} ${edit}: ${name}`];
      expect(k("display buffer bytes"), "the worker sent the result halved to the canvas's level").toBeGreaterThan(0);
      expect(k("that frame's whole uploads"), "the frame timed is the one that uploads the result").toBeGreaterThanOrEqual(1);
      // The halving that cost 314-533 ms at 100 MP on the UI thread (phase4b1 rulings, F1) is gone: the
      // frame reads the adopted halving.
      expect(k("layerPixels in the frame after ms"), "no halving on the UI thread").toBeLessThan(20);
      // F1's acceptance was 350 / 400 ms or lower (the 4b-1 budgets were 350 and 500-600); measured on the
      // HD 520 (p45-scratch, 2026-09-30): 8-21 ms at 24 MP and 12-23 ms at 100 MP, one upload of the
      // adopted halving. 100 ms at both sizes leaves room for this laptop's swings (ruling OQ3).
      expect(k("frame after the result is put back ms")).toBeLessThan(100);
      expect(k("longest frame gap while the worker edits")).toBeLessThan(100);
      expect(k("jobInput ms")).toBeLessThan(label === "24 MP" ? 150 : 450);
      expect(k("installJob ms")).toBeLessThan(label === "24 MP" ? 150 : 450);
    }
  }
});

test("1.4.5 results: frames of a document with a Soft Light layer, Levels in Linear Dodge, a stack based in Vivid Light and Hue/Saturation +50, at 24 and 100 MP", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  let renderer = "";
  for (const [label, w, h] of SIZES) {
    renderer = await ready(page);
    const r = await page.evaluate(async ([w, h]) => {
      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const s = () => api.store.getState();
      const result: Record<string, number> = {};
      const run = (doc: string, cmd: unknown) => api.engine.execute(doc, cmd);
      const doc = api.engine.newDocument(10, 10, false);
      run(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const base = api.engine.state(doc).layers[0].id;
      // A Soft Light copy, a stack based in Vivid Light with a half-opaque copy clipped to it, a Levels
      // layer in Linear Dodge and a Hue/Saturation layer at +50: every result Tasks 6-8 changed.
      const top = () => api.engine.state(doc).activeLayerId as string;
      run(doc, { type: "DuplicateLayer", id: base }); run(doc, { type: "SetLayerBlendMode", id: top(), mode: "Soft Light" });
      run(doc, { type: "DuplicateLayer", id: base }); const stack = top(); run(doc, { type: "SetLayerBlendMode", id: stack, mode: "Vivid Light" });
      run(doc, { type: "DuplicateLayer", id: base }); const child = top();
      run(doc, { type: "SetLayerOpacity", id: child, opacity: 0.5 }); run(doc, { type: "ToggleClipping", id: child });
      run(doc, { type: "AddAdjustmentLayer", kind: "Levels", seed: 0, shadows: null, highlights: null });
      run(doc, { type: "SetLayerBlendMode", id: top(), mode: "Linear Dodge (Add)" });
      run(doc, { type: "AddAdjustmentLayer", kind: "Hue/Saturation", seed: 0, shadows: null, highlights: null });
      const hsv = api.engine.state(doc).layers.find((l: any) => l.id === top());
      const saturate = (amount: number) => run(doc, { type: "SetAdjustment", id: hsv.id, adjustment: { ...hsv.adjustment,
        hsvSettings: { range: "Master", colorize: false, invertRange: false,
          adjustments: { Master: { hue: 0, saturation: amount, lightness: 0 } }, bands: hsv.adjustment.hsvSettings?.bands ?? {} } } });
      saturate(50);
      s().openDocument(doc);
      await settle();
      // Cold (LL-074): the first frame compiles the programs and uploads every layer; logged only.
      result["cold first frame ms"] = Math.round(frame());
      for (const zoom of ["fit", "1:1"]) {
        if (zoom === "1:1") { await api.setZoom(1); await settle(); result["cold first 1:1 frame ms"] = Math.round(frame()); }
        let worst = 0;
        for (let i = 0; i < 5; i++) { s().invalidate(); worst = Math.max(worst, frame()); await settle(); }
        result[`${zoom}: frame, worst of 5 ms`] = Math.round(worst);
        // Dragging the Saturation field: the engine's edit, the store and the frame, per step. One step first,
        // logged only (LL-074): a first run measured 834 ms for the first step at 100 MP fit, 16-31 ms after.
        let t = performance.now(); saturate(45); s().refresh(doc); frame();
        result[`cold first saturation step at ${zoom} ms`] = Math.round(performance.now() - t);
        await settle();
        let step = 0, gap = 0;
        // One contiguous window over the steps: each gap is taken inside the rAF callback after its step (LL-074).
        let last = performance.now();
        for (const amount of [40, 45, 55, 60, 50]) {
          const t0 = performance.now();
          saturate(amount); s().refresh(doc);
          frame();
          step = Math.max(step, performance.now() - t0);
          await new Promise<void>((done) => requestAnimationFrame(() => { const now = performance.now(); gap = Math.max(gap, now - last); last = now; done(); }));
        }
        await settle();
        result[`${zoom}: saturation step (engine, store and frame), worst ms`] = Math.round(step);
        result[`${zoom}: longest frame gap over the steps ms`] = Math.round(gap);
      }
      s().closeDocument(doc);
      return result;
    }, [w, h] as [number, number]);
    for (const [k, v] of Object.entries(r)) out[`${label} ${k}`] = v;
  }
  console.log(`1.4.5 results (release wasm, Edge): ${JSON.stringify(out)}`);
  console.log(`1.4.5 results renderer: ${renderer}`);
  // Measured on the HD 520 (p45-scratch, 2026-09-30, three runs): frames 24-42 ms and saturation steps 26-43 ms at both
  // sizes and zooms; the seven-layer document with a group costs about what 4b-1's drag tick did (50 ms budget).
  for (const [label] of SIZES) for (const zoom of ["fit", "1:1"]) {
    expect(out[`${label} ${zoom}: longest frame gap over the steps ms`], "each gap holds one step and one frame").toBeLessThan(100);
    expect(out[`${label} ${zoom}: frame, worst of 5 ms`]).toBeLessThan(80);
    expect(out[`${label} ${zoom}: saturation step (engine, store and frame), worst ms`]).toBeLessThan(100);
  }
});
test("Add Mask with a selection at 24 and 100 MP: Reveal Selection and Hide Selection, the step and the frame after", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  let renderer = "";
  for (const [label, w, h] of SIZES) {
    renderer = await ready(page);
    const r = await page.evaluate(async ([w, h]) => {
      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const s = () => api.store.getState();
      const result: Record<string, number> = {};
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const layer = api.engine.state(doc).layers[0].id;
      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
      s().openDocument(doc);
      await settle(); frame();
      // An antialiased ellipse over the middle 80%, then the button (Reveal) or Alt-click (Hide): the
      // store's step (the engine rasterizes the clip at the layer's size) and the frame that uploads the mask.
      const once = async (revealing: boolean) => {
        api.engine.execute(doc, { type: "SelectShape", kind: "Ellipse", points: [[w * 0.1, h * 0.1], [w * 0.9, h * 0.1], [w * 0.9, h * 0.9], [w * 0.1, h * 0.9]], mode: "Replace", antialiased: true });
        s().refresh(doc); frame(); await settle();
        const t0 = performance.now();
        s().run({ type: "AddMaskFromSelection", id: layer, revealing });
        const step = performance.now() - t0;
        const after = frame();
        await settle();
        api.engine.execute(doc, { type: "DeleteMask", id: layer }); s().refresh(doc); frame(); await settle();
        return [Math.round(step), Math.round(after)];
      };
      // Cold (LL-074): the first one at this size, logged only.
      [result["cold step ms"], result["cold frame after ms"]] = await once(true);
      for (const [name, revealing] of [["Reveal Selection", true], ["Hide Selection", false]] as [string, boolean][]) {
        let step = 0, after = 0;
        for (let i = 0; i < 3; i++) { const [a, b] = await once(revealing); step = Math.max(step, a); after = Math.max(after, b); }
        result[`${name}: step, worst of 3 ms`] = step;
        result[`${name}: frame after, worst of 3 ms`] = after;
      }
      s().closeDocument(doc);
      return result;
    }, [w, h] as [number, number]);
    for (const [k, v] of Object.entries(r)) out[`${label} ${k}`] = v;
  }
  console.log(`Add Mask with a selection (release wasm, Edge): ${JSON.stringify(out)}`);
  console.log(`Add Mask renderer: ${renderer}`);
  // Measured on the HD 520 (p45-scratch, 2026-09-30, two runs): steps 162-212 ms at 24 MP and 506-545 ms at 100 MP
  // (the clip rasterized at the layer's size on the UI thread, as since Phase 4a; the swap of tones costs
  // nothing, ruling OQ9); frames after 19-20 ms and 59-77 ms. The frame budgets are 4b-1's for a whole mask
  // upload (150 / 400); the step ones leave this laptop's swings room (ruling OQ17).
  for (const [label] of SIZES) for (const name of ["Reveal Selection", "Hide Selection"]) {
    expect(out[`${label} ${name}: step, worst of 3 ms`]).toBeLessThan(label === "24 MP" ? 400 : 1000);
    expect(out[`${label} ${name}: frame after, worst of 3 ms`]).toBeLessThan(label === "24 MP" ? 150 : 400);
  }
});
test("Inverse at 24 and 100 MP: of Select All (no selection left) and of a marquee, the step and the frame after", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  let renderer = "";
  for (const [label, w, h] of SIZES) {
    renderer = await ready(page);
    const r = await page.evaluate(async ([w, h]) => {
      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const s = () => api.store.getState();
      const result: Record<string, number> = {};
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
      s().openDocument(doc);
      await settle(); frame();
      const marquee = { type: "SelectShape", kind: "Rectangle", points: [[w * 0.2, h * 0.2], [w * 0.8, h * 0.2], [w * 0.8, h * 0.8], [w * 0.2, h * 0.8]], mode: "Replace", antialiased: true };
      // Ctrl+Shift+I as the key sends it (useShortcuts: `select-inverse`), the store's run and the frame after.
      const once = async (select: unknown) => {
        s().run(select); frame(); await settle();
        const t0 = performance.now();
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "I", ctrlKey: true, shiftKey: true }));
        const step = performance.now() - t0;
        const after = frame();
        const none = api.engine.state(doc).selection === null ? 1 : 0;
        await settle();
        return [Math.round(step), Math.round(after), none];
      };
      [result["cold step ms"], result["cold frame after ms"]] = await once({ type: "SelectAll" });
      for (const [name, select] of [["of Select All", { type: "SelectAll" }], ["of a marquee", marquee]] as [string, unknown][]) {
        let step = 0, after = 0, none = 0;
        for (let i = 0; i < 3; i++) { const [a, b, n] = await once(select); step = Math.max(step, a); after = Math.max(after, b); none += n; }
        result[`${name}: step, worst of 3 ms`] = step;
        result[`${name}: frame after, worst of 3 ms`] = after;
        result[`${name}: times no selection was left`] = none;
      }
      s().closeDocument(doc);
      return result;
    }, [w, h] as [number, number]);
    for (const [k, v] of Object.entries(r)) out[`${label} ${k}`] = v;
  }
  console.log(`Inverse (release wasm, Edge): ${JSON.stringify(out)}`);
  console.log(`Inverse renderer: ${renderer}`);
  for (const [label] of SIZES) {
    expect(out[`${label} of Select All: times no selection was left`], "the inverse of everything is no selection").toBe(3);
    expect(out[`${label} of a marquee: times no selection was left`]).toBe(0);
    // Measured on the HD 520 (p45-scratch, 2026-09-30, two runs): steps 1 ms and frames 4 ms at both sizes (the
    // outline is geometry, not pixels). A step inside a frame and 4b-1's frame budget.
    for (const name of ["of Select All", "of a marquee"]) {
      expect(out[`${label} ${name}: step, worst of 3 ms`]).toBeLessThan(16);
      expect(out[`${label} ${name}: frame after, worst of 3 ms`]).toBeLessThan(33);
    }
  }
});
test("Ungroup Layers at 24 and 100 MP: the step and the frame after", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  let renderer = "";
  for (const [label, w, h] of SIZES) {
    renderer = await ready(page);
    const r = await page.evaluate(async ([w, h]) => {
      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const s = () => api.store.getState();
      const result: Record<string, number> = {};
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const base = api.engine.state(doc).layers[0].id;
      // A folder at half opacity holding a full-size copy in Screen and a copy clipped to it:
      // ungrouping drops the folder's look, so the frame after composites the children anew.
      api.engine.execute(doc, { type: "DuplicateLayer", id: base });
      const a = api.engine.state(doc).activeLayerId;
      api.engine.execute(doc, { type: "SetLayerBlendMode", id: a, mode: "Screen" });
      api.engine.execute(doc, { type: "DuplicateLayer", id: a });
      const b = api.engine.state(doc).activeLayerId;
      api.engine.execute(doc, { type: "SetLayerBlendMode", id: b, mode: "Normal" });
      api.engine.execute(doc, { type: "SetLayerOpacity", id: b, opacity: 0.5 });
      api.engine.execute(doc, { type: "ToggleClipping", id: b });
      s().openDocument(doc);
      await settle(); frame();
      const once = async () => {
        api.engine.execute(doc, { type: "GroupLayers", ids: [a, b] });
        const folder = api.engine.state(doc).activeLayerId;
        api.engine.execute(doc, { type: "SetLayerOpacity", id: folder, opacity: 0.5 });
        s().refresh(doc); s().selectLayers([folder], folder); frame(); await settle();
        const t0 = performance.now();
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "G", ctrlKey: true, shiftKey: true }));
        const left = api.engine.state(doc).layers.some((l: any) => l.id === folder) ? 1 : 0;
        const step = performance.now() - t0;
        const after = frame();
        await settle();
        return [Math.round(step), Math.round(after), left];
      };
      [result["cold step ms"], result["cold frame after ms"]] = await once();
      let step = 0, after = 0, left = 0;
      for (let i = 0; i < 3; i++) { const [x, y, f] = await once(); step = Math.max(step, x); after = Math.max(after, y); left += f; }
      result["folders left after Shift+Ctrl+G"] = left;
      result["step, worst of 3 ms"] = step;
      result["frame after, worst of 3 ms"] = after;
      s().closeDocument(doc);
      return result;
    }, [w, h] as [number, number]);
    for (const [k, v] of Object.entries(r)) out[`${label} ${k}`] = v;
  }
  console.log(`Ungroup (release wasm, Edge): ${JSON.stringify(out)}`);
  console.log(`Ungroup renderer: ${renderer}`);
  for (const [label] of SIZES) {
    expect(out[`${label} folders left after Shift+Ctrl+G`], "every timed step ungrouped").toBe(0);
    // Measured on the HD 520 (p45-scratch, 2026-09-30, two runs): steps 2-5 ms and frames after 10-13 ms at both sizes
    // (a structure change: no pixels move). A step inside a frame and 4b-1's frame budget.
    expect(out[`${label} step, worst of 3 ms`]).toBeLessThan(16);
    expect(out[`${label} frame after, worst of 3 ms`]).toBeLessThan(33);
  }
});

test("dragging a tab at 24 and 100 MP: the tick that starts the drag (it selects the tab) and every later tick, as frame gaps", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  let renderer = "";
  for (const [label, w, h] of SIZES) {
    renderer = await ready(page);
    // Two documents of this size and a small one; the drag takes the second big one's tab to the end.
    const ids = await page.evaluate(async ([w, h]) => {
      const api = (window as any).__compositor;
      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const make = (width: number, height: number) => {
        const doc = api.engine.newDocument(10, 10, false);
        api.engine.execute(doc, { type: "CanvasSize", width, height, anchor: 4, fill: [0.5, 0.4, 0.3] });
        api.store.getState().openDocument(doc);
        return doc as string;
      };
      const ids = [make(w, h), make(w, h), make(256, 256)];
      await settle();
      return ids;
    }, [w, h] as [number, number]);
    const tab = (id: string) => page.locator(`[data-testid="project-tab"][data-doc-id="${id}"]`);
    // Warm-up (LL-074): switch to each tab once so every document's layers are on the GPU.
    for (const id of ids) { await tab(id).click(); await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)))); }
    await tab(ids[0]).click();
    await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
    // Frame gaps in one contiguous window from the press to the release.
    await page.evaluate(() => {
      const w = window as any; w.__gaps = [] as number[]; w.__gapping = true;
      let last = performance.now();
      const tick = () => { const now = performance.now(); w.__gaps.push(now - last); last = now; if (w.__gapping) requestAnimationFrame(tick); };
      requestAnimationFrame(tick);
    });
    const box = (await tab(ids[1]).boundingBox())!;
    const end = (await tab(ids[2]).boundingBox())!;
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    const startAt = await page.evaluate(() => (window as any).__gaps.length);
    await page.mouse.move(box.x + box.width / 2 + 6, box.y + box.height / 2);
    await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
    const afterStart = await page.evaluate(() => (window as any).__gaps.length);
    const steps = 20;
    for (let i = 1; i <= steps; i++) await page.mouse.move(box.x + box.width / 2 + 6 + (end.x + end.width - box.x) * i / steps, box.y + box.height / 2);
    await page.mouse.up();
    const r = await page.evaluate(([startAt, afterStart, ids]) => {
      const w = window as any; w.__gapping = false;
      const gaps: number[] = w.__gaps;
      const s = w.__compositor.store.getState();
      return {
        start: Math.round(Math.max(0, ...gaps.slice(startAt, afterStart))),
        later: Math.round(Math.max(0, ...gaps.slice(afterStart))),
        moved: s.order.join() === [ids[0], ids[2], ids[1]].join() ? 1 : 0,
        active: s.activeId === ids[1] ? 1 : 0,
      };
    }, [startAt, afterStart, ids] as [number, number, string[]]);
    out[`${label} longest gap around the tick that starts the drag, ms`] = r.start;
    out[`${label} longest gap over the later ticks, ms`] = r.later;
    out[`${label} dropped at the end`] = r.moved;
    out[`${label} the dragged tab is active`] = r.active;
    await page.evaluate((ids) => { const s = (window as any).__compositor.store.getState(); for (const id of ids) s.closeDocument(id); }, ids);
  }
  console.log(`tab drag (release wasm, Edge): ${JSON.stringify(out)}`);
  console.log(`tab drag renderer: ${renderer}`);
  for (const [label] of SIZES) {
    expect(out[`${label} dropped at the end`], "the drag reordered the tabs").toBe(1);
    expect(out[`${label} the dragged tab is active`], "dragging a tab selects it").toBe(1);
    // Measured on the HD 520 (p45-scratch, 2026-09-30, two runs): 22-24 ms around the tick that starts the drag (it
    // selects the tab, so a frame of the other 24 or 100 MP document follows) and 18-33 ms over the later ticks.
    // 4b-1's gap budget for the first and its drag-tick budget for the rest.
    expect(out[`${label} longest gap around the tick that starts the drag, ms`]).toBeLessThan(100);
    expect(out[`${label} longest gap over the later ticks, ms`]).toBeLessThan(50);
  }
});

test("a resize-handle drag with snapping at 24 and 100 MP: the tick (snapping, store and frame) and the release", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  let renderer = "";
  for (const [label, w, h] of SIZES) {
    renderer = await ready(page);
    const r = await page.evaluate(async ([w, h]) => {
      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const s = () => api.store.getState();
      const result: Record<string, number> = {};
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const layer = api.engine.state(doc).layers[0].id;
      // The layer at half size in the top-left quarter; its right-middle handle is dragged toward the
      // canvas's centre line, which it snaps to on the last tick.
      const t = api.engine.state(doc).layers[0].transform;
      api.engine.execute(doc, { type: "SetLayerTransform", id: layer, transform: { ...t, origin: [w * 0.1, h * 0.1], size: [w * 0.25, h * 0.5] } });
      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
      s().openDocument(doc); s().setTool("move");
      await settle(); frame();
      const el = (document.querySelector('[data-testid="canvas-view"] canvas') as HTMLElement).parentElement!;
      const at = (x: number, y: number) => {
        const vp = s().viewports[doc]; const p = vp.viewPoint({ x, y }, { width: w, height: h }); const rect = el.getBoundingClientRect();
        return { clientX: rect.left + p.x, clientY: rect.top + p.y };
      };
      const fire = (type: string, x: number, y: number) => el.dispatchEvent(new PointerEvent(type, { ...at(x, y), pointerId: 1, button: 0, buttons: type === "pointerup" ? 0 : 1, bubbles: true }));
      // The handle sits at (0.35 w, 0.35 h). A synthetic pointer cannot be captured, so the press's
      // setPointerCapture throws after the session starts; the page logs it and the drag goes on.
      const drag = async (ticks: number) => {
        const x0 = w * 0.35, y = h * 0.35, x1 = w * 0.5 - 2 / s().viewports[doc].pointsPerPixel;
        fire("pointerdown", x0, y);
        const times: number[] = [];
        for (let i = 1; i <= ticks; i++) {
          const t0 = performance.now();
          fire("pointermove", x0 + (x1 - x0) * i / ticks, y);
          frame();
          times.push(performance.now() - t0);
        }
        const draft = s().transformEdit?.draft;
        const snapped = draft ? Math.abs(draft.origin[0] + draft.size[0] - w * 0.5) < 1e-6 : false;
        const t0 = performance.now();
        fire("pointerup", x1, y); frame();
        const release = performance.now() - t0;
        await settle();
        return { times, snapped, release };
      };
      // Cold (LL-074): one drag first, logged; then the timed one.
      const cold = await drag(4);
      result["cold first tick ms"] = Math.round(cold.times[0]);
      api.engine.undo(doc); s().refresh(doc); frame(); await settle();
      const timed = await drag(10);
      result["tick (snapping, store and frame), worst ms"] = Math.round(Math.max(...timed.times));
      result["release (commit and frame) ms"] = Math.round(timed.release);
      result["the right edge snapped to the centre line"] = timed.snapped ? 1 : 0;
      result["undo depth"] = api.engine.state(doc).undoDepth;
      s().closeDocument(doc);
      return result;
    }, [w, h] as [number, number]);
    for (const [k, v] of Object.entries(r)) out[`${label} ${k}`] = v;
  }
  console.log(`resize drag (release wasm, Edge): ${JSON.stringify(out)}`);
  console.log(`resize drag renderer: ${renderer}`);
  for (const [label] of SIZES) {
    expect(out[`${label} the right edge snapped to the centre line`], "the drag snapped").toBe(1);
    // Measured on the HD 520 (p45-scratch, 2026-09-30, two release runs): ticks 5-7 ms and releases 4-5 ms at both sizes
    // (a transform moves no pixels until it is applied). 4b-1's drag-tick and release budgets.
    expect(out[`${label} tick (snapping, store and frame), worst ms`]).toBeLessThan(50);
    expect(out[`${label} release (commit and frame) ms`]).toBeLessThan(150);
  }
});

test("a typed W at 24 and 100 MP: Enter in the Move bar (the transform applied, one step) and the frame after", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  let renderer = "";
  for (const [label, w, h] of SIZES) {
    renderer = await ready(page);
    const r = await page.evaluate(async ([w, h]) => {
      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      const s = () => api.store.getState();
      const result: Record<string, number> = {};
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const layer = api.engine.state(doc).layers[0].id;
      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
      s().openDocument(doc); s().setTool("move");
      await settle(); frame();
      // The W field as the user types into it: the value set the way React sees typing, then Enter.
      const type = async (value: number) => {
        const input = document.querySelector('input[aria-label="W"]') as HTMLInputElement;
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, String(value));
        input.dispatchEvent(new Event("input", { bubbles: true }));
        await settle();
        const t0 = performance.now();
        input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
        const step = performance.now() - t0;
        const after = frame();
        await settle();
        return [Math.round(step), Math.round(after)];
      };
      [result["cold step ms"], result["cold frame after ms"]] = await type(Math.round(w * 0.9));
      let step = 0, after = 0;
      for (const f of [0.8, 0.7, 0.6]) { const [a, b] = await type(Math.round(w * f)); step = Math.max(step, a); after = Math.max(after, b); }
      result["Enter (store, engine), worst of 3 ms"] = step;
      result["frame after, worst of 3 ms"] = after;
      const t = api.engine.state(doc).layers[0].transform;
      result["width x 1000 / canvas"] = Math.round(t.size[0] / w * 1000);
      result["height x 1000 / canvas"] = Math.round(t.size[1] / h * 1000);
      s().closeDocument(doc);
      return result;
    }, [w, h] as [number, number]);
    for (const [k, v] of Object.entries(r)) out[`${label} ${k}`] = v;
  }
  console.log(`typed W (release wasm, Edge): ${JSON.stringify(out)}`);
  console.log(`typed W renderer: ${renderer}`);
  for (const [label] of SIZES) {
    // The last value typed was 0.6 of the width; the lock (on) scaled the height with it.
    expect(out[`${label} width x 1000 / canvas`]).toBe(600);
    expect(out[`${label} height x 1000 / canvas`]).toBe(600);
    // Measured on the HD 520 (p45-scratch, 2026-09-30, two runs): Enter 1-2 ms and the frame after 4-15 ms at both sizes
    // (a transform moves no pixels). A step inside a frame and 4b-1's frame budget.
    expect(out[`${label} Enter (store, engine), worst of 3 ms`]).toBeLessThan(16);
    expect(out[`${label} frame after, worst of 3 ms`]).toBeLessThan(33);
  }
});
