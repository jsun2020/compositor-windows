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

test("F1: a Delete inside a selection after a job's result at 24 and 100 MP, at fit and at 40 %: the adopted halving seeded, not made again", async ({ page }) => {
  // `seed_halvings` -> `seed_adopted` (raster.rs): after a whole-layer result is adopted, an edit inside a selection on
  // the UI thread copies the adopted halving (1/4^level of the layer: 6.25 MB at level 3 of 100 MP, 100 MB at level 1)
  // and halves again only the selection's reach. 40 % gives level 1, the dearest copy.
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  let renderer = "";
  for (const [label, w, h] of SIZES) {
    renderer = await ready(page);
    const r = await page.evaluate(async ([w, h]) => {
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
      // Each install's display halving (the fill's result adopted), and `layerPixels` - a halving made on the UI
      // thread - timed inside the frame after the Delete.
      let inFrame = false, halving = 0;
      const displays: number[] = [];
      const install = api.engine.installJob.bind(api.engine);
      api.engine.installJob = (...a: unknown[]) => { displays.push((a[6] as ArrayBuffer | null)?.byteLength ?? 0); return install(...a); };
      const pixels = api.engine.layerPixels.bind(api.engine);
      api.engine.layerPixels = (...a: unknown[]) => { const t = performance.now(); try { return pixels(...a); } finally { if (inFrame) halving += performance.now() - t; } };
      // One round: the whole layer filled through the worker (its result comes back halved to the canvas's level and
      // is adopted), then a 1024 x 1024 marquee cleared on the UI thread, as the Delete key does (`deleteKeyPressed`).
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
        halving = 0; inFrame = true;
        const after = frame();
        inFrame = false;
        const whole = (window as any).__uploads.image as number;
        const cleared = api.engine.composite(doc, { x: x + 512, y: y + 512, width: 1, height: 1 }, 1, 1)[3] === 0 ? 1 : 0;
        await settle();
        return { step, after, halving, whole, adopted, cleared };
      };
      for (const zoom of ["fit", "40 %"]) {
        if (zoom === "40 %") { await api.setZoom(0.4); await settle(); frame(); }
        // Cold (LL-074): the first round at this zoom, logged only.
        const cold = await round(0);
        result[`${zoom}: cold Delete ms`] = Math.round(cold.step);
        result[`${zoom}: cold frame after ms`] = Math.round(cold.after);
        let step = 0, after = 0, halved = 0, whole = 0, adopted = 0, cleared = 0;
        for (let n = 1; n <= 3; n++) {
          const t = await round(n);
          step = Math.max(step, t.step); after = Math.max(after, t.after); halved = Math.max(halved, t.halving);
          whole += t.whole; adopted += t.adopted; cleared += t.cleared;
        }
        result[`${zoom}: Delete (UI thread), worst of 3 ms`] = Math.round(step);
        result[`${zoom}: frame after, worst of 3 ms`] = Math.round(after);
        result[`${zoom}: layerPixels in the frame after, worst ms`] = Math.round(halved);
        result[`${zoom}: whole uploads in the frames after`] = whole;
        result[`${zoom}: rounds whose fill came back halved`] = adopted;
        result[`${zoom}: rounds whose Delete cleared the centre`] = cleared;
        result[`${zoom}: canvas scale x 1000`] = Math.round(s().viewports[doc].pointsPerPixel * (window.devicePixelRatio || 1) * 1000);
      }
      s().closeDocument(doc);
      return result;
    }, [w, h] as [number, number]);
    for (const [k, v] of Object.entries(r)) out[`${label} ${k}`] = v;
  }
  console.log(`F1 Delete after a job (release wasm, Edge): ${JSON.stringify(out)}`);
  console.log(`F1 Delete after a job renderer: ${renderer}`);
  // Not measured on the scratch clone (added by the pre-flight, I2). Budgets: 4b-1's for the frame after a partial edit
  // (33 ms, no whole upload: perf-4b1.spec.ts, ruling M4) and for a key on the UI thread (150 ms) at 24 MP; 400 ms at
  // 100 MP, where the seeding copies up to 100 MB at level 1. A halving made again on the UI thread is what F1 removed.
  for (const [label] of SIZES) for (const zoom of ["fit", "40 %"]) {
    const k = (name: string) => out[`${label} ${zoom}: ${name}`];
    expect(k("rounds whose fill came back halved"), "every fill's result came back halved and was adopted").toBe(3);
    expect(k("rounds whose Delete cleared the centre"), "every Delete ran").toBe(3);
    expect(k("whole uploads in the frames after"), "a partial edit uploads no whole texture").toBe(0);
    expect(k("layerPixels in the frame after, worst ms"), "the adopted halving was seeded, not made again").toBeLessThan(20);
    expect(k("frame after, worst of 3 ms")).toBeLessThan(33);
    expect(k("Delete (UI thread), worst of 3 ms")).toBeLessThan(label === "24 MP" ? 150 : 400);
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
        timed("jobInput");
        timed("layerPixels");
        timed("installJob", (a) => displays.push((a[6] as ArrayBuffer | null)?.byteLength ?? 0), () => {
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
