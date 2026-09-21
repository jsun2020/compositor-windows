interface Entry { tex: WebGLTexture; revision: number; }
export class MaskTextures {
  private masks = new Map<string, Entry>();
  constructor(private readonly gl: WebGL2RenderingContext) {}
  private key(docId: string, layerId: string): string { return `${docId}:${layerId}`; }
  sync(docId: string, layerId: string, revision: number, width: number, height: number, pixels: Uint8Array): WebGLTexture {
    const k = this.key(docId, layerId); const e = this.masks.get(k);
    if (e && e.revision === revision) return e.tex;
    if (e) this.gl.deleteTexture(e.tex);
    const gl = this.gl; const tex = gl.createTexture()!;
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1); gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0); gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, 0); gl.pixelStorei(gl.UNPACK_SKIP_ROWS, 0);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, width, height, 0, gl.RED, gl.UNSIGNED_BYTE, pixels);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 4);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    this.masks.set(k, { tex, revision }); return tex;
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
