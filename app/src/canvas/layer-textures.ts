export const CHUNK = 2048;
/** A 30000-pixel side reaches 1 pixel in 15 halvings; mirrors MAX_PREFILTER_LEVEL in the engine. */
export const MAX_PREFILTER_LEVEL = 16;

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

export interface Chunk { texture: WebGLTexture; x: number; y: number; width: number; height: number; }
/** `key` names the bytes: the layer's pixels at a revision, or its effects image (GlRenderer.syncTextures). */
export interface LayerTexture { key: string; level: number; width: number; height: number; nearest: boolean; chunks: Chunk[]; }

/** Two sessions opened from the same `.comp` file carry identical layer ids (they come from
 * the manifest), so the cache is keyed by document handle *and* layer id -- otherwise one
 * document's texture can be served to another that happens to share a layer id and revision. */
function key(docId: string, layerId: string): string { return `${docId}:${layerId}`; }

export class LayerTextures {
  private layers = new Map<string, LayerTexture>();
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
  sync(docId: string, id: string, bytesKey: string, nearest: boolean, pixels: Uint8Array | null, level: number, size: { width: number; height: number }): void {
    const k = key(docId, id);
    const existing = this.layers.get(k);
    if (!pixels || size.width === 0 || size.height === 0) { if (existing) this.remove(docId, id); return; }
    if (existing && existing.key === bytesKey && existing.level === level && existing.nearest === nearest) return;
    if (existing) this.remove(docId, id);
    const gl = this.gl;
    const chunks: Chunk[] = [];
    gl.pixelStorei(gl.UNPACK_ROW_LENGTH, size.width);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    for (let y = 0; y < size.height; y += CHUNK) {
      for (let x = 0; x < size.width; x += CHUNK) {
        const width = Math.min(CHUNK, size.width - x), height = Math.min(CHUNK, size.height - y);
        const texture = gl.createTexture()!;
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
    this.layers.set(k, { key: bytesKey, level, width: size.width, height: size.height, nearest, chunks });
  }
  remove(docId: string, id: string): void {
    const k = key(docId, id);
    const t = this.layers.get(k);
    if (!t) return;
    for (const c of t.chunks) this.gl.deleteTexture(c.texture);
    this.layers.delete(k);
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
    }
  }
  dispose(): void {
    for (const t of this.layers.values()) for (const c of t.chunks) this.gl.deleteTexture(c.texture);
    this.layers.clear();
  }
}
