import { describe, expect, it } from "vitest";
import { MaskTextures } from "../../src/canvas/gl/mask-textures";

/** Just enough of WebGL2 for MaskTextures, recording every texImage2D upload. */
function stubGl() {
  const uploads: { width: number; height: number; length: number }[] = [];
  let next = 1;
  const gl = {
    TEXTURE_2D: 1, R8: 2, RED: 3, UNSIGNED_BYTE: 4, UNPACK_ALIGNMENT: 5, UNPACK_ROW_LENGTH: 6,
    UNPACK_SKIP_PIXELS: 7, UNPACK_SKIP_ROWS: 8, TEXTURE_WRAP_S: 9, TEXTURE_WRAP_T: 10,
    CLAMP_TO_EDGE: 11, TEXTURE_MIN_FILTER: 12, TEXTURE_MAG_FILTER: 13, NEAREST: 14, LINEAR: 15,
    createTexture: () => ({ id: next++ }) as unknown as WebGLTexture,
    bindTexture: () => {},
    pixelStorei: () => {},
    texParameteri: () => {},
    deleteTexture: () => {},
    texImage2D: (_t: number, _l: number, _i: number, width: number, height: number, _b: number, _f: number, _ty: number, pixels: Uint8Array) => {
      uploads.push({ width, height, length: pixels.length });
    },
  } as unknown as WebGL2RenderingContext;
  return { gl, uploads };
}

describe("MaskTextures.get", () => {
  it("returns undefined on a miss and uploads nothing", () => {
    const { gl, uploads } = stubGl();
    const masks = new MaskTextures(gl);
    expect(masks.get("D", "A")).toBeUndefined();
    expect(uploads).toEqual([]);
  });

  it("returns the cached texture after a sync, without uploading again", () => {
    const { gl, uploads } = stubGl();
    const masks = new MaskTextures(gl);
    const tex = masks.sync("D", "A", 1, 4, 4, new Uint8Array(16));
    expect(uploads).toEqual([{ width: 4, height: 4, length: 16 }]);
    expect(masks.get("D", "A")).toBe(tex);
    expect(uploads.length).toBe(1);
  });

  it("is keyed by document as well as layer", () => {
    const { gl } = stubGl();
    const masks = new MaskTextures(gl);
    masks.sync("D", "A", 1, 4, 4, new Uint8Array(16));
    expect(masks.get("OTHER", "A")).toBeUndefined();
  });

  // Why `get` exists: the coverage pass used to re-enter `sync` with an empty buffer to fetch a
  // cached texture. That is only safe on a hit. On a miss it uploads an undersized buffer,
  // which a real context rejects with INVALID_OPERATION and which leaves a garbage mask cached.
  it("re-entering sync on a miss would upload an undersized buffer", () => {
    const { gl, uploads } = stubGl();
    const masks = new MaskTextures(gl);
    masks.sync("D", "A", 1, 4, 4, new Uint8Array(0));
    expect(uploads).toEqual([{ width: 4, height: 4, length: 0 }]);
  });
});
