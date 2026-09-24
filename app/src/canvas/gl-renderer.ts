import type { Coverage, DocumentState, LayerDraw, PreviewEdit, RenderPlan } from "../engine/types";
import { DRAWN_ADJUSTMENT_KINDS } from "../engine/types";
import type { EngineClient } from "../engine/client";
import type { Viewport } from "./viewport";
import { LayerTextures, prefilterLevel, sizeAtLevel } from "./layer-textures";
import type { RenderOptions, Renderer } from "./renderer";
import { ADJUST_KIND, BLEND_INDEX, createPrograms, disposePrograms, type Programs } from "./gl/programs";
import { FboPool } from "./gl/framebuffers";
import { MaskTextures } from "./gl/mask-textures";
import { AdjustTextures } from "./gl/adjust-textures";
import { cornersOf, fromTuple, homographyUnitTo, mat3Invert, mat3Mul, pixelToDocument, type Mat3, type P } from "../tools/transform-geometry";

const MAX_CLIP_LEVELS = 3;

export class GlRenderer implements Renderer {
  readonly kind = "gl" as const;
  private textures: LayerTextures;
  private masks: MaskTextures;
  private fbos: FboPool;
  private programs: Programs;
  private adjustTextures: AdjustTextures;
  private white: WebGLTexture;
  private transparent: WebGLTexture;
  private W = 0; private H = 0;
  constructor(private readonly canvas: HTMLCanvasElement, private readonly gl: WebGL2RenderingContext) {
    this.textures = new LayerTextures(gl); this.masks = new MaskTextures(gl); this.fbos = new FboPool(gl); this.programs = createPrograms(gl);
    this.adjustTextures = new AdjustTextures(gl);
    this.white = this.solid(gl.R8, gl.RED, [255]); this.transparent = this.solid(gl.RGBA8, gl.RGBA, [0, 0, 0, 0]);
  }
  private solid(internal: number, format: number, bytes: number[]): WebGLTexture {
    const gl = this.gl; const t = gl.createTexture()!; gl.bindTexture(gl.TEXTURE_2D, t);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1); gl.texImage2D(gl.TEXTURE_2D, 0, internal, 1, 1, 0, format, gl.UNSIGNED_BYTE, new Uint8Array(bytes)); gl.pixelStorei(gl.UNPACK_ALIGNMENT, 4);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    return t;
  }

  /**
   * Uploads each layer at the prefilter level the plan's draw calls for, so the texture is the
   * same reduction the CPU compositor samples at this zoom. A layer drawn more than once (a
   * plan node and a clipping source) takes the smallest level, i.e. the most detail.
   *
   * This runs with the plan and viewport in hand rather than ahead of them, because the level
   * depends on both: it needs the displayed transform (a pending scale drag moves it) and the
   * device pixels per document unit.
   */
  private syncTextures(engine: EngineClient, state: DocumentState, plan: RenderPlan, viewport: Viewport, dpr: number): void {
    const outPerDoc = viewport.pointsPerPixel * dpr;
    const levels = new Map<string, number>();
    const adjustKeys = new Set<string>();
    const note = (d: LayerDraw) => {
      if (d.adjustment) adjustKeys.add(AdjustTextures.key(d.adjustment));
      if (d.pixelsWidth === 0) return;
      const layer = state.layers.find((l) => l.id === d.id);
      const nearest = layer?.transform.sampling === "Nearest";
      // Nearest never prefilters, and neither does a distortion: the homography resamples the
      // full raster. Both match `compositor::prefilters` in the engine.
      const level = nearest || d.corners ? 0 : prefilterLevel(d.pixelsWidth, d.pixelsHeight, d.pixelsWidth / Math.max(1e-9, d.transform.size[0] * outPerDoc));
      const seen = levels.get(d.id);
      levels.set(d.id, seen === undefined ? level : Math.min(seen, level));
    };
    for (const n of plan.nodes) { if (n.kind === "layer") note(n.draw); else { note(n.base); n.children.forEach(note); } }
    for (const s of plan.sources) note(s);
    this.adjustTextures.retain(adjustKeys);
    const keep = new Set<string>();
    for (const layer of state.layers) {
      keep.add(layer.id);
      const level = levels.get(layer.id) ?? 0;
      if (!this.textures.needsUpload(state.id, layer, level)) continue;
      const size = sizeAtLevel(layer.pixelsWidth, layer.pixelsHeight, level);
      const pixels = layer.pixelsWidth > 0 ? engine.layerPixels(state.id, layer.id, level) : null;
      this.textures.sync(state.id, layer, pixels, level, size);
    }
    this.textures.retainOnly(state.id, keep);
  }

  render(engine: EngineClient, state: DocumentState, viewport: Viewport, dpr: number, options: RenderOptions, edit: PreviewEdit | null): void {
    const gl = this.gl;
    const W = Math.max(1, Math.round(viewport.viewSize.width * dpr)), H = Math.max(1, Math.round(viewport.viewSize.height * dpr));
    if (this.canvas.width !== W || this.canvas.height !== H) { this.canvas.width = W; this.canvas.height = H; }
    this.W = W; this.H = H; this.fbos.resize(W, H);
    const plan = engine.renderPlan(state.id, edit);
    this.syncTextures(engine, state, plan, viewport, dpr);
    this.syncMasks(engine, state, plan);
    gl.bindVertexArray(this.programs.vao);
    gl.viewport(0, 0, W, H);
    gl.disable(gl.BLEND);
    const ctx: Ctx = { state, plan, viewport, dpr, engine };
    this.fbos.clear("mainA", "rgba", 0);
    for (const node of plan.nodes) {
      if (node.kind === "layer") this.drawInto(ctx, "main", node.draw, node.draw.blend, true, 0);
      else {
        this.fbos.clear("stackA", "rgba", 0);
        this.drawInto(ctx, "stack", node.base, "Normal", true, 0);
        this.pass(this.programs.alphaOf, "baseAlpha", "r8", { src: this.fbos.get("stackA", "rgba").tex });
        this.pass(this.programs.opaque, "stackB", "rgba", { src: this.fbos.get("stackA", "rgba").tex }); this.fbos.swap("stackA", "stackB");
        for (const child of node.children) this.drawInto(ctx, "stack", child, child.blend, false, 0);
        this.pass(this.programs.restore, "stackB", "rgba", { src: this.fbos.get("stackA", "rgba").tex, alpha: this.fbos.get("baseAlpha", "r8").tex }); this.fbos.swap("stackA", "stackB");
        this.buildCoverage(ctx, node.folderCoverages, 0);
        const stackTex = this.fbos.get("stackA", "rgba").tex;
        this.fbos.blit("mainA", "mainB");
        this.composeTexture(ctx, "mainB", stackTex, this.viewCorners(viewport), { x: 0, y: 0, w: 1, h: 1 }, false, true, 1, BLEND_INDEX[node.base.blend], node.folderCoverages.length > 0 ? 0 : null, this.fbos.get("mainA", "rgba").tex);
        this.fbos.swap("mainA", "mainB");
      }
    }
    this.screenPass(state, viewport, dpr, options);
  }

  private syncMasks(engine: EngineClient, state: DocumentState, plan: RenderPlan): void {
    const keep = new Set<string>();
    const visit = (c: Coverage) => { keep.add(c.layerId); const px = engine.maskPixels(state.id, c.layerId); if (px) this.masks.sync(state.id, c.layerId, c.maskRevision, c.width, c.height, px); };
    for (const n of plan.nodes) { if (n.kind === "layer") n.draw.coverages.forEach(visit); else { n.base.coverages.forEach(visit); n.children.forEach((c) => c.coverages.forEach(visit)); n.folderCoverages.forEach(visit); } }
    for (const s of plan.sources) s.coverages.forEach(visit);
    this.masks.retainOnly(state.id, keep);
  }

  /** Document -> view CSS px. */
  private docToView(viewport: Viewport, state: DocumentState): Mat3 {
    const rect = viewport.documentRect({ width: state.width, height: state.height }); const ppp = viewport.pointsPerPixel;
    return [ppp, 0, rect.x, 0, ppp, rect.y, 0, 0, 1];
  }
  /** View CSS px -> clip space. */
  private viewToClip(viewport: Viewport): Mat3 { const vw = viewport.viewSize.width, vh = viewport.viewSize.height; return [2 / vw, 0, -1, 0, -2 / vh, 1, 0, 0, 1]; }
  /** gl_FragCoord (device px, y up) -> document. */
  private deviceToDoc(viewport: Viewport, state: DocumentState, dpr: number): Mat3 {
    const rect = viewport.documentRect({ width: state.width, height: state.height }); const ppp = viewport.pointsPerPixel;
    return [1 / (dpr * ppp), 0, -rect.x / ppp, 0, -1 / (dpr * ppp), (this.H / dpr - rect.y) / ppp, 0, 0, 1];
  }
  private viewCorners(viewport: Viewport): P[] { const w = viewport.viewSize.width, h = viewport.viewSize.height; return [{ x: 0, y: 0 }, { x: w, y: 0 }, { x: w, y: h }, { x: 0, y: h }]; }

  private buildCoverage(ctx: Ctx, coverages: Coverage[], level: number): void {
    const gl = this.gl; const name = `coverage${level}`;
    this.fbos.clear(name, "r8", 1);
    if (coverages.length === 0) return;
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbos.get(name, "r8").fbo);
    gl.enable(gl.BLEND); gl.blendFunc(gl.DST_COLOR, gl.ZERO);
    const p = this.programs.coverage; gl.useProgram(p.program);
    const d2d = this.deviceToDoc(ctx.viewport, ctx.state, ctx.dpr);
    for (const c of coverages) {
      // syncMasks uploaded every coverage in the plan before this ran, so a miss means the
      // layer's mask went away between the two; skip it rather than uploading an empty buffer.
      const tex = this.masks.get(ctx.state.id, c.layerId);
      if (!tex) continue;
      this.masks.setFilter(tex, c.nearest);
      let maskFromDoc: Mat3 | null;
      if (c.corners) {
        const inv = mat3Invert(homographyUnitTo(c.corners.map(fromTuple)));
        const fx = c.placement.flipX, fy = c.placement.flipY;
        maskFromDoc = inv ? mat3Mul([fx ? -c.width : c.width, 0, fx ? c.width : 0, 0, fy ? -c.height : c.height, fy ? c.height : 0, 0, 0, 1], inv) : null;
      } else maskFromDoc = mat3Invert(pixelToDocument(c.placement, c.width, c.height));
      if (!maskFromDoc) continue;
      const m = mat3Mul(maskFromDoc, d2d);
      gl.uniformMatrix3fv(p.uniforms.deviceToMask, true, new Float32Array(m));
      gl.uniform2f(p.uniforms.maskSize, c.width, c.height);
      gl.uniform1f(p.uniforms.background, c.background / 255);
      gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, tex); gl.uniform1i(p.uniforms.mask, 0);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    }
    gl.disable(gl.BLEND);
  }

  /** Multiplies coverage{level} by the alpha of the source drawn into clip{level}. */
  private applyClip(ctx: Ctx, sourceId: string, level: number): void {
    const gl = this.gl;
    const source = ctx.plan.sources.find((s) => s.id === sourceId);
    if (!source || level >= MAX_CLIP_LEVELS) return;
    this.fbos.clear(`clip${level}`, "rgba", 0);
    this.buildCoverage(ctx, source.coverages, level + 1);
    if (source.clip) this.applyClip(ctx, source.clip, level + 1);
    // No backdrop: clip{level} was just cleared, so the destination is zero everywhere.
    this.drawLayer(ctx, `clip${level}`, null, source, 0, source.coverages.length > 0 || !!source.clip ? level + 1 : null);
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbos.get(`coverage${level}`, "r8").fbo);
    gl.enable(gl.BLEND); gl.blendFunc(gl.DST_COLOR, gl.ZERO);
    const p = this.programs.alphaOf; gl.useProgram(p.program);
    gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, this.fbos.get(`clip${level}`, "rgba").tex); gl.uniform1i(p.uniforms.src, 0);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    gl.disable(gl.BLEND);
  }

  private drawInto(ctx: Ctx, pair: "main" | "stack", draw: LayerDraw, blend: string, useClip: boolean, level: number): void {
    const hasCoverage = draw.coverages.length > 0 || (useClip && !!draw.clip);
    if (draw.adjustment) {
      if (hasCoverage) { this.buildCoverage(ctx, draw.coverages, level); if (useClip && draw.clip) this.applyClip(ctx, draw.clip, level); }
      this.adjustPass(ctx, pair, draw, blend, hasCoverage ? level : null);
      this.fbos.swap(`${pair}A`, `${pair}B`);
      return;
    }
    const t = this.textures.get(ctx.state.id, draw.id);
    if (!t) return;
    if (hasCoverage) { this.buildCoverage(ctx, draw.coverages, level); if (useClip && draw.clip) this.applyClip(ctx, draw.clip, level); }
    this.fbos.blit(`${pair}A`, `${pair}B`);
    const backdrop = this.fbos.get(`${pair}A`, "rgba").tex;
    this.drawLayer(ctx, `${pair}B`, backdrop, draw, BLEND_INDEX[blend as keyof typeof BLEND_INDEX], hasCoverage ? level : null);
    this.fbos.swap(`${pair}A`, `${pair}B`);
  }

  /** Maps what is already in the pair's A buffer through the adjustment, into B. Nothing is
   * sampled from the layer: an adjustment layer has no pixels of its own. */
  private adjustPass(ctx: Ctx, pair: "main" | "stack", draw: LayerDraw, blend: string, coverageLevel: number | null): void {
    const gl = this.gl; const adjustment = draw.adjustment!;
    const p = this.programs.adjust;
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbos.get(`${pair}B`, "rgba").fbo);
    gl.useProgram(p.program);
    const kind = !DRAWN_ADJUSTMENT_KINDS.includes(adjustment.kind) ? ADJUST_KIND.identity
      : adjustment.kind === "Hue/Saturation" ? ADJUST_KIND.hsv : adjustment.kind === "Grain" ? ADJUST_KIND.grain
      : adjustment.kind === "Gradient Map" ? ADJUST_KIND.gradientMap : ADJUST_KIND.tables;
    gl.uniform1i(p.uniforms.kind, kind);
    gl.uniform1f(p.uniforms.opacity, draw.opacity);
    gl.uniform1i(p.uniforms.mode, BLEND_INDEX[blend as keyof typeof BLEND_INDEX]);
    gl.uniform1i(p.uniforms.useCoverage, coverageLevel === null ? 0 : 1);
    gl.uniformMatrix3fv(p.uniforms.deviceToDoc, true, new Float32Array(this.deviceToDoc(ctx.viewport, ctx.state, ctx.dpr)));
    const hsv = adjustment.hsvSettings;
    // Colorize reads the SELECTED range's adjustment, not Master, mirroring hsv.rs::adjust_rgb,
    // whose colorize branch is settings.adjustment(settings.range). The finished panel cannot
    // produce colorize with a non-Master range (the Colorize toggle resets the whole settings
    // object to Master), but a .comp file or a SetAdjustment command can, and the parity
    // constraint covers every adjustment layer, not only UI-reachable ones.
    // When the selected range is ABSENT from the sparse adjustments map, fall back to ZERO, not
    // to Master. Rust's HueSaturationSettings::adjustment is
    // adjustments.get(&range).copied().unwrap_or_default() and the Mac is
    // adjustments[range]?.hue ?? 0 (HueSaturation.swift:188) -- both zero. is_valid() never
    // requires `range` to be a key, so {range:"Reds", colorize:true, adjustments:{Master:...}}
    // passes set_adjustment and .comp load. Falling back to Master here measured 26/255 against
    // the CPU. With no hsvSettings at all, resolved_hsv() builds one from the legacy scalars
    // under Master with range defaulting to Master, so the scalars are correct in that case and
    // only in that case.
    const selected = hsv
        ? (hsv.adjustments?.[hsv.range] ?? { hue: 0, saturation: 0, lightness: 0 })
        : { hue: adjustment.hue, saturation: adjustment.saturation, lightness: adjustment.lightness };
    gl.uniform1i(p.uniforms.colorize, (hsv?.colorize ?? adjustment.colorize) ? 1 : 0);
    gl.uniform3f(p.uniforms.colorizeAmounts, selected.hue, selected.saturation, selected.lightness);
    const rawGrain = adjustment.grainSettings ?? { amount: 25, size: 1.5, roughness: 50, seed: 0 };
    // Mirrors GrainSettings::normalized() in engine/src/adjust/settings.rs, which prepare() always
    // applies before use. is_valid() prevents a live divergence today, but the CPU normalizes
    // defensively and the GPU should too -- that asymmetry is exactly what produced Fix 1 above.
    // clampOr mirrors settings.rs's own clamp_or: a non-finite input falls back to the field's
    // default rather than clamping (every comparison with NaN is false, so Math.min/Math.max
    // alone would let a non-finite value through untouched).
    const clampOr = (n: number, lo: number, hi: number, fallback: number): number =>
      Number.isFinite(n) ? Math.min(hi, Math.max(lo, n)) : fallback;
    const grain = {
      amount: clampOr(rawGrain.amount, 0, 100, 25),
      size: clampOr(rawGrain.size, 0.5, 20, 1.5),
      roughness: clampOr(rawGrain.roughness, 0, 100, 50),
      seed: rawGrain.seed,
    };
    // strength mirrors `grain_strength` in engine/src/adjust/grain.rs.
    gl.uniform3f(p.uniforms.grain, grain.size, grain.roughness, Math.min(1, grain.amount / 100) * 0.35 * 255);
    gl.uniform1ui(p.uniforms.grainSeed, grain.seed >>> 0);
    gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, this.fbos.get(`${pair}A`, "rgba").tex); gl.uniform1i(p.uniforms.src, 0);
    gl.activeTexture(gl.TEXTURE1); gl.bindTexture(gl.TEXTURE_2D, coverageLevel === null ? this.white : this.fbos.get(`coverage${coverageLevel}`, "r8").tex); gl.uniform1i(p.uniforms.coverage, 1);
    gl.activeTexture(gl.TEXTURE2); gl.bindTexture(gl.TEXTURE_2D, this.adjustTextures.lut(ctx.engine, adjustment) ?? this.white); gl.uniform1i(p.uniforms.lut, 2);
    gl.activeTexture(gl.TEXTURE3); gl.bindTexture(gl.TEXTURE_2D, this.adjustTextures.response(ctx.engine, adjustment) ?? this.white); gl.uniform1i(p.uniforms.response, 3);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  }

  /** Draws every chunk of a layer texture into `target` reading `backdrop`, or onto a cleared
   * target when `backdrop` is null. */
  private drawLayer(ctx: Ctx, target: string, backdrop: WebGLTexture | null, draw: LayerDraw, mode: number, coverageLevel: number | null): void {
    const t = this.textures.get(ctx.state.id, draw.id); if (!t) return;
    const d2v = this.docToView(ctx.viewport, ctx.state);
    const cornersDoc = draw.corners ? draw.corners.map(fromTuple) : cornersOf(draw.transform);
    const cornersView = cornersDoc.map((p) => ({ x: d2v[0] * p.x + d2v[1] * p.y + d2v[2], y: d2v[3] * p.x + d2v[4] * p.y + d2v[5] }));
    for (const chunk of t.chunks) {
      const rect = { x: chunk.x / t.width, y: chunk.y / t.height, w: chunk.width / t.width, h: chunk.height / t.height };
      this.composeTexture(ctx, target, chunk.texture, cornersView, rect, draw.transform.flipX, draw.transform.flipY, draw.opacity, mode, coverageLevel, backdrop);
    }
  }

  /** Composes `tex` into `target`, reading `backdrop`. Never blits or swaps -- the caller owns that. */
  private composeTexture(ctx: Ctx, target: string, tex: WebGLTexture, cornersView: P[], uvRect: { x: number; y: number; w: number; h: number }, flipX: boolean, flipY: boolean, opacity: number, mode: number, coverageLevel: number | null, backdrop?: WebGLTexture | null): void {
    const gl = this.gl;
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbos.get(target, "rgba").fbo);
    const p = this.programs.layer; gl.useProgram(p.program);
    const unitToClip = mat3Mul(this.viewToClip(ctx.viewport), homographyUnitTo(cornersView));
    gl.uniformMatrix3fv(p.uniforms.unitToClip, true, new Float32Array(unitToClip));
    gl.uniform4f(p.uniforms.uvRect, uvRect.x, uvRect.y, uvRect.w, uvRect.h);
    gl.uniform1i(p.uniforms.flipX, flipX ? 1 : 0); gl.uniform1i(p.uniforms.flipY, flipY ? 1 : 0);
    gl.uniform1f(p.uniforms.opacity, opacity); gl.uniform1i(p.uniforms.mode, mode);
    gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, tex); gl.uniform1i(p.uniforms.tex, 0);
    gl.activeTexture(gl.TEXTURE1); gl.bindTexture(gl.TEXTURE_2D, backdrop ?? this.transparent); gl.uniform1i(p.uniforms.backdrop, 1);
    gl.activeTexture(gl.TEXTURE2); gl.bindTexture(gl.TEXTURE_2D, coverageLevel === null ? this.white : this.fbos.get(`coverage${coverageLevel}`, "r8").tex); gl.uniform1i(p.uniforms.coverage, 2);
    gl.uniform1i(p.uniforms.useCoverage, coverageLevel === null ? 0 : 1);
    gl.uniform1i(p.uniforms.useBackdrop, backdrop ? 1 : 0);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  }

  private pass(p: { program: WebGLProgram; uniforms: Record<string, WebGLUniformLocation | null> }, target: string, format: "rgba" | "r8", textures: Record<string, WebGLTexture>): void {
    const gl = this.gl; gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbos.get(target, format).fbo); gl.useProgram(p.program);
    let unit = 0;
    for (const [name, tex] of Object.entries(textures)) { gl.activeTexture(gl.TEXTURE0 + unit); gl.bindTexture(gl.TEXTURE_2D, tex); gl.uniform1i(p.uniforms[name], unit); unit++; }
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  }

  private screenPass(state: DocumentState, viewport: Viewport, dpr: number, options: RenderOptions): void {
    const gl = this.gl; const W = this.W, H = this.H;
    gl.bindFramebuffer(gl.FRAMEBUFFER, null); gl.viewport(0, 0, W, H); gl.disable(gl.SCISSOR_TEST);
    gl.clearColor(0.16, 0.16, 0.16, 1); gl.clear(gl.COLOR_BUFFER_BIT);
    const rect = viewport.documentRect({ width: state.width, height: state.height });
    const x0 = Math.round(rect.x * dpr), x1 = Math.round((rect.x + rect.width) * dpr), y0 = Math.round(rect.y * dpr), y1 = Math.round((rect.y + rect.height) * dpr);
    gl.enable(gl.SCISSOR_TEST); gl.scissor(x0, H - y1, x1 - x0, y1 - y0);
    if (options.checkerboard) {
      const p = this.programs.checker; gl.useProgram(p.program);
      gl.uniformMatrix3fv(p.uniforms.unitToClip, true, new Float32Array(mat3Mul(this.viewToClip(viewport), homographyUnitTo([{ x: rect.x, y: rect.y }, { x: rect.x + rect.width, y: rect.y }, { x: rect.x + rect.width, y: rect.y + rect.height }, { x: rect.x, y: rect.y + rect.height }]))));
      gl.uniform4f(p.uniforms.uvRect, 0, 0, 1, 1); gl.uniform1i(p.uniforms.flipX, 0); gl.uniform1i(p.uniforms.flipY, 0);
      gl.uniform2f(p.uniforms.sizePx, rect.width * dpr, rect.height * dpr); gl.uniform1f(p.uniforms.cell, 8 * dpr);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    } else { gl.clearColor(0, 0, 0, 0); gl.clear(gl.COLOR_BUFFER_BIT); }
    gl.enable(gl.BLEND); gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    const b = this.programs.blit; gl.useProgram(b.program);
    gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, this.fbos.get("mainA", "rgba").tex); gl.uniform1i(b.uniforms.src, 0);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    gl.disable(gl.BLEND); gl.disable(gl.SCISSOR_TEST);
  }

  readPixels(): Uint8Array {
    const gl = this.gl; const W = this.canvas.width, H = this.canvas.height;
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    const out = new Uint8Array(W * H * 4); gl.readPixels(0, 0, W, H, gl.RGBA, gl.UNSIGNED_BYTE, out);
    const row = W * 4; const flipped = new Uint8Array(W * H * 4);
    for (let y = 0; y < H; y++) flipped.set(out.subarray(y * row, (y + 1) * row), (H - 1 - y) * row);
    return flipped;
  }
  dispose(): void { this.textures.dispose(); this.masks.dispose(); this.fbos.dispose(); this.adjustTextures.dispose(); disposePrograms(this.gl, this.programs); this.gl.deleteTexture(this.white); this.gl.deleteTexture(this.transparent); }
}

interface Ctx { state: DocumentState; plan: RenderPlan; viewport: Viewport; dpr: number; engine: EngineClient; }
