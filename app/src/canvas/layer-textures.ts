import type { LayerState } from "../engine/types";

export const CHUNK = 2048;

export interface Chunk { texture: WebGLTexture; x: number; y: number; width: number; height: number; }
export interface LayerTexture { revision: number; width: number; height: number; nearest: boolean; chunks: Chunk[]; }

/** Two sessions opened from the same `.comp` file carry identical layer ids (they come from
 * the manifest), so the cache is keyed by document handle *and* layer id -- otherwise one
 * document's texture can be served to another that happens to share a layer id and revision. */
function key(docId: string, layerId: string): string { return `${docId}:${layerId}`; }

export class LayerTextures {
  private layers = new Map<string, LayerTexture>();
  constructor(private readonly gl: WebGL2RenderingContext) {}

  get(docId: string, id: string): LayerTexture | undefined { return this.layers.get(key(docId, id)); }

  /** Uploads when the revision or sampling changed. `pixels` is the layer's contiguous premultiplied RGBA buffer. */
  sync(docId: string, layer: LayerState, pixels: Uint8Array | null): void {
    const nearest = layer.transform.sampling === "Nearest";
    const k = key(docId, layer.id);
    const existing = this.layers.get(k);
    if (!pixels || layer.pixelsWidth === 0) { if (existing) this.remove(docId, layer.id); return; }
    if (existing && existing.revision === layer.pixelsRevision && existing.nearest === nearest) return;
    if (existing) this.remove(docId, layer.id);
    const gl = this.gl;
    const chunks: Chunk[] = [];
    gl.pixelStorei(gl.UNPACK_ROW_LENGTH, layer.pixelsWidth);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    for (let y = 0; y < layer.pixelsHeight; y += CHUNK) {
      for (let x = 0; x < layer.pixelsWidth; x += CHUNK) {
        const width = Math.min(CHUNK, layer.pixelsWidth - x), height = Math.min(CHUNK, layer.pixelsHeight - y);
        const texture = gl.createTexture()!;
        gl.bindTexture(gl.TEXTURE_2D, texture);
        gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, x);
        gl.pixelStorei(gl.UNPACK_SKIP_ROWS, y);
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, width, height, 0, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, nearest ? gl.NEAREST : gl.LINEAR);
        if (nearest) { gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST); }
        else { gl.generateMipmap(gl.TEXTURE_2D); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR); }
        chunks.push({ texture, x, y, width, height });
      }
    }
    gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, 0);
    gl.pixelStorei(gl.UNPACK_SKIP_ROWS, 0);
    gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0);
    this.layers.set(k, { revision: layer.pixelsRevision, width: layer.pixelsWidth, height: layer.pixelsHeight, nearest, chunks });
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
