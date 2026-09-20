import type { LayerState } from "../engine/types";

export const CHUNK = 2048;

export interface Chunk { texture: WebGLTexture; x: number; y: number; width: number; height: number; }
export interface LayerTexture { revision: number; width: number; height: number; nearest: boolean; chunks: Chunk[]; }

export class LayerTextures {
  private layers = new Map<string, LayerTexture>();
  constructor(private readonly gl: WebGL2RenderingContext) {}

  get(id: string): LayerTexture | undefined { return this.layers.get(id); }

  /** Uploads when the revision or sampling changed. `pixels` is the layer's contiguous premultiplied RGBA buffer. */
  sync(layer: LayerState, pixels: Uint8Array | null): void {
    const nearest = layer.transform.sampling === "Nearest";
    const existing = this.layers.get(layer.id);
    if (!pixels || layer.pixelsWidth === 0) { if (existing) this.remove(layer.id); return; }
    if (existing && existing.revision === layer.pixelsRevision && existing.nearest === nearest) return;
    if (existing) this.remove(layer.id);
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
    this.layers.set(layer.id, { revision: layer.pixelsRevision, width: layer.pixelsWidth, height: layer.pixelsHeight, nearest, chunks });
  }
  remove(id: string): void {
    const t = this.layers.get(id);
    if (!t) return;
    for (const c of t.chunks) this.gl.deleteTexture(c.texture);
    this.layers.delete(id);
  }
  retainOnly(ids: Set<string>): void { for (const id of [...this.layers.keys()]) if (!ids.has(id)) this.remove(id); }
  dispose(): void { this.retainOnly(new Set()); }
}
