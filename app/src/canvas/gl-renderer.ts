import type { DocumentState, LayerState } from "../engine/types";
import type { EngineClient } from "../engine/client";
import type { Viewport } from "./viewport";
import { LayerTextures } from "./layer-textures";
import { renderOrder, type RenderOptions, type Renderer } from "./renderer";

const VERT = `#version 300 es
in vec2 unit;
uniform mat3 unitToClip;
uniform vec4 uvRect; // x, y, w, h within the layer in 0..1
out vec2 uv;
void main() {
  vec3 clip = unitToClip * vec3(unit, 1.0);
  gl_Position = vec4(clip.xy, 0.0, 1.0);
  uv = unit;
}`;
const FRAG = `#version 300 es
precision highp float;
in vec2 uv;
uniform sampler2D tex;
uniform float opacity;
out vec4 color;
void main() { color = texture(tex, uv) * opacity; }`;
const CHECKER_FRAG = `#version 300 es
precision highp float;
in vec2 uv;
uniform vec2 sizePx; // document rect size in device pixels
uniform float cell;
out vec4 color;
void main() {
  vec2 p = floor(uv * sizePx / cell);
  float c = mod(p.x + p.y, 2.0) < 1.0 ? 0.80 : 0.95;
  color = vec4(c, c, c, 1.0);
}`;

function compile(gl: WebGL2RenderingContext, vert: string, frag: string): WebGLProgram {
  const make = (type: number, src: string) => { const s = gl.createShader(type)!; gl.shaderSource(s, src); gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s) ?? "shader"); return s; };
  const p = gl.createProgram()!;
  gl.attachShader(p, make(gl.VERTEX_SHADER, vert)); gl.attachShader(p, make(gl.FRAGMENT_SHADER, frag)); gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p) ?? "link");
  return p;
}

/** Column-major mat3 mapping unit square -> clip space for a rectangle placed by an affine map in view CSS pixels. */
function unitToClip(a: number, b: number, c: number, d: number, tx: number, ty: number, viewW: number, viewH: number): Float32Array {
  // view -> clip: x' = 2x/W - 1, y' = 1 - 2y/H
  const sx = 2 / viewW, sy = -2 / viewH;
  return new Float32Array([a * sx, b * sy, 0, c * sx, d * sy, 0, tx * sx - 1, ty * sy + 1, 1]);
}

export class GlRenderer implements Renderer {
  readonly kind = "gl" as const;
  private textures: LayerTextures;
  private layerProgram: WebGLProgram;
  private checkerProgram: WebGLProgram;
  private vao: WebGLVertexArrayObject;
  private vertexBuffer: WebGLBuffer;
  constructor(private readonly canvas: HTMLCanvasElement, private readonly gl: WebGL2RenderingContext) {
    this.textures = new LayerTextures(gl);
    this.layerProgram = compile(gl, VERT, FRAG);
    this.checkerProgram = compile(gl, VERT, CHECKER_FRAG);
    this.vao = gl.createVertexArray()!;
    gl.bindVertexArray(this.vao);
    this.vertexBuffer = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.vertexBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, 0, 1, 0, 0, 1, 1, 1]), gl.STATIC_DRAW);
    for (const p of [this.layerProgram, this.checkerProgram]) {
      const loc = gl.getAttribLocation(p, "unit");
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
    }
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
  }

  sync(engine: EngineClient, state: DocumentState): void {
    const keep = new Set<string>();
    for (const layer of state.layers) {
      keep.add(layer.id);
      const pixels = layer.pixelsWidth > 0 ? engine.layerPixels(state.id, layer.id) : null;
      this.textures.sync(state.id, layer, pixels);
    }
    this.textures.retainOnly(state.id, keep);
  }

  /** Affine placing layer pixel (px, py) in view CSS pixels: doc = T(px), view = rect.origin + doc * ppp. */
  private layerToView(layer: LayerState, viewport: Viewport, state: DocumentState) {
    const [ox, oy] = layer.transform.origin; const [w, h] = layer.transform.size;
    const cx = ox + w / 2, cy = oy + h / 2;
    const rad = (layer.transform.rotation % 360) * Math.PI / 180;
    const cos = Math.cos(rad), sin = Math.sin(rad);
    const fx = layer.transform.flipX ? -1 : 1, fy = layer.transform.flipY ? -1 : 1;
    // unit (0..1) -> doc: center + R * ((u - 0.5) * size * flip)
    const a = cos * w * fx, b = sin * w * fx, c = -sin * h * fy, d = cos * h * fy;
    const tx = cx - (a + c) / 2, ty = cy - (b + d) / 2;
    const ppp = viewport.pointsPerPixel; const rect = viewport.documentRect({ width: state.width, height: state.height });
    return { a: a * ppp, b: b * ppp, c: c * ppp, d: d * ppp, tx: rect.x + tx * ppp, ty: rect.y + ty * ppp };
  }

  render(state: DocumentState, viewport: Viewport, dpr: number, options: RenderOptions): void {
    const gl = this.gl;
    const W = Math.max(1, Math.round(viewport.viewSize.width * dpr)), H = Math.max(1, Math.round(viewport.viewSize.height * dpr));
    if (this.canvas.width !== W || this.canvas.height !== H) { this.canvas.width = W; this.canvas.height = H; }
    gl.viewport(0, 0, W, H);
    gl.clearColor(0.16, 0.16, 0.16, 1); gl.clear(gl.COLOR_BUFFER_BIT);
    gl.bindVertexArray(this.vao);
    const rect = viewport.documentRect({ width: state.width, height: state.height });
    const vw = viewport.viewSize.width, vh = viewport.viewSize.height;
    // Round the left/right and top/bottom edges independently, then derive width/height from
    // their difference, so the scissor rect and readDocumentPixels agree on exactly the same
    // device pixels even when the document rect isn't pixel-aligned in view space.
    const scissorX0 = Math.round(rect.x * dpr), scissorX1 = Math.round((rect.x + rect.width) * dpr);
    const scissorYTop0 = Math.round(rect.y * dpr), scissorYTop1 = Math.round((rect.y + rect.height) * dpr);
    const scissorX = scissorX0, scissorW = scissorX1 - scissorX0;
    const scissorY = H - scissorYTop1, scissorH = scissorYTop1 - scissorYTop0;
    // Document background: checkerboard or transparent black.
    gl.useProgram(this.checkerProgram);
    gl.uniformMatrix3fv(gl.getUniformLocation(this.checkerProgram, "unitToClip"), false, unitToClip(rect.width, 0, 0, rect.height, rect.x, rect.y, vw, vh));
    gl.uniform2f(gl.getUniformLocation(this.checkerProgram, "sizePx"), rect.width * dpr, rect.height * dpr);
    gl.uniform1f(gl.getUniformLocation(this.checkerProgram, "cell"), options.checkerboard ? 8 * dpr : 1e9);
    if (options.checkerboard) gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    else { gl.enable(gl.SCISSOR_TEST); gl.scissor(scissorX, scissorY, scissorW, scissorH); gl.clearColor(0, 0, 0, 0); gl.clear(gl.COLOR_BUFFER_BIT); gl.disable(gl.SCISSOR_TEST); }
    // Clip layers to the document.
    gl.enable(gl.SCISSOR_TEST);
    gl.scissor(scissorX, scissorY, scissorW, scissorH);
    gl.useProgram(this.layerProgram);
    const uMat = gl.getUniformLocation(this.layerProgram, "unitToClip");
    const uOpacity = gl.getUniformLocation(this.layerProgram, "opacity");
    gl.uniform1i(gl.getUniformLocation(this.layerProgram, "tex"), 0);
    for (const layer of renderOrder(state)) {
      const t = this.textures.get(state.id, layer.id);
      if (!t) continue;
      const m = this.layerToView(layer, viewport, state);
      gl.uniform1f(uOpacity, layer.opacity);
      for (const chunk of t.chunks) {
        // Sub-rectangle of the unit square this chunk covers.
        const u0 = chunk.x / t.width, v0 = chunk.y / t.height, uw = chunk.width / t.width, vh2 = chunk.height / t.height;
        const a = m.a * uw, b = m.b * uw, c = m.c * vh2, d = m.d * vh2;
        const tx = m.tx + m.a * u0 + m.c * v0, ty = m.ty + m.b * u0 + m.d * v0;
        gl.uniformMatrix3fv(uMat, false, unitToClip(a, b, c, d, tx, ty, vw, vh));
        gl.bindTexture(gl.TEXTURE_2D, chunk.texture);
        gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
      }
    }
    gl.disable(gl.SCISSOR_TEST);
  }

  readPixels(): Uint8Array {
    const gl = this.gl; const W = this.canvas.width, H = this.canvas.height;
    const out = new Uint8Array(W * H * 4);
    gl.readPixels(0, 0, W, H, gl.RGBA, gl.UNSIGNED_BYTE, out);
    // Flip bottom-up to top-down.
    const row = W * 4; const flipped = new Uint8Array(W * H * 4);
    for (let y = 0; y < H; y++) flipped.set(out.subarray(y * row, (y + 1) * row), (H - 1 - y) * row);
    return flipped;
  }
  dispose(): void {
    this.textures.dispose();
    const gl = this.gl;
    gl.deleteProgram(this.layerProgram);
    gl.deleteProgram(this.checkerProgram);
    gl.deleteVertexArray(this.vao);
    gl.deleteBuffer(this.vertexBuffer);
  }
}
