import { describe, expect, it } from "vitest";
import { LayerTextures, levelRect, sizeAtLevel } from "../../src/canvas/layer-textures";
import { MaskTextures, syncMask } from "../../src/canvas/gl/mask-textures";
import type { PixelRect } from "../../src/engine/types";

/** Just enough of WebGL2 for the texture caches, recording every upload with the pixel-store state.
 * `pixelsLengths` records the byte length of whatever buffer each call actually received, parallel
 * to `calls` -- so a test can tell a live upload from one fed a detached (zero-length) view, which
 * the pixel-store fields alone (`calls`) cannot show. */
function stubGl() {
  const calls: { kind: "image" | "sub"; x: number; y: number; width: number; height: number; skipX: number; skipY: number; rowLength: number }[] = [];
  const pixelsLengths: number[] = [];
  const store: Record<number, number> = {};
  let next = 1;
  const gl = {
    TEXTURE_2D: 1, R8: 2, RED: 3, UNSIGNED_BYTE: 4, UNPACK_ALIGNMENT: 5, UNPACK_ROW_LENGTH: 6,
    UNPACK_SKIP_PIXELS: 7, UNPACK_SKIP_ROWS: 8, TEXTURE_WRAP_S: 9, TEXTURE_WRAP_T: 10,
    CLAMP_TO_EDGE: 11, TEXTURE_MIN_FILTER: 12, TEXTURE_MAG_FILTER: 13, NEAREST: 14, LINEAR: 15, RGBA8: 16, RGBA: 17, UNPACK_PREMULTIPLY_ALPHA_WEBGL: 18,
    createTexture: () => ({ id: next++ }) as unknown as WebGLTexture,
    bindTexture: () => {}, texParameteri: () => {}, deleteTexture: () => {},
    pixelStorei: (k: number, v: number) => { store[k] = v; },
    texImage2D: (_t: number, _l: number, _i: number, width: number, height: number, _b: number, _f: number, _ty: number, pixels: ArrayBufferView) => {
      calls.push({ kind: "image", x: 0, y: 0, width, height, skipX: store[7] ?? 0, skipY: store[8] ?? 0, rowLength: store[6] ?? 0 });
      pixelsLengths.push(pixels.byteLength);
    },
    texSubImage2D: (_t: number, _l: number, x: number, y: number, width: number, height: number, _f: number, _ty: number, pixels: ArrayBufferView) => {
      calls.push({ kind: "sub", x, y, width, height, skipX: store[7] ?? 0, skipY: store[8] ?? 0, rowLength: store[6] ?? 0 });
      pixelsLengths.push(pixels.byteLength);
    },
  } as unknown as WebGL2RenderingContext;
  return { gl, calls, pixelsLengths };
}

/** Independently of `levelRect`: the pixels after `level` halvings whose block of source pixels,
 * [x * 2^level, (x + 1) * 2^level) on each axis, meets `rect`. */
function reached(rect: PixelRect, level: number, width: number, height: number): PixelRect {
  const size = sizeAtLevel(width, height, level), f = 2 ** level;
  const xs: number[] = [], ys: number[] = [];
  for (let x = 0; x < size.width; x++) if (x * f < rect.x + rect.width && (x + 1) * f > rect.x) xs.push(x);
  for (let y = 0; y < size.height; y++) if (y * f < rect.y + rect.height && (y + 1) * f > rect.y) ys.push(y);
  if (xs.length === 0 || ys.length === 0) return { x: 0, y: 0, width: 0, height: 0 };
  return { x: xs[0], y: ys[0], width: xs.length, height: ys.length };
}

describe("levelRect", () => {
  it("takes every reduced pixel whose block the change reaches, at every level", () => {
    for (const rect of [{ x: 5, y: 7, width: 9, height: 4 }, { x: 30, y: 20, width: 7, height: 9 }, { x: 0, y: 0, width: 1, height: 1 }, { x: 36, y: 28, width: 1, height: 1 }]) {
      for (const level of [0, 1, 2, 3]) expect(levelRect(rect, level, 37, 29), `${JSON.stringify(rect)} at ${level}`).toEqual(reached(rect, level, 37, 29));
    }
  });
  it("is empty for an empty change", () => {
    expect(levelRect({ x: 4, y: 4, width: 0, height: 3 }, 2, 37, 29)).toEqual({ x: 0, y: 0, width: 0, height: 0 });
  });
});

describe("LayerTextures.update", () => {
  it("uploads the changed rectangle's bytes into each chunk it meets", () => {
    const { gl, calls } = stubGl();
    const textures = new LayerTextures(gl);
    // 5000 x 3000: chunks start at x 0, 2048, 4096 and y 0, 2048.
    textures.sync("D", "A", "px:1", false, new Uint8Array(4), 0, { width: 5000, height: 3000 }, 1);
    expect(calls.filter((c) => c.kind === "image").length).toBe(6);
    calls.length = 0;
    textures.update("D", "A", "px:2", 2, { x: 2000, y: 1000, width: 100, height: 1100 }, new Uint8Array(4));
    // Across the x = 2048 edge and the y = 2048 edge: four pieces, each placed in its own chunk and
    // read from its own part of the 100 x 1100 region.
    expect(calls).toEqual([
      { kind: "sub", x: 2000, y: 1000, width: 48, height: 1048, skipX: 0, skipY: 0, rowLength: 100 },
      { kind: "sub", x: 0, y: 1000, width: 52, height: 1048, skipX: 48, skipY: 0, rowLength: 100 },
      { kind: "sub", x: 2000, y: 0, width: 48, height: 52, skipX: 0, skipY: 1048, rowLength: 100 },
      { kind: "sub", x: 0, y: 0, width: 52, height: 52, skipX: 48, skipY: 1048, rowLength: 100 },
    ]);
    const t = textures.get("D", "A")!;
    expect([t.key, t.revision]).toEqual(["px:2", 2]);
  });
});

describe("MaskTextures partial uploads", () => {
  it("takes the changed rectangle when the engine knows it, and the whole mask otherwise", () => {
    const { gl, calls } = stubGl();
    const masks = new MaskTextures(gl);
    masks.sync("D", "A", 1, 60, 40, new Uint8Array(2400));
    expect(masks.cachedRevision("D", "A", 60, 40)).toBe(1);
    masks.sync("D", "A", 2, 60, 40, new Uint8Array(2400), { x: 3, y: 5, width: 7, height: 9 });
    expect(calls.at(-1)).toEqual({ kind: "sub", x: 3, y: 5, width: 7, height: 9, skipX: 3, skipY: 5, rowLength: 60 });
    expect(masks.has("D", "A", 2)).toBe(true);
    masks.sync("D", "A", 3, 60, 40, new Uint8Array(2400), null);
    expect(calls.at(-1)?.kind).toBe("image");
    // Another size is always uploaded whole: cachedRevision returns undefined for it, so a caller
    // never asks for a delta against it.
    expect(masks.cachedRevision("D", "A", 30, 40)).toBeUndefined();
    masks.sync("D", "A", 4, 30, 40, new Uint8Array(1200), undefined);
    expect(calls.at(-1)).toMatchObject({ kind: "image", width: 30, height: 40 });
  });
});

describe("syncMask", () => {
  it("asks maskDelta before maskPixels, so a wasm call that grows memory in between cannot hand a detached view to the GPU", () => {
    const { gl, calls, pixelsLengths } = stubGl();
    const masks = new MaskTextures(gl);
    masks.sync("D", "A", 1, 60, 40, new Uint8Array(2400));

    // A real WebAssembly.Memory stands in for the engine's linear memory, exactly what
    // EngineClient.maskPixels views: growing it detaches any view already taken on its old buffer
    // (WebAssembly.Memory.grow does this per spec), just as mask_delta's own string marshalling and
    // Vec<f64> allocation can do to a view EngineClient.maskPixels already handed out.
    const mem = new WebAssembly.Memory({ initial: 1 });
    const maskPixels = () => new Uint8Array(mem.buffer, 0, 2400).fill(42);
    const asked: number[] = [];
    const maskDelta = (from: number) => { asked.push(from); mem.grow(1); return { x: 3, y: 5, width: 7, height: 9 }; };

    // A view taken before the delta call is detached by it, in production exactly what the pre-fix
    // `visit` (maskPixels, then a delta callback inside sync) would have hung on to.
    const staleIfReadFirst = maskPixels();
    maskDelta(1);
    expect(staleIfReadFirst.length, "growing memory really does detach the earlier view").toBe(0);
    asked.length = 0;

    // syncMask (the production ordering): delta before pixels, so what reaches sync is never stale.
    syncMask(masks, "D", "A", 2, 60, 40, maskDelta, maskPixels);
    expect(asked).toEqual([1]);
    expect(calls.at(-1)).toEqual({ kind: "sub", x: 3, y: 5, width: 7, height: 9, skipX: 3, skipY: 5, rowLength: 60 });
    // The buffer that actually reached texSubImage2D is the fresh one (2400 bytes), never the
    // detached one growing memory during the delta call would have left behind.
    expect(pixelsLengths.at(-1)).toBe(2400);
    expect(masks.has("D", "A", 2)).toBe(true);
  });
  it("asks nothing and uploads whole when nothing is cached at this size", () => {
    const { gl, calls } = stubGl();
    const masks = new MaskTextures(gl);
    const asked: number[] = [];
    syncMask(masks, "D", "A", 1, 60, 40, (from) => { asked.push(from); return { x: 0, y: 0, width: 1, height: 1 }; }, () => new Uint8Array(2400));
    expect(asked).toEqual([]);
    expect(calls.at(-1)).toMatchObject({ kind: "image", width: 60, height: 40 });
  });
});
