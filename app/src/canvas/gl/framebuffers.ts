export interface Target { fbo: WebGLFramebuffer; tex: WebGLTexture; format: "rgba" | "r8"; }

export class FboPool {
  private targets = new Map<string, Target>();
  private W = 0; private H = 0;
  constructor(private readonly gl: WebGL2RenderingContext) {}
  resize(W: number, H: number): void { if (W === this.W && H === this.H) return; this.W = W; this.H = H; this.dispose(); }
  get(name: string, format: "rgba" | "r8"): Target {
    const existing = this.targets.get(name);
    if (existing && existing.format === format) return existing;
    if (existing) this.remove(name);
    const gl = this.gl;
    const tex = gl.createTexture()!; gl.bindTexture(gl.TEXTURE_2D, tex);
    if (format === "rgba") gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, this.W, this.H, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    else gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, this.W, this.H, 0, gl.RED, gl.UNSIGNED_BYTE, null);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    const fbo = gl.createFramebuffer()!; gl.bindFramebuffer(gl.FRAMEBUFFER, fbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tex, 0);
    if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) throw new Error(`framebuffer ${name} incomplete`);
    const t = { fbo, tex, format }; this.targets.set(name, t); return t;
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
  dispose(): void { for (const k of [...this.targets.keys()]) this.remove(k); }
}
