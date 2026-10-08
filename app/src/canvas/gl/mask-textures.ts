import type { PixelRect } from "../../engine/types";

interface Entry { tex: WebGLTexture; revision: number; width: number; height: number; }
export class MaskTextures {
  private masks = new Map<string, Entry>();
  private serial = 0;
  get generation(): number { return this.serial; }
  constructor(private readonly gl: WebGL2RenderingContext) {}
  private key(docId: string, layerId: string): string { return `${docId}:${layerId}`; }
  /** The cached texture, or undefined. Callers that only want what is already uploaded must use
   * this rather than re-entering `sync` with an empty buffer: on a cache miss `sync` would call
   * texImage2D with an undersized buffer, raise INVALID_OPERATION and cache a garbage mask. */
  get(docId: string, layerId: string): WebGLTexture | undefined { return this.masks.get(this.key(docId, layerId))?.tex; }
  /** Whether the mask at `revision` is already uploaded: nothing to read from the engine. */
  has(docId: string, layerId: string, revision: number): boolean { return this.masks.get(this.key(docId, layerId))?.revision === revision; }
  /** The revision currently cached for this mask, when it is there and its size still matches
   * (`width`/`height`): the baseline a caller asks the engine's `mask_delta` from. Undefined with
   * nothing cached, or a size change, either of which forces a whole upload.
   *
   * Split out from `sync` so the caller asks for the delta BEFORE it reads the mask's pixels: the
   * engine's `mask_delta` call marshals two strings into wasm and allocates its return value,
   * either of which can grow linear memory and detach an already-captured pixels view. Asking
   * first, then reading pixels, keeps the view that finally reaches `sync` valid. */
  cachedRevision(docId: string, layerId: string, width: number, height: number): number | undefined {
    const e = this.masks.get(this.key(docId, layerId));
    return e && e.width === width && e.height === height ? e.revision : undefined;
  }
  /** Uploads the mask at `revision` unless it is there. `rect` is the caller's already-resolved
   * `mask_delta` result (from `cachedRevision`'s baseline): a rectangle takes only what changed,
   * with `texSubImage2D` straight from `pixels`; `null` or `undefined` (no cached revision to diff
   * against, or the engine could not bound the change) uploads the whole mask. */
  sync(docId: string, layerId: string, revision: number, width: number, height: number, pixels: Uint8Array, rect?: PixelRect | null): WebGLTexture {
    const k = this.key(docId, layerId); const e = this.masks.get(k);
    if (e && e.revision === revision) return e.tex;
    const gl = this.gl;
    if (e && e.width === width && e.height === height && rect !== undefined) {
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
        this.serial++;
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
    this.masks.set(k, { tex, revision, width, height }); this.serial++; return tex;
  }
  setFilter(tex: WebGLTexture, nearest: boolean): void {
    const gl = this.gl; gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, nearest ? gl.NEAREST : gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, nearest ? gl.NEAREST : gl.LINEAR);
  }
  retainOnly(docId: string, layerIds: Set<string>): void {
    for (const k of [...this.masks.keys()]) { const [d, l] = k.split(":"); if (d !== docId || !layerIds.has(l)) { this.gl.deleteTexture(this.masks.get(k)!.tex); this.masks.delete(k); this.serial++; } }
  }
  dispose(): void { if (this.masks.size) this.serial++; for (const e of this.masks.values()) this.gl.deleteTexture(e.tex); this.masks.clear(); }
}

/** Resolves and uploads one coverage's mask texture (`GlRenderer.syncMasks`'s `visit`): asks
 * `maskDelta` for what changed since the cached revision BEFORE calling `maskPixels`, so a wasm
 * call in between (mask_delta marshals two strings in and allocates its own return value, either
 * of which can grow linear memory and detach an already-captured pixels view) can never hand a
 * detached buffer to `sync`. Nothing is asked, and the mask uploads whole, when there is nothing
 * cached at this size (`cachedRevision` returns undefined). */
export function syncMask(
  masks: MaskTextures, docId: string, layerId: string, revision: number, width: number, height: number,
  maskDelta: (from: number) => PixelRect | null, maskPixels: () => Uint8Array | null,
): void {
  if (masks.has(docId, layerId, revision)) return;
  const from = masks.cachedRevision(docId, layerId, width, height);
  const rect = from !== undefined ? maskDelta(from) : undefined;
  const pixels = maskPixels();
  if (pixels) masks.sync(docId, layerId, revision, width, height, pixels, rect);
}
