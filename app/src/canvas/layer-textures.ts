import type { PixelRect } from "../engine/types";

export const CHUNK = 2048;
/** A 30000-pixel side reaches 1 pixel in 15 halvings; mirrors MAX_PREFILTER_LEVEL in the engine. */
export const MAX_PREFILTER_LEVEL = 16;

/** Mac 1.4.5 LayerRenderer.interpolation copies upright pixels when its final
 * prefilter level lands at 1:1. Drawing never changes the saved sampling mode. */
export function pixelCopyAtScale(rotation: number, width: number, drawnWidth: number, outPerDoc: number, level: number, distorted: boolean): boolean {
  return !distorted && rotation % 360 === 0 && width > 0 && Math.abs(drawnWidth * outPerDoc / width * 2 ** level - 1) < 0.001;
}

/**
 * Sharp halvings before the final resample, mirroring `compositor::prefilter_level` in the
 * engine exactly: halve until one output pixel covers at most 2 source pixels, stopping when
 * the raster runs out of pixels. The GL renderer uploads the layer at this level and the CPU
 * compositor prefilters to the same one, so the two agree at every zoom.
 */
export function prefilterLevel(width: number, height: number, sourcePerOutput: number): number {
  let w = width, h = height, factor = sourcePerOutput, level = 0;
  while (factor > 2 && w > 1 && h > 1 && level < MAX_PREFILTER_LEVEL) {
    w = Math.max(1, Math.floor(w / 2)); h = Math.max(1, Math.floor(h / 2)); level++; factor /= 2;
  }
  return level;
}
/** The raster's size after `level` halvings, the same floor-and-clamp the engine applies. */
export function sizeAtLevel(width: number, height: number, level: number): { width: number; height: number } {
  let w = width, h = height;
  for (let i = 0; i < level; i++) { if (w <= 1 || h <= 1) break; w = Math.max(1, Math.floor(w / 2)); h = Math.max(1, Math.floor(h / 2)); }
  return { width: w, height: h };
}

/** The part of the raster after `level` halvings that a changed rectangle of the full raster
 * (`width` x `height`) reaches: each halving takes every pixel whose 2 x 2 block meets it, cut to the
 * smaller size, exactly as the engine's `PixelRect::halved`, so a partial upload at a reduced zoom
 * covers every texel the change moved. */
export function levelRect(rect: PixelRect, level: number, width: number, height: number): PixelRect {
  let r = rect, w = width, h = height;
  for (let i = 0; i < level; i++) {
    if (w <= 1 || h <= 1) break;
    w = Math.max(1, Math.floor(w / 2)); h = Math.max(1, Math.floor(h / 2));
    if (r.width === 0 || r.height === 0) return { x: 0, y: 0, width: 0, height: 0 };
    const x0 = Math.min(Math.floor(r.x / 2), w), y0 = Math.min(Math.floor(r.y / 2), h);
    const x1 = Math.min(Math.ceil((r.x + r.width) / 2), w), y1 = Math.min(Math.ceil((r.y + r.height) / 2), h);
    r = { x: x0, y: y0, width: Math.max(0, x1 - x0), height: Math.max(0, y1 - y0) };
  }
  // A change only in a dropped odd last row or column reaches nothing.
  return r.width === 0 || r.height === 0 ? { x: 0, y: 0, width: 0, height: 0 } : r;
}

export interface Chunk { texture: WebGLTexture; x: number; y: number; width: number; height: number; }
/** `key` names the bytes: the layer's pixels at a revision, or its effects image (GlRenderer.syncTextures).
 * `revision` is the pixels revision uploaded, for asking the engine what changed since (null for an
 * effects image, which is always uploaded whole). */
export interface LayerTexture { key: string; revision: number | null; level: number; width: number; height: number; nearest: boolean; chunks: Chunk[]; }

/** Two sessions opened from the same `.comp` file carry identical layer ids (they come from
 * the manifest), so the cache is keyed by document handle *and* layer id -- otherwise one
 * document's texture can be served to another that happens to share a layer id and revision. */
function key(docId: string, layerId: string): string { return `${docId}:${layerId}`; }

export class LayerTextures {
  private layers = new Map<string, LayerTexture>();
  private serial = 0;
  /** Changes whenever retained GPU content or its sampling changes. */
  get generation(): number { return this.serial; }
  constructor(private readonly gl: WebGL2RenderingContext) {}

  get(docId: string, id: string): LayerTexture | undefined { return this.layers.get(key(docId, id)); }

  /** Whether a call to `sync` would have to upload, i.e. the cache does not already hold these
   * bytes (`bytesKey`) for this layer at this sampling and prefilter level. */
  needsUpload(docId: string, id: string, bytesKey: string, level: number, nearest: boolean): boolean {
    const existing = this.layers.get(key(docId, id));
    return !existing || existing.key !== bytesKey || existing.level !== level || existing.nearest !== nearest;
  }

  /** Uploads when the bytes, sampling or prefilter level changed. `pixels` is the contiguous
   * premultiplied RGBA buffer *at `level`* (the layer's pixels, or its effects image), and `size`
   * its dimensions there.
   *
   * No mipmaps: the chain would be built per 2048-pixel chunk with CLAMP_TO_EDGE, which seams
   * a large layer at low zoom, and the CPU compositor has no equivalent. Both renderers instead
   * reduce the whole raster with the same `prefilterLevel` rule and take one LINEAR tap. */
  sync(docId: string, id: string, bytesKey: string, nearest: boolean, pixels: Uint8Array | null, level: number, size: { width: number; height: number }, revision: number | null = null): void {
    const k = key(docId, id);
    const existing = this.layers.get(k);
    if (!pixels || size.width === 0 || size.height === 0) { if (existing) this.remove(docId, id); return; }
    if (existing && existing.key === bytesKey && existing.level === level && existing.nearest === nearest) return;
    // A whole upload still replaces every texel, but a same-sized result need
    // not destroy/recreate its GL objects. The source prefilter level can change
    // when a reduced preview becomes the full result without changing this grid.
    // Keep the chunk objects, replace every texel and update their filters/metadata.
    const reuse = existing && existing.width === size.width && existing.height === size.height ? existing : undefined;
    if (existing && !reuse) this.remove(docId, id);
    const gl = this.gl;
    const chunks: Chunk[] = [];
    gl.pixelStorei(gl.UNPACK_ROW_LENGTH, size.width);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    for (let y = 0; y < size.height; y += CHUNK) {
      for (let x = 0; x < size.width; x += CHUNK) {
        const width = Math.min(CHUNK, size.width - x), height = Math.min(CHUNK, size.height - y);
        const texture = reuse?.chunks[chunks.length]?.texture ?? gl.createTexture()!;
        gl.bindTexture(gl.TEXTURE_2D, texture);
        gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, x);
        gl.pixelStorei(gl.UNPACK_SKIP_ROWS, y);
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, width, height, 0, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, nearest ? gl.NEAREST : gl.LINEAR);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, nearest ? gl.NEAREST : gl.LINEAR);
        chunks.push({ texture, x, y, width, height });
      }
    }
    gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, 0);
    gl.pixelStorei(gl.UNPACK_SKIP_ROWS, 0);
    gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0);
    this.layers.set(k, { key: bytesKey, revision, level, width: size.width, height: size.height, nearest, chunks });
    this.serial++;
  }
  /** Uploads `region`, the bytes of `rect` (in the texture's own pixels, at its level; the engine's
   * `layer_region`), into every chunk the rectangle meets: `texSubImage2D` with the region's row length
   * and each chunk's skips. The texture then names `bytesKey` and `revision`. */
  update(docId: string, id: string, bytesKey: string, revision: number, rect: PixelRect, region: Uint8Array): void {
    const t = this.layers.get(key(docId, id));
    if (!t) return;
    const gl = this.gl;
    if (rect.width > 0 && rect.height > 0) {
      gl.pixelStorei(gl.UNPACK_ROW_LENGTH, rect.width);
      gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
      for (const c of t.chunks) {
        const x0 = Math.max(rect.x, c.x), y0 = Math.max(rect.y, c.y);
        const x1 = Math.min(rect.x + rect.width, c.x + c.width), y1 = Math.min(rect.y + rect.height, c.y + c.height);
        if (x1 <= x0 || y1 <= y0) continue;
        gl.bindTexture(gl.TEXTURE_2D, c.texture);
        gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, x0 - rect.x);
        gl.pixelStorei(gl.UNPACK_SKIP_ROWS, y0 - rect.y);
        gl.texSubImage2D(gl.TEXTURE_2D, 0, x0 - c.x, y0 - c.y, x1 - x0, y1 - y0, gl.RGBA, gl.UNSIGNED_BYTE, region);
      }
      gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, 0);
      gl.pixelStorei(gl.UNPACK_SKIP_ROWS, 0);
      gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0);
    }
    t.key = bytesKey; t.revision = revision;
    this.serial++;
  }
  remove(docId: string, id: string): void {
    const k = key(docId, id);
    const t = this.layers.get(k);
    if (!t) return;
    for (const c of t.chunks) this.gl.deleteTexture(c.texture);
    this.layers.delete(k);
    this.serial++;
  }
  /** Keeps only `${docId}:${id}` entries for the given document's current layer ids, dropping
   * every other document's textures too -- each document only ever retains its own keys. */
  retainOnly(docId: string, ids: Set<string>): void {
    const keep = new Set([...ids].map((id) => key(docId, id)));
    for (const k of [...this.layers.keys()]) {
      if (keep.has(k)) continue;
      const t = this.layers.get(k)!;
      for (const c of t.chunks) this.gl.deleteTexture(c.texture);
      this.layers.delete(k);
      this.serial++;
    }
  }
  dispose(): void {
    if (this.layers.size) this.serial++;
    for (const t of this.layers.values()) for (const c of t.chunks) this.gl.deleteTexture(c.texture);
    this.layers.clear();
  }
}
