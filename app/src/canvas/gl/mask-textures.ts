import type { PixelRect } from "../../engine/types";

interface Entry { tex: WebGLTexture; revision: number; width: number; height: number; }
export class MaskTextures {
  private masks = new Map<string, Entry>();
  constructor(private readonly gl: WebGL2RenderingContext) {}
  private key(docId: string, layerId: string): string { return `${docId}:${layerId}`; }
  /** The cached texture, or undefined. Callers that only want what is already uploaded must use
   * this rather than re-entering `sync` with an empty buffer: on a cache miss `sync` would call
   * texImage2D with an undersized buffer, raise INVALID_OPERATION and cache a garbage mask. */
  get(docId: string, layerId: string): WebGLTexture | undefined { return this.masks.get(this.key(docId, layerId))?.tex; }
  /** Whether the mask at `revision` is already uploaded: nothing to read from the engine. */
  has(docId: string, layerId: string, revision: number): boolean { return this.masks.get(this.key(docId, layerId))?.revision === revision; }
  /** Uploads the mask at `revision` unless it is there. A mask of the same size already uploaded at
   * another revision takes only what changed since (`delta`, the engine's `mask_delta`: a rectangle,
   * or null for the whole mask), with `texSubImage2D` straight from `pixels`. */
  sync(docId: string, layerId: string, revision: number, width: number, height: number, pixels: Uint8Array, delta?: (from: number) => PixelRect | null): WebGLTexture {
    const k = this.key(docId, layerId); const e = this.masks.get(k);
    if (e && e.revision === revision) return e.tex;
    const gl = this.gl;
    if (e && e.width === width && e.height === height && delta) {
      const rect = delta(e.revision);
      if (rect) {
        if (rect.width > 0 && rect.height > 0) {
          gl.bindTexture(gl.TEXTURE_2D, e.tex);
          gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1); gl.pixelStorei(gl.UNPACK_ROW_LENGTH, width);
          gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, rect.x); gl.pixelStorei(gl.UNPACK_SKIP_ROWS, rect.y);
          gl.texSubImage2D(gl.TEXTURE_2D, 0, rect.x, rect.y, rect.width, rect.height, gl.RED, gl.UNSIGNED_BYTE, pixels);
          gl.pixelStorei(gl.UNPACK_ALIGNMENT, 4); gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0);
          gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, 0); gl.pixelStorei(gl.UNPACK_SKIP_ROWS, 0);
        }
        e.revision = revision;
        return e.tex;
      }
    }
    if (e) gl.deleteTexture(e.tex);
    const tex = gl.createTexture()!;
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1); gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0); gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, 0); gl.pixelStorei(gl.UNPACK_SKIP_ROWS, 0);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, width, height, 0, gl.RED, gl.UNSIGNED_BYTE, pixels);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 4);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    this.masks.set(k, { tex, revision, width, height }); return tex;
  }
  setFilter(tex: WebGLTexture, nearest: boolean): void {
    const gl = this.gl; gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, nearest ? gl.NEAREST : gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, nearest ? gl.NEAREST : gl.LINEAR);
  }
  retainOnly(docId: string, layerIds: Set<string>): void {
    for (const k of [...this.masks.keys()]) { const [d, l] = k.split(":"); if (d !== docId || !layerIds.has(l)) { this.gl.deleteTexture(this.masks.get(k)!.tex); this.masks.delete(k); } }
  }
  dispose(): void { for (const e of this.masks.values()) this.gl.deleteTexture(e.tex); this.masks.clear(); }
}
