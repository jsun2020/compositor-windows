import { test, expect, type Page } from "@playwright/test";
import type { PointTuple, WarpMode } from "../../src/engine/types";

async function ready(page: Page) {
  await page.goto("/"); await expect(page.getByTestId("engine-ready")).toBeVisible();
}

// Run the actual dedicated WebGL2 passes against the existing WASM reference
// command. Returning aggregate differences avoids giant failure attachments.
async function compare(page: Page, mode: WarpMode, width: number, height: number, points: PointTuple[],
  diameter: number, hardness: number, strength: number, stripe = false) {
  return page.evaluate(async args => {
    const path = "/src/canvas/gpu-warp.ts";
    const { GpuWarpStroke }: typeof import("../../src/canvas/gpu-warp") = await import(/* @vite-ignore */ path);
    const { mode, width, height, points, diameter, hardness, strength, stripe } = args;
    const source = new Uint8Array(width * height * 4);
    for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
      const i = (y * width + x) * 4, alpha = stripe ? 255 : (x * 17 + y * 29) % 256;
      source[i] = stripe ? (x === 0 ? 255 : 0) : Math.floor(alpha * ((x * 11 + y * 7) % 256) / 255);
      source[i + 1] = stripe ? source[i] : Math.floor(alpha / 2); source[i + 2] = stripe ? source[i] : Math.floor(alpha / 3); source[i + 3] = alpha;
    }
    const canvas = document.createElement("canvas"), gl = canvas.getContext("webgl2", { antialias: false })!;
    const lose = gl.getExtension("WEBGL_lose_context");
    const stroke = new GpuWarpStroke(gl, width, height, source, { mode, diameter, hardness, strength }, { sourceTileSide: 8, workTileSide: 8 });
    try {
      const initial = stroke.diagnostics(), scheduled = points.map(p => stroke.append(p).length);
      const after = source.slice();
      for (const tile of stroke.readTiles()) for (let row = 0; row < tile.height; row++)
        after.set(tile.pixels.subarray(row * tile.width * 4, (row + 1) * tile.width * 4), ((tile.y + row) * width + tile.x) * 4);
      const api = (window as any).__compositor;
      const doc = api.engine.newDocument(width, height, false);
      api.engine.pastePixels(doc, width, height, source.slice().buffer, [0, 0]);
      const layer = api.engine.state(doc).activeLayerId, before = api.engine.state(doc).undoDepth;
      api.engine.execute(doc, { type: "WarpStroke", id: layer, mask: false, warp: { mode, diameter, hardness, strength, points } });
      const expected: Uint8Array = api.engine.layerPixels(doc, layer);
      let maxError = 0, different = 0, changed = 0, premultiplied = true;
      for (let i = 0; i < after.length; i++) { const error = Math.abs(after[i] - expected[i]); maxError = Math.max(maxError, error); if (error) different++; if (after[i] !== source[i]) changed++; }
      for (let i = 0; i < after.length; i += 4) if (after[i] > after[i + 3] || after[i + 1] > after[i + 3] || after[i + 2] > after[i + 3]) premultiplied = false;
      const diagnostic = stroke.diagnostics(); stroke.dispose(); stroke.dispose();
      return { initial, diagnostic, released: stroke.diagnostics(), scheduled, maxError, different, changed, premultiplied,
        programUnbound: gl.getParameter(gl.CURRENT_PROGRAM) === null,
        red: Array.from(after.filter((_, i) => i % 4 === 0)), historyDelta: api.engine.state(doc).undoDepth - before };
    } finally { stroke.dispose(); lose?.loseContext(); }
  }, { mode, width, height, points, diameter, hardness, strength, stripe });
}

for (const mode of ["Smudge", "Liquify"] as const) {
  test(`${mode} GPU: analytic stroke, exact WASM bytes and disposal`, async ({ page }) => {
    await ready(page);
    const r = await compare(page, mode, 7, 1, [[0, 0], [6, 0]], 2, 0.5, 0.5, true);
    expect(r.maxError).toBe(0); expect(r.different).toBe(0); expect(r.historyDelta).toBe(1);
    // At diameter 2 the adjacent offset has zero falloff: Liquify samples the
    // immutable stripe, while Smudge carries the fractional fading trail.
    expect(r.red).toEqual(mode === "Smudge" ? [255, 128, 64, 32, 16, 8, 4] : [255, 128, 0, 0, 0, 0, 0]);
    expect(r.scheduled).toEqual([0, 6]);
    expect(r.diagnostic.uploadedBytes).toBe(r.initial.uploadedBytes);
    expect(r.programUnbound).toBe(true);
    expect(r.released).toMatchObject({ disposed: true, allocatedBytes: 0, liveTextures: 0, liveFramebuffers: 0, livePrograms: 0, tileCount: 0 });
  });
  test(`${mode} GPU: mixed alpha, fractional tip, repeated turns across source/work tile boundaries`, async ({ page }) => {
    await ready(page);
    const r = await compare(page, mode, 37, 23, [[7, 8], [18, 12], [32, 21], [8, 3]], 9.5, 0.25, 0.33);
    // Driver float contraction can affect a last-bit rounding decision. Keep
    // the analytic exact test separate; never claim exact Metal byte parity.
    expect(r.maxError).toBeLessThanOrEqual(1); expect(r.changed).toBeGreaterThan(100);
    expect(r.premultiplied).toBe(true); expect(r.diagnostic.tileCount).toBeGreaterThan(4);
    expect(r.diagnostic.uploadedBytes).toBe(37 * 23 * 4); expect(r.historyDelta).toBe(1);
    test.info().annotations.push({ type: "GPU/WASM delta", description: `${r.different} channels differed, maximum ${r.maxError}/255` });
  });
  test(`${mode} GPU: pickup/sub-spacing do not allocate edited tiles or change history`, async ({ page }) => {
    await ready(page);
    const r = await compare(page, mode, 17, 9, [[3, 4], [3.5, 4]], 2, 0.5, 0.5);
    expect(r.scheduled).toEqual([0, 0]); expect(r.diagnostic.tileCount).toBe(0);
    expect(r.maxError).toBe(0); expect(r.changed).toBe(0); expect(r.historyDelta).toBe(0);
  });
  test(`${mode} GPU: negative half pickup, transparent replacement, clamped thin axis`, async ({ page }) => {
    await ready(page);
    const r = await compare(page, mode, 1, 9, [[-0.5, 2], [0.5, 7], [0, 1]], 3, 0, 0.7);
    expect(r.maxError).toBeLessThanOrEqual(1); expect(r.premultiplied).toBe(true);
    expect(r.diagnostic.uploadedBytes).toBe(36);
  });
}

test("GPU warp refuses append limits before changing existing tiles, then permits cancellation", async ({ page }) => {
  await ready(page);
  const r = await page.evaluate(async () => {
    const path = "/src/canvas/gpu-warp.ts", { GpuWarpStroke }: typeof import("../../src/canvas/gpu-warp") = await import(/* @vite-ignore */ path);
    const gl = document.createElement("canvas").getContext("webgl2")!, source = new Uint8Array(32 * 16 * 4);
    for (let i = 0; i < source.length; i += 4) { source[i] = i % 251; source[i + 3] = 255; }
    const stroke = new GpuWarpStroke(gl, 32, 16, source, { mode: "Liquify" as const, diameter: 2, hardness: 0.5, strength: 0.5 },
      { sourceTileSide: 8, workTileSide: 8, maxBytes: 32 * 16 * 4 + 9 * 9 * 8 + 8 * 8 * 12 });
    stroke.append([3, 3]); stroke.append([4, 3]);
    const before = stroke.readTiles().map(t => Array.from(t.pixels)), diagnostic = stroke.diagnostics(), errors: string[] = [];
    for (const point of [[20, 3], [Infinity, 0], [10000, 3]] as PointTuple[]) try { stroke.append(point); } catch (e) { errors.push(String(e)); }
    const after = stroke.readTiles().map(t => Array.from(t.pixels)), refused = stroke.diagnostics();
    // Rejected moves did not advance the previous anchor.
    const resumed = stroke.append([5, 3]); stroke.dispose();
    let afterDispose = ""; try { stroke.append([6, 3]); } catch (e) { afterDispose = String(e); }
    gl.getExtension("WEBGL_lose_context")?.loseContext();
    return { before, after, diagnostic, refused, errors, resumed, afterDispose, released: stroke.diagnostics() };
  });
  expect(r.after).toEqual(r.before); expect(r.refused).toEqual(r.diagnostic);
  expect(r.errors[0]).toContain("GPU memory budget"); expect(r.errors[1]).toContain("Invalid warp point"); expect(r.errors[2]).toContain("too long");
  expect(r.resumed).toEqual([[5, 3]]); expect(r.afterDispose).toContain("disposed");
  expect(r.released).toMatchObject({ liveTextures: 0, liveFramebuffers: 0, livePrograms: 0 });
});

test("GPU warp capability and allocation refusals do not leak resources", async ({ page }) => {
  await ready(page);
  const r = await page.evaluate(async () => {
    const path = "/src/canvas/gpu-warp.ts", { GpuWarpStroke }: typeof import("../../src/canvas/gpu-warp") = await import(/* @vite-ignore */ path);
    const gl = document.createElement("canvas").getContext("webgl2")!;
    const settings = { mode: "Liquify" as const, diameter: 2, hardness: 0.5, strength: 0.5 }, errors: string[] = [];
    const original = gl.getExtension.bind(gl); let creates = 0;
    const create = gl.createTexture.bind(gl); gl.createTexture = () => { creates++; return create(); };
    gl.getExtension = (name: string) => name === "EXT_color_buffer_float" ? null : original(name);
    try { new GpuWarpStroke(gl, 1, 1, new Uint8Array(4), settings); } catch (e) { errors.push(String(e)); }
    gl.getExtension = original;
    try { new GpuWarpStroke(gl, 1, 1, new Uint8Array(4), settings, { maxBytes: 1 }); } catch (e) { errors.push(String(e)); }
    try { new GpuWarpStroke(gl, 1, 1, new Uint8Array([255, 0, 0, 0]), settings); } catch (e) { errors.push(String(e)); }
    const parameter = gl.getParameter.bind(gl);
    gl.getParameter = (name: number) => name === gl.MAX_ARRAY_TEXTURE_LAYERS ? 1 : parameter(name);
    try { new GpuWarpStroke(gl, 17, 9, new Uint8Array(17 * 9 * 4), settings, { sourceTileSide: 8 }); } catch (e) { errors.push(String(e)); }
    gl.getParameter = parameter;
    // Simulate a float target becoming unsupported after the extension check.
    let textures = 0, fbos = 0, programs = 0;
    const deletedTexture = gl.deleteTexture.bind(gl), createFbo = gl.createFramebuffer.bind(gl), deletedFbo = gl.deleteFramebuffer.bind(gl);
    const createProgram = gl.createProgram.bind(gl), deletedProgram = gl.deleteProgram.bind(gl);
    gl.createTexture = () => { textures++; return create(); }; gl.deleteTexture = t => { if (t) textures--; deletedTexture(t); };
    gl.createFramebuffer = () => { fbos++; return createFbo(); }; gl.deleteFramebuffer = f => { if (f) fbos--; deletedFbo(f); };
    gl.createProgram = () => { programs++; return createProgram(); }; gl.deleteProgram = p => { if (p) programs--; deletedProgram(p); };
    gl.checkFramebufferStatus = () => gl.FRAMEBUFFER_UNSUPPORTED;
    try { new GpuWarpStroke(gl, 1, 1, new Uint8Array(4), settings); } catch (e) { errors.push(String(e)); }
    original("WEBGL_lose_context")?.loseContext();
    return { errors, creates, textures, fbos, programs };
  });
  expect(r.creates).toBe(0); expect(r.errors).toHaveLength(5);
  expect(r.errors[0]).toContain("float render targets"); expect(r.errors[1]).toContain("memory budget");
  expect(r.errors[2]).toContain("premultiplied"); expect(r.errors[3]).toContain("array texture capacity"); expect(r.errors[4]).toContain("framebuffer is incomplete");
  expect(r.textures).toBe(0); expect(r.fbos).toBe(0); expect(r.programs).toBe(0);
});

test("GPU warp context loss discards all resources and refuses further reads/movement", async ({ page }) => {
  await ready(page);
  const r = await page.evaluate(async () => {
    const path = "/src/canvas/gpu-warp.ts", { GpuWarpStroke }: typeof import("../../src/canvas/gpu-warp") = await import(/* @vite-ignore */ path);
    const canvas = document.createElement("canvas"), gl = canvas.getContext("webgl2")!;
    const stroke = new GpuWarpStroke(gl, 16, 16, new Uint8Array(16 * 16 * 4), { mode: "Smudge", diameter: 2, hardness: 0.5, strength: 0.5 });
    stroke.append([3, 3]); stroke.append([6, 3]);
    const event = new Promise<void>(resolve => canvas.addEventListener("webglcontextlost", () => resolve(), { once: true }));
    gl.getExtension("WEBGL_lose_context")!.loseContext(); await event;
    const errors: string[] = []; try { stroke.append([7, 3]); } catch (e) { errors.push(String(e)); }
    try { stroke.readTiles(); } catch (e) { errors.push(String(e)); }
    return { errors, released: stroke.diagnostics() };
  });
  expect(r.errors.every(e => e.includes("context lost"))).toBe(true);
  expect(r.released).toMatchObject({ disposed: true, allocatedBytes: 0, liveTextures: 0, liveFramebuffers: 0, livePrograms: 0 });
});
