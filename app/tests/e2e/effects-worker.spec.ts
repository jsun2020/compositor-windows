import { test, expect, type Page } from "@playwright/test";
import { softBlobPngBase64, solidPngBase64 } from "./helpers";

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
  const id = s.documents[s.activeId].layers[0].id;
  return { outside, full: api.engine.hasEffectsImage(s.activeId, id, null) as boolean,
    drawn: String(api.renderer.textureKey(s.activeId, id) ?? "").startsWith("fx:") };
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
  // Full size allowed again: the worker's image goes into the engine's cache and the GPU draws it
  // (a frame after it is kept: effects-images.ts holds the reduced image until then).
  await page.evaluate(() => { const api = (window as any).__compositor; api.effectsLimits.full = 24_000_000; api.store.getState().invalidate(); });
  await expect.poll(async () => { const l = await look(page, "full"); return l.full && l.drawn; }, { timeout: 15_000 }).toBe(true);
  expect((await look(page, "full")).outside).toBeGreaterThan(50);
  expect(await worst(page, "full", "cpu"), "the full-size image is the CPU's").toBeLessThanOrEqual(2);
  expect(await worst(page, "reduced", "full"), "the reduced image was a reduced one").toBeGreaterThan(2);
});

test("fix round 1, issues 1 and 4: the engine's cache evicting a large layer's full image never disturbs its texture or restarts the worker, and hiding then showing the layer never stretches a stale placement", async ({ page }) => {
  test.setTimeout(60_000);
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const blob = await page.evaluate(softBlobPngBase64);
  const tiny = await page.evaluate(solidPngBase64, { width: 2, height: 2, color: "#448899" });

  const result = await page.evaluate(async ({ blob, tiny, doc, styled, effects }) => {
    const api = (window as any).__compositor;
    // A padded image over `sync` but a longer side well under `reduced` (the default 1536): the
    // full-size job is asked for straight away, with no reduced stage at all (issue 4's note: some
    // large layers never get a reduced fallback), so the only thing standing between "plain" and a
    // stretched stale placement, once the engine's cache lets the full image go, is `shown` staying
    // in step with what the texture actually holds.
    api.effectsLimits.sync = 500;
    const manifest = { format: "com.compositor.project", version: 9, colorSpace: "sRGB", documentID: doc, width: 96, height: 73,
      layers: [{ id: styled, name: "Styled", isVisible: true, imageFile: `${styled}.png`, effects,
        transform: { origin: [28, 24], size: [40, 24], rotation: 0, flipX: false, flipY: false, sampling: "High quality" } }] };
    const images = [{ name: `${styled}.png`, bytes: Uint8Array.from(atob(blob), (c: string) => c.charCodeAt(0)) }];
    const id = api.engine.openPackage({ manifest: JSON.stringify(manifest), images }, null);
    api.setCheckerboard(false);
    api.store.getState().openDocument(id);
    await api.setZoom(1);

    // Watch every job channel asked from here, so "no new job starts" can be checked precisely.
    const jobs = api.store.getState().jobs;
    const askedChannels: string[] = [];
    const realRun = jobs.run.bind(jobs);
    jobs.run = (channel: string, request: unknown) => { askedChannels.push(channel); return realRun(channel, request); };

    const render = () => { const s = api.store.getState(); api.renderer.render(api.engine, s.documents[s.activeId], s.viewports[s.activeId], window.devicePixelRatio || 1, { checkerboard: false }, null); };
    const textureKey = () => String(api.renderer.textureKey(id, styled) ?? "");
    const outside = () => {
      render();
      const gpu = api.readDocumentPixels() as Uint8Array;
      let n = 0;
      for (let y = 0; y < 73; y++) for (let x = 0; x < 96; x++) if ((x < 28 || x >= 68 || y < 24 || y >= 48) && gpu[(y * 96 + x) * 4 + 3] > 0) n++;
      return n;
    };
    const waitFor = (done: () => boolean, limit: number) => new Promise<boolean>((resolve) => {
      const start = performance.now();
      const poll = () => { render(); if (done()) resolve(true); else if (performance.now() - start > limit) resolve(false); else setTimeout(poll, 20); };
      poll();
    });

    const landed = await waitFor(() => api.engine.hasEffectsImage(id, styled, null) && textureKey().startsWith("fx:"), 15_000);

    // Fill the engine's effects cache (8 entries, `EFFECTS_CACHE_ENTRIES`) with 8 unrelated small
    // styled layers, made and cached synchronously (`engine.drawPixels`, the same UI-thread path a
    // small effects layer always takes, bypassing the worker entirely), so the ninth entry -- this
    // layer's own -- is the one the LRU trim drops.
    const helperDoc = "0B6C6B1E-4F1B-4B4E-9E0A-EFEFEFEFEF20";
    const helperLayers = Array.from({ length: 8 }, (_, i) => `0B6C6B1E-4F1B-4B4E-9E0A-EFEFEFEFEF3${i}`);
    const hManifest = {
      format: "com.compositor.project", version: 9, colorSpace: "sRGB", documentID: helperDoc, width: 4, height: 4,
      // A layer's imageFile must be exactly "<its own id>.png" (Manifest::validate); each layer gets
      // its own copy of the tiny image, so each still decodes to its own distinct pixel buffer.
      layers: helperLayers.map((hid, i) => ({
        id: hid, name: `H${i}`, isVisible: true, imageFile: `${hid}.png`,
        effects: { stroke: { blue: 0.1, green: 0.1, inside: false, opacity: 0.5, red: 0.1, size: 1 } },
        transform: { origin: [0, 0], size: [2, 2], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
      })),
    };
    const hImages = helperLayers.map((hid) => ({ name: `${hid}.png`, bytes: Uint8Array.from(atob(tiny), (c: string) => c.charCodeAt(0)) }));
    const hDoc = api.engine.openPackage({ manifest: JSON.stringify(hManifest), images: hImages }, null);
    for (const hid of helperLayers) api.engine.drawPixels(hDoc, hid, 0, null, () => null);

    const evicted = !api.engine.hasEffectsImage(id, styled, null);
    const keyRightAfterEviction = textureKey();
    // Several more frames: the fast path (fix round 1, issue 1) never re-asks the worker or the
    // engine's cache once the texture already holds this exact image.
    const askedBefore = askedChannels.length;
    for (let i = 0; i < 5; i++) { render(); await new Promise((r) => requestAnimationFrame(r)); }
    const newJobsForThisLayer = askedChannels.slice(askedBefore).filter((c) => c === `fx:${id}:${styled}`);
    const keyAfterMoreFrames = textureKey();

    // Hide, then show again: the cache stays evicted and this layer never had a reduced fallback
    // (its longer side is under `reduced`), so the very first frame after showing it again has
    // nothing to draw but the plain layer -- which must never be stretched over the old inset
    // (fix round 1, issue 4).
    api.engine.execute(id, { type: "SetLayerVisible", id: styled, visible: false });
    api.store.getState().refresh(id);
    render();
    const keyWhileHidden = textureKey();
    api.engine.execute(id, { type: "SetLayerVisible", id: styled, visible: true });
    api.store.getState().refresh(id);
    const outsideRightAfterShowing = outside();

    return { landed, evicted, keyRightAfterEviction, newJobsForThisLayer, keyAfterMoreFrames, keyWhileHidden, outsideRightAfterShowing };
  }, { blob, tiny, doc: DOC, styled: STYLED, effects: EFFECTS });

  expect(result.landed, "the full image landed before the cache was filled").toBe(true);
  expect(result.evicted, "the engine's own cache no longer has it").toBe(true);
  expect(result.keyRightAfterEviction.startsWith("fx:"), "the texture still holds the full image right after eviction").toBe(true);
  expect(result.newJobsForThisLayer, "no new job was asked for this layer: its texture already held the full image").toEqual([]);
  expect(result.keyAfterMoreFrames.startsWith("fx:"), "and still does after several more frames").toBe(true);
  expect(result.keyWhileHidden.startsWith("px:"), "hidden: the layer's own pixels, not the old padded image").toBe(true);
  expect(result.outsideRightAfterShowing, "shown again: no stretched placement of whatever the texture now holds").toBe(0);
});
