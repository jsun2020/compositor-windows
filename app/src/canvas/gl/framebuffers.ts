export interface Target { fbo: WebGLFramebuffer; tex: WebGLTexture; format: "rgba" | "r8"; }

export class FboPool {
  private targets = new Map<string, Target>();
  private sizedTargets = new Map<string, Target & { aw: number; ah: number }>();
  private W = 0; private H = 0;
  private AW = 0; private AH = 0;
  constructor(private readonly gl: WebGL2RenderingContext) {}
  /** The buffers' size is W x H, and that is all any pass reads; their textures are allocated in
   * steps of `step` pixels, so a frame whose size changes by a few pixels as the view pans keeps
   * its buffers (audit F-M1). The GPU's frame passes 256; everything else passes 1, which is
   * exactly the allocation of the view it always had. */
  resize(W: number, H: number, step = 1): void {
    const aw = Math.ceil(W / step) * step, ah = Math.ceil(H / step) * step;
    this.W = W; this.H = H;
    if (aw === this.AW && ah === this.AH) return;
    this.AW = aw; this.AH = ah; this.dispose();
  }
  /** The allocated texture size, for drawing a whole buffer back (GlRenderer.viewCorners). */
  allocated(): { w: number; h: number } { return { w: this.AW, h: this.AH }; }
  get(name: string, format: "rgba" | "r8"): Target {
    const existing = this.targets.get(name);
    if (existing && existing.format === format) return existing;
    if (existing) this.remove(name);
    const gl = this.gl;
    const tex = gl.createTexture()!; gl.bindTexture(gl.TEXTURE_2D, tex);
    if (format === "rgba") gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, this.AW, this.AH, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    else gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, this.AW, this.AH, 0, gl.RED, gl.UNSIGNED_BYTE, null);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    const fbo = gl.createFramebuffer()!; gl.bindFramebuffer(gl.FRAMEBUFFER, fbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tex, 0);
    if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) throw new Error(`framebuffer ${name} incomplete`);
    const t = { fbo, tex, format }; this.targets.set(name, t); return t;
  }
  /** An RGBA target for a pass below the frame's resolution (a blur's reduced copies), allocated
   * in steps of 256 like the frame and kept across frames while that allocation is unchanged. The
   * passes that use it bound every read by the size they are given. */
  sized(name: string, w: number, h: number): Target {
    const aw = Math.ceil(w / 256) * 256, ah = Math.ceil(h / 256) * 256;
    const existing = this.sizedTargets.get(name);
    if (existing && existing.aw === aw && existing.ah === ah) return existing;
    const gl = this.gl;
    if (existing) { gl.deleteFramebuffer(existing.fbo); gl.deleteTexture(existing.tex); }
    const tex = gl.createTexture()!; gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, aw, ah, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    const fbo = gl.createFramebuffer()!; gl.bindFramebuffer(gl.FRAMEBUFFER, fbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tex, 0);
    if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) throw new Error(`framebuffer ${name} incomplete`);
    const t = { fbo, tex, format: "rgba" as const, aw, ah }; this.sizedTargets.set(name, t); return t;
  }
  swap(a: string, b: string): void { const ta = this.targets.get(a), tb = this.targets.get(b); if (ta && tb) { this.targets.set(a, tb); this.targets.set(b, ta); } }
  blit(from: string, to: string, format: "rgba" | "r8" = "rgba"): void {
    const gl = this.gl; const f = this.get(from, format), t = this.get(to, format);
    gl.bindFramebuffer(gl.READ_FRAMEBUFFER, f.fbo); gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, t.fbo);
    gl.blitFramebuffer(0, 0, this.W, this.H, 0, 0, this.W, this.H, gl.COLOR_BUFFER_BIT, gl.NEAREST);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  }
  clear(name: string, format: "rgba" | "r8", value: number): void {
    const gl = this.gl; gl.bindFramebuffer(gl.FRAMEBUFFER, this.get(name, format).fbo); gl.disable(gl.SCISSOR_TEST);
    gl.clearColor(value, value, value, value); gl.clear(gl.COLOR_BUFFER_BIT);
  }
  private remove(name: string): void { const t = this.targets.get(name); if (!t) return; this.gl.deleteFramebuffer(t.fbo); this.gl.deleteTexture(t.tex); this.targets.delete(name); }
  dispose(): void {
    for (const k of [...this.targets.keys()]) this.remove(k);
    for (const t of this.sizedTargets.values()) { this.gl.deleteFramebuffer(t.fbo); this.gl.deleteTexture(t.tex); }
    this.sizedTargets.clear();
  }
}
