import type { PointTuple, WarpMode } from "../engine/types";
import { WARP_VERTEX, WARP_COPY, WARP_PICKUP, WARP_CARRY, WARP_STAMP, WARP_OFFSET, WARP_LIQUIFY_COLOR } from "./warp-shaders";

export interface GpuWarpSettings { mode: WarpMode; diameter: number; hardness: number; strength: number; }
export interface GpuWarpLimits {
  /** An explicit allocation cap, including padded source, scratch and float carry. */
  maxBytes?: number;
  sourceTileSide?: number;
  workTileSide?: number;
}
export interface WarpTile { x: number; y: number; width: number; height: number; pixels: Uint8Array; }
interface Rect { x: number; y: number; width: number; height: number; }
interface Target { tex: WebGLTexture; fbo: WebGLFramebuffer; width: number; height: number; bytes: number; }
interface Tile { x: number; y: number; color: Target; field: Target | null; }
interface Pass { program: WebGLProgram; uniforms: Map<string, WebGLUniformLocation | null>; }
const DEFAULT_BUDGET = 512 * 1024 * 1024;
const MAX_PIXELS = 100_000_000;
const MAX_APPEND_SAMPLES = 16_777_216;
const MAX_STROKE_SAMPLES = 67_108_864;
// Swift/Rust round halves away from zero, unlike Math.round for negative halves.
const centerOf = (p: PointTuple): PointTuple => p.map(v => v < 0 ? -Math.floor(-v + 0.5) : Math.floor(v + 0.5)) as PointTuple;

/** GPU algorithm session, using a dedicated WebGL2 context. It owns no engine
 * layer, preview or history entry. Its immutable source is uploaded once to a
 * tiled array; only touched color/offset tiles and one dab snapshot are writable.
 * Sparse readback is for guarded worker writeback, not a committed layer.
 * Dispose on cancellation, document switch, completion or context loss.
 */
export class GpuWarpStroke {
  private textures = new Set<WebGLTexture>();
  private framebuffers = new Set<WebGLFramebuffer>();
  private passes = new Map<string, Pass>();
  private tiles = new Map<string, Tile>();
  private source!: WebGLTexture;
  private scratch!: Target;
  private carry: [Target, Target] | null = null;
  private vao: WebGLVertexArrayObject | null = null;
  private last: PointTuple | null = null;
  private disposed = false;
  private allocatedBytes = 0;
  private uploadedBytes = 0;
  private dabCount = 0;
  private inputCount = 0;
  private samples = 0;
  private readonly sourceSide: number;
  private readonly workSide: number;
  private readonly columns: number;
  private readonly radius: number;
  private readonly side: number;
  private readonly budget: number;
  private readonly spacing: number;
  private readonly lost = () => this.dispose();

  constructor(private readonly gl: WebGL2RenderingContext, readonly width: number, readonly height: number,
    pixels: Uint8Array, readonly settings: Readonly<GpuWarpSettings>, limits: GpuWarpLimits = {}) {
    if (!Number.isSafeInteger(width) || !Number.isSafeInteger(height) || width < 1 || height < 1 || width > 30_000 || height > 30_000 ||
      width * height > MAX_PIXELS || pixels.length !== width * height * 4) throw new Error("Invalid warp plane (maximum 100 MP)");
    for (let i = 0; i < pixels.length; i += 4) if (pixels[i] > pixels[i + 3] || pixels[i + 1] > pixels[i + 3] || pixels[i + 2] > pixels[i + 3])
      throw new Error("Warp source must be premultiplied RGBA8");
    const { mode, diameter, hardness, strength } = settings;
    if (!["Liquify", "Smudge"].includes(mode) || !Number.isFinite(diameter) || diameter < 2 || diameter > 2000 ||
      !Number.isFinite(hardness) || hardness < 0 || hardness > 0.98 || !Number.isFinite(strength) || strength < 0.01 || strength > 1)
      throw new Error("Warp settings out of range");
    // Retain a private immutable copy of settings, not a caller's mutable object.
    this.settings = Object.freeze({ ...settings });
    if (gl.isContextLost() || !gl.getExtension("EXT_color_buffer_float")) throw new Error("Warp needs WebGL2 float render targets");
    const maxSide = gl.getParameter(gl.MAX_TEXTURE_SIZE) as number;
    this.sourceSide = limits.sourceTileSide ?? Math.min(1024, Math.max(width, height));
    this.workSide = limits.workTileSide ?? Math.min(256, Math.max(width, height));
    this.budget = limits.maxBytes ?? DEFAULT_BUDGET;
    if (![this.sourceSide, this.workSide].every(n => Number.isInteger(n) && n > 0 && n <= maxSide) ||
      !Number.isSafeInteger(this.budget) || this.budget < 1) throw new Error("Invalid warp resource limits");
    this.columns = Math.ceil(width / this.sourceSide);
    const layers = this.columns * Math.ceil(height / this.sourceSide);
    if (layers > (gl.getParameter(gl.MAX_ARRAY_TEXTURE_LAYERS) as number)) throw new Error("Warp source exceeds GPU array texture capacity");
    this.radius = Math.ceil(diameter / 2); this.side = this.radius * 2 + 1;
    this.spacing = Math.max(1, diameter * (mode === "Smudge" ? 0.005 : 0.025));
    const scratchSide = mode === "Smudge" ? this.side : this.side + 2 * (Math.ceil(this.spacing * strength) + 2);
    if (scratchSide > maxSide) throw new Error("Warp brush exceeds GPU texture capacity");
    const sourceBytes = this.sourceSide ** 2 * layers * 4;
    const scratchBytes = scratchSide ** 2 * (mode === "Smudge" ? 4 : 8);
    const carryBytes = mode === "Smudge" ? this.side ** 2 * 16 * 2 : 0;
    this.checkBudget(sourceBytes + scratchBytes + carryBytes);
    try {
      gl.disable(gl.BLEND); gl.disable(gl.DITHER); gl.disable(gl.DEPTH_TEST); gl.disable(gl.SCISSOR_TEST);
      this.vao = gl.createVertexArray(); if (!this.vao) throw new Error("Cannot allocate warp vertex array");
      gl.bindVertexArray(this.vao);
      this.compile("copy", WARP_COPY);
      if (mode === "Smudge") { this.compile("pickup", WARP_PICKUP); this.compile("carry", WARP_CARRY); this.compile("stamp", WARP_STAMP); }
      else { this.compile("offset", WARP_OFFSET); this.compile("liquify", WARP_LIQUIFY_COLOR); }
      this.checkGl("shader setup");
      this.source = this.texture(gl.TEXTURE_2D_ARRAY);
      gl.texStorage3D(gl.TEXTURE_2D_ARRAY, 1, gl.RGBA8, this.sourceSide, this.sourceSide, layers);
      this.checkGl("source allocation");
      this.allocatedBytes += sourceBytes;
      gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1); gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
      gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false); gl.pixelStorei(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL, gl.NONE);
      gl.pixelStorei(gl.UNPACK_ROW_LENGTH, width);
      gl.pixelStorei(gl.UNPACK_IMAGE_HEIGHT, height);
      for (let y = 0; y < height; y += this.sourceSide) for (let x = 0; x < width; x += this.sourceSide) {
        gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, x); gl.pixelStorei(gl.UNPACK_SKIP_ROWS, y);
        gl.texSubImage3D(gl.TEXTURE_2D_ARRAY, 0, 0, 0, y / this.sourceSide * this.columns + x / this.sourceSide,
          Math.min(this.sourceSide, width - x), Math.min(this.sourceSide, height - y), 1, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
        this.checkGl("source tile upload");
      }
      gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0); gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, 0); gl.pixelStorei(gl.UNPACK_SKIP_ROWS, 0);
      gl.pixelStorei(gl.UNPACK_IMAGE_HEIGHT, 0);
      this.uploadedBytes = pixels.length;
      this.scratch = this.target(scratchSide, scratchSide, mode === "Smudge" ? "rgba8" : "rg32f");
      if (mode === "Smudge") this.carry = [this.target(this.side, this.side, "rgba32f"), this.target(this.side, this.side, "rgba32f")];
      this.prime();
      this.checkGl();
      gl.canvas.addEventListener("webglcontextlost", this.lost);
    } catch (error) { this.dispose(); throw error; }
  }

  diagnostics() {
    return { allocatedBytes: this.allocatedBytes, uploadedBytes: this.uploadedBytes, dabCount: this.dabCount,
      tileCount: this.tiles.size, liveTextures: this.textures.size, liveFramebuffers: this.framebuffers.size,
      livePrograms: this.passes.size, disposed: this.disposed };
  }

  /** Scheduled dab centers. A pickup and sub-spacing move do not change pixels.
   * Point/work/budget refusals are checked before any dab in this append.
   * A GPU runtime failure invalidates the session, never an engine layer.
   */
  append(point: PointTuple): PointTuple[] {
    this.live();
    if (point.length !== 2 || point.some(v => !Number.isFinite(v) || Math.abs(v) > 1_000_000)) throw new Error("Invalid warp point");
    if (this.inputCount >= 4096) throw new Error("The warp stroke has too many input points");
    if (!this.last) {
      try {
        if (this.carry) {
          const center = centerOf(point), rect = { x: center[0] - this.radius, y: center[1] - this.radius, width: this.side, height: this.side };
          this.snapshotColor(rect);
          const pass = this.begin("pickup", this.carry[0], rect);
          this.sampler(pass, "under", this.scratch.tex, 0); this.draw(); this.checkGl();
        }
        this.last = [...point]; this.inputCount++; return [];
      } catch (error) { this.dispose(); throw error; }
    }
    const from = this.last, dx = point[0] - from[0], dy = point[1] - from[1], distance = Math.hypot(dx, dy);
    if (distance < this.spacing) { this.inputCount++; return []; }
    const steps = Math.ceil(distance / this.spacing), work = steps * this.side ** 2;
    if (steps > 4096 || work > MAX_APPEND_SAMPLES || this.samples + work > MAX_STROKE_SAMPLES) throw new Error("The warp stroke is too long");
    const dabs: PointTuple[] = [];
    const needed = new Map<string, PointTuple>();
    for (let step = 1; step <= steps; step++) {
      const t = step / steps, p: PointTuple = [from[0] + dx * t, from[1] + dy * t]; dabs.push(p);
      for (const tile of this.tileCoordinates(this.tipRect(p))) if (!this.tiles.has(this.key(tile))) needed.set(this.key(tile), tile);
    }
    this.checkBudget(needed.size * this.workSide ** 2 * (this.settings.mode === "Smudge" ? 4 : 12));
    try {
      for (const tile of needed.values()) this.addTile(tile);
      let previous = from;
      for (const p of dabs) { if (this.carry) this.smudge(p); else this.liquify(previous, p); previous = p; }
      this.checkGl(); this.last = [...point]; this.inputCount++; this.samples += work; this.dabCount += dabs.length;
      return dabs;
    } catch (error) { this.dispose(); throw error; }
  }

  /** Read only touched tiles. Returned rows are already top-down. No full-plane
   * CPU buffer or readback is made on pointer movement. Unchanged pixels within
   * a tile remain the original; the worker still applies the final footprint.
   */
  readTiles(): WarpTile[] {
    this.live(); const out: WarpTile[] = [], gl = this.gl;
    try {
      for (const tile of this.tiles.values()) {
        const width = Math.min(this.workSide, this.width - tile.x), height = Math.min(this.workSide, this.height - tile.y);
        const pixels = new Uint8Array(width * height * 4);
        gl.bindFramebuffer(gl.FRAMEBUFFER, tile.color.fbo); gl.pixelStorei(gl.PACK_ALIGNMENT, 1);
        gl.readPixels(0, 0, width, height, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
        out.push({ x: tile.x, y: tile.y, width, height, pixels });
      }
      this.checkGl(); return out;
    } catch (error) { this.dispose(); throw error; }
  }

  dispose(): void {
    if (this.disposed) return; this.disposed = true;
    const gl = this.gl; gl.canvas.removeEventListener("webglcontextlost", this.lost);
    gl.useProgram(null); gl.bindVertexArray(null); gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    for (const fbo of this.framebuffers) gl.deleteFramebuffer(fbo);
    for (const tex of this.textures) gl.deleteTexture(tex);
    for (const pass of this.passes.values()) gl.deleteProgram(pass.program);
    if (this.vao) gl.deleteVertexArray(this.vao); this.vao = null;
    this.framebuffers.clear(); this.textures.clear(); this.passes.clear(); this.tiles.clear(); this.carry = null;
    this.allocatedBytes = 0;
  }

  private live(): void {
    if (this.gl.isContextLost()) { this.dispose(); throw new Error("Warp GPU context lost; discard this stroke"); }
    if (this.disposed) throw new Error("Warp session disposed");
  }
  private checkBudget(extra: number): void {
    if (this.allocatedBytes + extra > this.budget) throw new Error("Warp exceeds its GPU memory budget; cancel this stroke or use a smaller brush/path");
  }
  private checkGl(stage = "operation"): void {
    const error = this.gl.getError();
    if (error !== this.gl.NO_ERROR) throw new Error(`Warp GPU ${stage} failed (${error})`);
    if (this.gl.isContextLost()) throw new Error("Warp GPU context lost; discard this stroke");
  }
  private texture(kind: number): WebGLTexture {
    const gl = this.gl, tex = gl.createTexture(); if (!tex) throw new Error("Cannot allocate warp texture");
    this.textures.add(tex); gl.bindTexture(kind, tex);
    gl.texParameteri(kind, gl.TEXTURE_MIN_FILTER, gl.NEAREST); gl.texParameteri(kind, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(kind, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(kind, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    return tex;
  }
  private target(width: number, height: number, format: "rgba8" | "rg32f" | "rgba32f"): Target {
    const gl = this.gl, bytes = width * height * (format === "rgba8" ? 4 : format === "rg32f" ? 8 : 16);
    this.checkBudget(bytes); const tex = this.texture(gl.TEXTURE_2D);
    gl.texStorage2D(gl.TEXTURE_2D, 1, format === "rgba8" ? gl.RGBA8 : format === "rg32f" ? gl.RG32F : gl.RGBA32F, width, height);
    this.checkGl(`${format} allocation`);
    const fbo = gl.createFramebuffer(); if (!fbo) throw new Error("Cannot allocate warp framebuffer");
    this.framebuffers.add(fbo); gl.bindFramebuffer(gl.FRAMEBUFFER, fbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tex, 0);
    if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) throw new Error("Warp float framebuffer is incomplete");
    this.allocatedBytes += bytes; gl.disable(gl.SCISSOR_TEST); gl.clearColor(0, 0, 0, 0); gl.clear(gl.COLOR_BUFFER_BIT);
    this.checkGl(`${format} clear`);
    return { tex, fbo, width, height, bytes };
  }
  private releaseTarget(target: Target): void {
    this.gl.deleteFramebuffer(target.fbo); this.framebuffers.delete(target.fbo);
    this.gl.deleteTexture(target.tex); this.textures.delete(target.tex); this.allocatedBytes -= target.bytes;
  }
  /** Drivers can defer native shader/pipeline compilation until the first draw.
   * Issue each format/pass on disposable one-pixel targets during preparation,
   * so the first pointer movement does not pay that cost. No real tile/anchor
   * is touched and callers can wait for a fence before accepting input.
   */
  private prime(): void {
    const gl = this.gl, color = this.target(1, 1, "rgba8"); let field: Target | null = null;
    const rect = { x: 0, y: 0, width: 1, height: 1 }, point: PointTuple = [0, 0];
    try {
      const copy = this.begin("copy", color, rect); this.sourceUniforms(copy); this.draw();
      if (this.carry) {
        const [old, next] = this.carry;
        const pickup = this.begin("pickup", old, rect); this.sampler(pickup, "under", this.scratch.tex, 0); gl.viewport(0, 0, 1, 1); this.draw();
        const carry = this.begin("carry", next, rect, point); this.sampler(carry, "under", this.scratch.tex, 0); this.sampler(carry, "carried", old.tex, 1); gl.viewport(0, 0, 1, 1); this.draw();
        const stamp = this.begin("stamp", color, rect, point); this.sampler(stamp, "carried", next.tex, 0); gl.uniform2i(this.uniform(stamp, "tipOrigin"), 0, 0); this.draw();
      } else {
        field = this.target(1, 1, "rg32f");
        const offset = this.begin("offset", field, rect, point); this.sampler(offset, "snapshot", this.scratch.tex, 0);
        gl.uniform2i(this.uniform(offset, "dependencyOrigin"), 0, 0); gl.uniform2i(this.uniform(offset, "dependencySize"), 1, 1);
        gl.uniform2f(this.uniform(offset, "movement"), 0, 0); this.draw();
        const painted = this.begin("liquify", color, rect, point); this.sourceUniforms(painted); this.sampler(painted, "offsets", field.tex, 0); this.draw();
      }
      this.checkGl("pipeline preparation");
    } finally { if (field) this.releaseTarget(field); this.releaseTarget(color); }
  }
  private compile(name: string, fragment: string): void {
    const gl = this.gl, shaders: WebGLShader[] = [];
    let program: WebGLProgram | null = null;
    try {
      for (const [kind, source] of [[gl.VERTEX_SHADER, WARP_VERTEX], [gl.FRAGMENT_SHADER, fragment]] as const) {
        const shader = gl.createShader(kind); if (!shader) throw new Error("Cannot allocate warp shader"); shaders.push(shader);
        gl.shaderSource(shader, source); gl.compileShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(`Warp shader: ${gl.getShaderInfoLog(shader)}`);
      }
      program = gl.createProgram(); if (!program) throw new Error("Cannot allocate warp program");
      for (const shader of shaders) gl.attachShader(program, shader); gl.linkProgram(program);
      if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(`Warp program: ${gl.getProgramInfoLog(program)}`);
      this.passes.set(name, { program, uniforms: new Map() }); program = null;
    } finally { for (const shader of shaders) gl.deleteShader(shader); if (program) gl.deleteProgram(program); }
  }
  private uniform(pass: Pass, name: string): WebGLUniformLocation | null {
    if (!pass.uniforms.has(name)) pass.uniforms.set(name, this.gl.getUniformLocation(pass.program, name));
    return pass.uniforms.get(name)!;
  }
  private sampler(pass: Pass, name: string, texture: WebGLTexture, unit: number, kind: number = this.gl.TEXTURE_2D): void {
    const gl = this.gl; gl.activeTexture(gl.TEXTURE0 + unit); gl.bindTexture(kind, texture); gl.uniform1i(this.uniform(pass, name), unit);
  }
  private begin(name: string, target: Target, rect: Rect, point?: PointTuple): Pass {
    const gl = this.gl, pass = this.passes.get(name)!;
    gl.bindFramebuffer(gl.FRAMEBUFFER, target.fbo); gl.viewport(0, 0, target.width, target.height);
    gl.disable(gl.SCISSOR_TEST); gl.useProgram(pass.program); gl.bindVertexArray(this.vao);
    gl.uniform2i(this.uniform(pass, "origin"), rect.x, rect.y); gl.uniform2i(this.uniform(pass, "planeSize"), this.width, this.height);
    if (point) {
      const center = centerOf(point); gl.uniform2i(this.uniform(pass, "center"), center[0], center[1]);
      gl.uniform1f(this.uniform(pass, "radius"), this.settings.diameter / 2);
      gl.uniform1f(this.uniform(pass, "hardness"), this.settings.hardness); gl.uniform1f(this.uniform(pass, "strength"), this.settings.strength);
    }
    return pass;
  }
  private sourceUniforms(pass: Pass): void {
    this.sampler(pass, "original", this.source, 3, this.gl.TEXTURE_2D_ARRAY);
    this.gl.uniform1i(this.uniform(pass, "sourceSide"), this.sourceSide); this.gl.uniform1i(this.uniform(pass, "sourceColumns"), this.columns);
  }
  private draw(): void { this.gl.drawArrays(this.gl.TRIANGLES, 0, 3); }
  private key(p: PointTuple): string { return `${p[0]},${p[1]}`; }
  private tipRect(p: PointTuple): Rect {
    const center = centerOf(p); return { x: center[0] - this.radius, y: center[1] - this.radius, width: this.side, height: this.side };
  }
  private tileCoordinates(rect: Rect): PointTuple[] {
    const out: PointTuple[] = [], right = Math.min(this.width, rect.x + rect.width), bottom = Math.min(this.height, rect.y + rect.height);
    for (let y = Math.floor(Math.max(0, rect.y) / this.workSide) * this.workSide; y < bottom; y += this.workSide)
      for (let x = Math.floor(Math.max(0, rect.x) / this.workSide) * this.workSide; x < right; x += this.workSide) out.push([x, y]);
    return out;
  }
  private addTile(p: PointTuple): void {
    const color = this.target(this.workSide, this.workSide, "rgba8");
    const pass = this.begin("copy", color, { x: p[0], y: p[1], width: this.workSide, height: this.workSide }); this.sourceUniforms(pass); this.draw();
    const field = this.settings.mode === "Liquify" ? this.target(this.workSide, this.workSide, "rg32f") : null;
    this.tiles.set(this.key(p), { x: p[0], y: p[1], color, field });
  }
  private blit(tile: Tile, target: Target, rect: Rect, field: boolean): void {
    const left = Math.max(rect.x, tile.x), top = Math.max(rect.y, tile.y);
    const right = Math.min(rect.x + rect.width, tile.x + this.workSide, this.width), bottom = Math.min(rect.y + rect.height, tile.y + this.workSide, this.height);
    if (right <= left || bottom <= top) return;
    const gl = this.gl; gl.bindFramebuffer(gl.READ_FRAMEBUFFER, (field ? tile.field! : tile.color).fbo);
    gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, target.fbo); gl.disable(gl.SCISSOR_TEST);
    gl.blitFramebuffer(left - tile.x, top - tile.y, right - tile.x, bottom - tile.y,
      left - rect.x, top - rect.y, right - rect.x, bottom - rect.y, gl.COLOR_BUFFER_BIT, gl.NEAREST);
  }
  private snapshotColor(rect: Rect): void {
    const pass = this.begin("copy", this.scratch, rect); this.sourceUniforms(pass); this.draw();
    for (const p of this.tileCoordinates(rect)) { const tile = this.tiles.get(this.key(p)); if (tile) this.blit(tile, this.scratch, rect, false); }
  }
  private restrict(tile: Tile, rect: Rect): void {
    const gl = this.gl, left = Math.max(tile.x, rect.x), top = Math.max(tile.y, rect.y);
    const right = Math.min(tile.x + this.workSide, rect.x + rect.width, this.width), bottom = Math.min(tile.y + this.workSide, rect.y + rect.height, this.height);
    gl.enable(gl.SCISSOR_TEST); gl.scissor(left - tile.x, top - tile.y, right - left, bottom - top);
  }
  private smudge(p: PointTuple): void {
    const rect = this.tipRect(p), [old, next] = this.carry!; this.snapshotColor(rect);
    const pass = this.begin("carry", next, rect, p); this.sampler(pass, "under", this.scratch.tex, 0); this.sampler(pass, "carried", old.tex, 1); this.draw();
    for (const coord of this.tileCoordinates(rect)) {
      const tile = this.tiles.get(this.key(coord))!;
      const pass = this.begin("stamp", tile.color, { x: tile.x, y: tile.y, width: this.workSide, height: this.workSide }, p);
      this.sampler(pass, "carried", next.tex, 0); this.gl.uniform2i(this.uniform(pass, "tipOrigin"), rect.x, rect.y);
      this.restrict(tile, rect); this.draw();
    }
    this.carry = [next, old];
  }
  private liquify(from: PointTuple, p: PointTuple): void {
    const rect = this.tipRect(p), movement: PointTuple = [Math.fround(Math.fround(p[0] - from[0]) * Math.fround(this.settings.strength)), Math.fround(Math.fround(p[1] - from[1]) * Math.fround(this.settings.strength))];
    const margin = Math.ceil(Math.max(Math.abs(movement[0]), Math.abs(movement[1]))) + 2;
    const left = Math.max(0, Math.min(this.width, rect.x - margin)), top = Math.max(0, Math.min(this.height, rect.y - margin));
    const right = Math.max(0, Math.min(this.width, rect.x + rect.width + margin)), bottom = Math.max(0, Math.min(this.height, rect.y + rect.height + margin));
    const dependency = { x: left, y: top, width: right - left, height: bottom - top };
    if (dependency.width === 0 || dependency.height === 0) return;
    const gl = this.gl; gl.bindFramebuffer(gl.FRAMEBUFFER, this.scratch.fbo); gl.disable(gl.SCISSOR_TEST);
    gl.clearColor(0, 0, 0, 0); gl.clear(gl.COLOR_BUFFER_BIT);
    // All contributing tiles are frozen before writing any destination tile.
    for (const coord of this.tileCoordinates(dependency)) { const tile = this.tiles.get(this.key(coord)); if (tile) this.blit(tile, this.scratch, dependency, true); }
    for (const coord of this.tileCoordinates(rect)) {
      const tile = this.tiles.get(this.key(coord))!;
      const pass = this.begin("offset", tile.field!, { x: tile.x, y: tile.y, width: this.workSide, height: this.workSide }, p);
      this.sampler(pass, "snapshot", this.scratch.tex, 0);
      gl.uniform2i(this.uniform(pass, "dependencyOrigin"), dependency.x, dependency.y); gl.uniform2i(this.uniform(pass, "dependencySize"), dependency.width, dependency.height);
      gl.uniform2f(this.uniform(pass, "movement"), movement[0], movement[1]); this.restrict(tile, rect); this.draw();
      const color = this.begin("liquify", tile.color, { x: tile.x, y: tile.y, width: this.workSide, height: this.workSide }, p);
      this.sourceUniforms(color); this.sampler(color, "offsets", tile.field!.tex, 0); this.restrict(tile, rect); this.draw();
    }
  }
}
